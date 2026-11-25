use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::context;
use crate::error::{ApiError, ApiResult};
use crate::models::DshResult;

fn find_on_path(name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let mut candidates = vec![name.to_string()];
    if cfg!(windows) {
        candidates.push(format!("{name}.cmd"));
        candidates.push(format!("{name}.exe"));
        candidates.push(format!("{name}.bat"));
    }
    for dir in std::env::split_paths(&path_var) {
        for candidate in &candidates {
            let path = dir.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

pub fn find_dsh_bin() -> Option<PathBuf> {
    if let Some(override_path) = std::env::var_os("MEDIADESK_DSH_BIN") {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Some(path);
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut candidates = vec![
            PathBuf::from(
                "/Applications/DeepSeek Harness.app/Contents/Resources/runtime/cli/bin/dsh",
            ),
            PathBuf::from(
                "/Applications/DeepSeek Harness.app/Contents/Resources/runtime/cli/bin/dsh.js",
            ),
        ];
        if let Some(home) = std::env::var_os("HOME") {
            candidates.push(
                PathBuf::from(home).join(
                    "Applications/DeepSeek Harness.app/Contents/Resources/runtime/cli/bin/dsh",
                ),
            );
        }
        for path in candidates {
            if path.is_file() {
                return Some(path);
            }
        }
    }

    find_on_path("dsh")
}

fn extract_url(line: &str) -> Option<String> {
    let start = line.find("http://").or_else(|| line.find("https://"))?;
    let rest = &line[start..];
    let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
    let url = rest[..end].trim();
    if url.len() <= "https://".len() {
        None
    } else {
        Some(url.to_string())
    }
}

fn open_url(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn start_error(
    message: impl Into<String>,
    project_path: &Path,
    binary: &Path,
    stderr: &str,
    files_written: &[String],
) -> ApiError {
    let hint = "请确认 DeepSeek Harness 已安装，且 `dsh web` 可以在终端启动。可通过 MEDIADESK_DSH_BIN 指定 dsh 路径。";
    let mut detail = message.into();
    if !stderr.trim().is_empty() {
        detail.push_str("：");
        detail.push_str(
            stderr
                .trim()
                .chars()
                .take(1200)
                .collect::<String>()
                .as_str(),
        );
    }
    ApiError::new(
        axum::http::StatusCode::BAD_GATEWAY,
        "DSH_START_FAILED",
        format!("{detail}。{hint}"),
    )
    .with_details(serde_json::json!({
        "projectPath": project_path.to_string_lossy(),
        "binary": binary.to_string_lossy(),
        "filesWritten": files_written,
        "hint": hint,
    }))
}

pub async fn launch(workspace: &Path, project_id: &str) -> ApiResult<DshResult> {
    let prepared = context::prepare(workspace, project_id).await?;
    let project_path = prepared.project_path.clone();
    let bin = find_dsh_bin().ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "DSH_NOT_FOUND",
            "未找到 DeepSeek Harness CLI。请先安装 DeepSeek Harness，或设置 MEDIADESK_DSH_BIN 指向 dsh 可执行文件。",
        )
        .with_details(serde_json::json!({
            "projectPath": project_path.to_string_lossy(),
            "manualCommand": "dsh web",
            "filesWritten": prepared.files,
        }))
    })?;

    let manual = format!("cd {} && dsh web", quote_path(&project_path));
    let mut child = Command::new(&bin)
        .args(["web", "--host", "127.0.0.1", "--port", "0", "--no-open"])
        .current_dir(&project_path)
        .env("NO_COLOR", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| {
            start_error(
                format!("启动 DeepSeek Harness 失败：{err}"),
                &project_path,
                &bin,
                "",
                &prepared.files,
            )
        })?;

    let stderr_buf = Arc::new(Mutex::new(String::new()));
    if let Some(stderr) = child.stderr.take() {
        let buffer = stderr_buf.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(mut text) = buffer.lock() {
                    text.push_str(&line);
                    text.push('\n');
                    if text.len() > 4000 {
                        let keep = text.len() - 4000;
                        text.drain(..keep);
                    }
                }
            }
        });
    }

    let stdout = child.stdout.take().ok_or_else(|| {
        start_error(
            "无法读取 DeepSeek Harness 启动输出",
            &project_path,
            &bin,
            "",
            &prepared.files,
        )
    })?;
    let mut lines = BufReader::new(stdout).lines();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut url = None;
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        match tokio::time::timeout(remaining, lines.next_line()).await {
            Ok(Ok(Some(line))) => {
                if let Some(found) = extract_url(&line) {
                    url = Some(found);
                    break;
                }
            }
            Ok(Ok(None)) => break,
            Ok(Err(_)) => break,
            Err(_) => break,
        }
    }

    let Some(url) = url else {
        let _ = child.start_kill();
        let _ = child.wait().await;
        let stderr = stderr_buf.lock().map(|s| s.clone()).unwrap_or_default();
        return Err(start_error(
            "DeepSeek Harness 启动超时或没有输出工作区地址",
            &project_path,
            &bin,
            &stderr,
            &prepared.files,
        ));
    };

    // Drain the rest of stdout so a long-running DSH process never blocks on a
    // full pipe. Keep the child process alive after Mediadesk returns.
    tokio::spawn(async move { while let Ok(Some(_)) = lines.next_line().await {} });
    tokio::spawn(async move {
        let _ = child.wait().await;
    });

    let opened = open_url(&url);
    Ok(DshResult {
        ok: true,
        url: url.clone(),
        workspace: project_path.to_string_lossy().to_string(),
        command: manual,
        files_written: prepared.files,
        message: if opened {
            "已生成 AI 上下文，并在 DeepSeek Harness 中打开当前项目目录".into()
        } else {
            format!("DeepSeek Harness 已启动，但浏览器未自动打开。请访问：{url}")
        },
    })
}

fn quote_path(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_url_from_the_dsh_ready_line() {
        assert_eq!(
            extract_url("dsh web: http://127.0.0.1:54252/?token=abc").as_deref(),
            Some("http://127.0.0.1:54252/?token=abc")
        );
        assert_eq!(
            extract_url("https://example.com/dsh").as_deref(),
            Some("https://example.com/dsh")
        );
        assert!(extract_url("starting...").is_none());
    }
}
