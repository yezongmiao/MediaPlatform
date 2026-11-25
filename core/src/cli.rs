use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

use crate::error::{ApiError, ApiResult};
use crate::models::{CliConfig, Config, KernelInfo, KernelStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kernel {
    Claude,
    Codex,
    Opencode,
    Deepseek,
}

impl Kernel {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            "opencode" => Some(Self::Opencode),
            "deepseek" => Some(Self::Deepseek),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
            Self::Deepseek => "deepseek",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Codex => "Codex CLI",
            Self::Opencode => "OpenCode",
            Self::Deepseek => "DeepSeek CLI",
        }
    }

    fn binary_names(self) -> &'static [&'static str] {
        match self {
            Self::Claude => &["claude"],
            Self::Codex => &["codex"],
            Self::Opencode => &["opencode"],
            Self::Deepseek => &["deepseek"],
        }
    }

    fn default_args(self) -> &'static [&'static str] {
        match self {
            // Claude Code print mode reads the prompt from stdin.
            Self::Claude => &["-p"],
            // Codex exec runs non-interactively and accepts the prompt as an argument.
            Self::Codex => &["exec", "{prompt}"],
            // `opencode run` receives the prompt on stdin when no message arg is given.
            Self::Opencode => &["run"],
            Self::Deepseek => &[],
        }
    }

    pub fn install_hint(self) -> &'static str {
        match self {
            Self::Claude => {
                "安装 Claude Code：npm install -g @anthropic-ai/claude-code；或在设置中填写可执行文件路径。"
            }
            Self::Codex => {
                "安装 Codex CLI：npm install -g @openai/codex；或在设置中填写可执行文件路径。"
            }
            Self::Opencode => {
                "安装 OpenCode：npm install -g opencode-ai；或在设置中填写可执行文件路径。"
            }
            Self::Deepseek => {
                "安装 DeepSeek CLI，或在设置中填写其可执行文件路径。"
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct CliOutcome {
    pub content: String,
    pub model: String,
    pub latency_ms: u64,
    pub binary: PathBuf,
}

fn env_override() -> Option<PathBuf> {
    std::env::var_os("MEDIADESK_CLI_BIN")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn find_on_path(names: &[&str]) -> Option<PathBuf> {
    let mut candidates: Vec<String> = names.iter().map(|name| name.to_string()).collect();
    if cfg!(windows) {
        for name in names {
            candidates.push(format!("{name}.cmd"));
            candidates.push(format!("{name}.exe"));
            candidates.push(format!("{name}.bat"));
        }
    }
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        for name in &candidates {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn resolve(cli: &CliConfig, kernel: Kernel) -> Option<PathBuf> {
    if let Some(path) = env_override() {
        return path.is_file().then_some(path);
    }
    if let Some(configured) = cli
        .binary
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    {
        let path = PathBuf::from(configured);
        return path.is_file().then_some(path);
    }
    // Claude Code is often shadowed by editor/native launchers that require a
    // profile flag. The npm installation is the stable non-interactive entry.
    if kernel == Kernel::Claude {
        #[cfg(target_os = "macos")]
        for preferred in ["/usr/local/bin/claude", "/opt/homebrew/bin/claude"] {
            let path = PathBuf::from(preferred);
            if path.is_file() {
                return Some(path);
            }
        }
    }
    find_on_path(kernel.binary_names())
}

fn failure_hint(kernel: Kernel, detail: &str) -> Option<&'static str> {
    if kernel == Kernel::Claude && detail.contains("--profile") {
        Some(
            "当前 claude 可执行文件要求 --profile，可能命中了编辑器/启动器自带的包装程序。请在「设置 → 文本内核」把 CLI 路径指向 /usr/local/bin/claude；若必须用该启动器，请在附加参数中每行一个填写完整参数，例如 `-p`、`--profile`、`<名称>`（附加参数会替换默认参数）。",
        )
    } else {
        None
    }
}

fn cli_failure(
    kernel: Kernel,
    exit_code: Option<i32>,
    detail: &str,
    binary: &Path,
    args: &[String],
) -> ApiError {
    let detail = detail.trim().chars().take(1200).collect::<String>();
    let detail = if detail.is_empty() {
        "没有错误输出".to_string()
    } else {
        detail
    };
    let hint = failure_hint(kernel, &detail);
    let mut message = format!(
        "{} 执行失败（退出码 {:?}）：{}",
        kernel.label(),
        exit_code,
        detail
    );
    if let Some(hint) = hint {
        message.push('\n');
        message.push_str(hint);
    }
    ApiError::new(StatusCode::BAD_GATEWAY, "CLI_FAILED", message).with_details(serde_json::json!({
        "kernel": kernel.id(),
        "exitCode": exit_code,
        "binary": binary.to_string_lossy(),
        "args": args,
        "hint": hint,
    }))
}

pub fn status(config: &Config) -> KernelStatus {
    let env_override = env_override().is_some();
    let kernels = [
        Kernel::Claude,
        Kernel::Codex,
        Kernel::Opencode,
        Kernel::Deepseek,
    ]
    .into_iter()
    .map(|kernel| {
        let path = resolve(&config.cli, kernel);
        KernelInfo {
            id: kernel.id().to_string(),
            label: kernel.label().to_string(),
            found: path.is_some(),
            path: path.map(|p| p.to_string_lossy().to_string()),
            install_hint: kernel.install_hint().to_string(),
        }
    })
    .collect();
    KernelStatus {
        selected: config.text_kernel.clone(),
        env_override,
        kernels,
    }
}

fn missing_error(kernel: Kernel, configured: Option<&str>) -> ApiError {
    ApiError::new(
        StatusCode::CONFLICT,
        "CLI_NOT_FOUND",
        format!(
            "未找到 {} 可执行文件。{}",
            kernel.label(),
            kernel.install_hint()
        ),
    )
    .with_details(serde_json::json!({
        "kernel": kernel.id(),
        "configuredBinary": configured,
        "installHint": kernel.install_hint(),
    }))
}

fn apply_args(args: &[String], prompt: &str) -> (Vec<String>, bool) {
    let mut prompt_in_arg = false;
    let rendered = args
        .iter()
        .map(|arg| {
            if arg.contains("{prompt}") {
                prompt_in_arg = true;
                arg.replace("{prompt}", prompt)
            } else {
                arg.clone()
            }
        })
        .collect();
    (rendered, prompt_in_arg)
}

fn build_command(binary: &Path, args: &[String], cwd: &Path) -> Command {
    let mut command = {
        #[cfg(windows)]
        {
            let is_script = binary
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("cmd") || ext.eq_ignore_ascii_case("bat"))
                .unwrap_or(false);
            if is_script {
                let mut cmd = Command::new("cmd");
                cmd.arg("/C").arg(binary);
                cmd
            } else {
                Command::new(binary)
            }
        }
        #[cfg(not(windows))]
        {
            Command::new(binary)
        }
    };
    command
        .args(args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .env("MEDIADESK_PROJECT_DIR", cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    command
}

/// Run a CLI and forward its output as it arrives. Claude Code uses stream-json
/// by default here, so the assistant can render text before the turn ends.
pub async fn run_stream<F>(
    cwd: &Path,
    cli: &CliConfig,
    kernel: Kernel,
    prompt: &str,
    mut on_delta: F,
) -> ApiResult<CliOutcome>
where
    F: FnMut(&str) + Send,
{
    let binary = resolve(cli, kernel).ok_or_else(|| {
        missing_error(
            kernel,
            cli.binary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
        )
    })?;

    let mut stream_json = false;
    let args: Vec<String> = if cli.args.is_empty() {
        if kernel == Kernel::Claude {
            stream_json = true;
            vec![
                "-p".to_string(),
                "--output-format".to_string(),
                "stream-json".to_string(),
                "--verbose".to_string(),
                "--include-partial-messages".to_string(),
            ]
        } else {
            kernel
                .default_args()
                .iter()
                .map(|arg| arg.to_string())
                .collect()
        }
    } else {
        cli.args.clone()
    };
    let (args, prompt_in_arg) = apply_args(&args, prompt);

    let started = Instant::now();
    let mut child = build_command(&binary, &args, cwd).spawn().map_err(|err| {
        ApiError::new(
            StatusCode::CONFLICT,
            "CLI_START_FAILED",
            format!("启动 {} 失败：{err}", kernel.label()),
        )
        .with_details(
            serde_json::json!({ "kernel": kernel.id(), "binary": binary.to_string_lossy() }),
        )
    })?;

    let stdin_task = if prompt_in_arg {
        child.stdin.take();
        None
    } else if let Some(mut stdin) = child.stdin.take() {
        let prompt = prompt.to_string();
        Some(tokio::spawn(async move {
            let _ = stdin.write_all(prompt.as_bytes()).await;
        }))
    } else {
        None
    };

    let stderr_task = child.stderr.take().map(|stderr| {
        tokio::spawn(async move {
            let mut text = String::new();
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                text.push_str(&line);
                text.push('\n');
            }
            text
        })
    });

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ApiError::internal("无法读取 CLI 输出"))?;
    let timeout = Duration::from_secs(cli.timeout_secs.clamp(10, 1800));

    let read_and_wait = async {
        let mut lines = BufReader::new(stdout).lines();
        let mut raw = String::new();
        let mut content = String::new();
        let mut final_result: Option<String> = None;
        while let Some(line) = lines.next_line().await? {
            if stream_json {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) {
                    let kind = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if kind == "error"
                        || value
                            .get("is_error")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false)
                    {
                        let message = value
                            .get("error")
                            .and_then(|v| v.get("message"))
                            .and_then(|v| v.as_str())
                            .or_else(|| value.get("result").and_then(|v| v.as_str()))
                            .unwrap_or("Claude CLI 返回错误");
                        return Err(ApiError::new(
                            StatusCode::BAD_GATEWAY,
                            "CLI_FAILED",
                            message.to_string(),
                        ));
                    }
                    if kind == "result" {
                        if let Some(result) = value.get("result").and_then(|v| v.as_str()) {
                            final_result = Some(result.to_string());
                        }
                    }
                    let event = value.get("event").unwrap_or(&value);
                    let event_kind = event.get("type").and_then(|v| v.as_str()).unwrap_or("");
                    if event_kind == "content_block_delta" {
                        let delta = event.get("delta");
                        if delta.and_then(|d| d.get("type")).and_then(|v| v.as_str())
                            == Some("text_delta")
                        {
                            if let Some(text) =
                                delta.and_then(|d| d.get("text")).and_then(|v| v.as_str())
                            {
                                if !text.is_empty() {
                                    content.push_str(text);
                                    on_delta(text);
                                }
                                continue;
                            }
                        }
                    }
                    raw.push_str(&line);
                    raw.push('\n');
                } else {
                    raw.push_str(&line);
                    raw.push('\n');
                }
            } else {
                content.push_str(&line);
                content.push('\n');
                on_delta(&format!("{line}\n"));
            }
        }
        let status = child.wait().await?;
        Ok::<_, ApiError>((status, raw, content, final_result))
    };

    let outcome = match tokio::time::timeout(timeout, read_and_wait).await {
        Ok(Ok(value)) => value,
        Ok(Err(err)) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            if let Some(task) = stdin_task {
                task.abort();
            }
            if let Some(task) = stderr_task {
                task.abort();
            }
            return Err(err);
        }
        Err(_) => {
            let _ = child.start_kill();
            let _ = child.wait().await;
            if let Some(task) = stdin_task {
                task.abort();
            }
            if let Some(task) = stderr_task {
                task.abort();
            }
            return Err(ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "CLI_TIMEOUT",
                format!(
                    "{} 在 {} 秒内没有响应，已终止。",
                    kernel.label(),
                    timeout.as_secs()
                ),
            )
            .with_details(
                serde_json::json!({ "kernel": kernel.id(), "timeoutSecs": timeout.as_secs() }),
            ));
        }
    };

    if let Some(task) = stdin_task {
        let _ = task.await;
    }
    let stderr = match stderr_task {
        Some(task) => task.await.unwrap_or_default(),
        None => String::new(),
    };
    let (status, raw, mut content, final_result) = outcome;

    if !status.success() {
        let detail = if stderr.trim().is_empty() {
            raw.trim()
        } else {
            stderr.trim()
        };
        return Err(cli_failure(kernel, status.code(), detail, &binary, &args));
    }

    let was_streamed = !content.trim().is_empty();
    if content.trim().is_empty() {
        if let Some(result) = final_result {
            content = result;
        } else if !raw.trim().is_empty() {
            content = raw.clone();
        }
    }
    if !was_streamed && !content.trim().is_empty() {
        on_delta(content.trim());
    }
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "CLI_EMPTY_OUTPUT",
            format!(
                "{} 没有返回内容。请检查 CLI 是否需要额外参数。",
                kernel.label()
            ),
        )
        .with_details(serde_json::json!({ "kernel": kernel.id() })));
    }

    Ok(CliOutcome {
        content,
        model: format!("{}-cli", kernel.id()),
        latency_ms: started.elapsed().as_millis() as u64,
        binary,
    })
}

/// Run the selected CLI with the prompt on stdin (or substituted into `{prompt}`).
/// `cwd` must already be the locked project/workspace directory.
pub async fn run(
    cwd: &Path,
    cli: &CliConfig,
    kernel: Kernel,
    prompt: &str,
) -> ApiResult<CliOutcome> {
    let binary = resolve(cli, kernel).ok_or_else(|| {
        missing_error(
            kernel,
            cli.binary
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty()),
        )
    })?;

    let args: Vec<String> = if cli.args.is_empty() {
        kernel
            .default_args()
            .iter()
            .map(|arg| arg.to_string())
            .collect()
    } else {
        cli.args.clone()
    };
    let (args, prompt_in_arg) = apply_args(&args, prompt);

    let mut command = build_command(&binary, &args, cwd);

    let started = Instant::now();
    let mut child = command.spawn().map_err(|err| {
        ApiError::new(
            StatusCode::CONFLICT,
            "CLI_START_FAILED",
            format!("启动 {} 失败：{err}", kernel.label()),
        )
        .with_details(
            serde_json::json!({ "kernel": kernel.id(), "binary": binary.to_string_lossy() }),
        )
    })?;

    let prompt_owned = prompt.to_string();
    let wait = async move {
        if !prompt_in_arg {
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(prompt_owned.as_bytes())
                    .await
                    .map_err(|err| ApiError::internal(format!("写入 CLI stdin 失败：{err}")))?;
            }
        }
        child
            .wait_with_output()
            .await
            .map_err(|err| ApiError::internal(format!("读取 CLI 输出失败：{err}")))
    };

    let timeout = Duration::from_secs(cli.timeout_secs.clamp(10, 1800));
    let output = match tokio::time::timeout(timeout, wait).await {
        Ok(Ok(output)) => output,
        Ok(Err(err)) => return Err(err),
        Err(_) => {
            return Err(ApiError::new(
                StatusCode::GATEWAY_TIMEOUT,
                "CLI_TIMEOUT",
                format!(
                    "{} 在 {} 秒内没有响应，已终止。",
                    kernel.label(),
                    timeout.as_secs()
                ),
            )
            .with_details(
                serde_json::json!({ "kernel": kernel.id(), "timeoutSecs": timeout.as_secs() }),
            ))
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        return Err(cli_failure(
            kernel,
            output.status.code(),
            detail,
            &binary,
            &args,
        ));
    }

    let content = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if content.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "CLI_EMPTY_OUTPUT",
            format!(
                "{} 没有返回内容。请检查 CLI 是否需要额外参数。",
                kernel.label()
            ),
        )
        .with_details(serde_json::json!({ "kernel": kernel.id() })));
    }

    Ok(CliOutcome {
        content,
        model: format!("{}-cli", kernel.id()),
        latency_ms: started.elapsed().as_millis() as u64,
        binary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_required_error_gets_an_actionable_hint() {
        let hint = failure_hint(Kernel::Claude, "error: --profile <name> is required")
            .expect("profile hint");
        assert!(hint.contains("/usr/local/bin/claude"));
        assert!(hint.contains("--profile"));

        assert!(failure_hint(Kernel::Claude, "network unavailable").is_none());
        assert!(failure_hint(Kernel::Codex, "--profile <name> is required").is_none());
    }

    #[test]
    fn cli_failure_mentions_binary_and_keeps_a_short_exit_message() {
        let error = cli_failure(
            Kernel::Claude,
            Some(1),
            "error: --profile <name> is required",
            Path::new("/usr/local/bin/claude"),
            &["-p".to_string()],
        );
        assert!(error.message.contains("Claude Code 执行失败"));
        assert!(error.message.contains("/usr/local/bin/claude"));
        assert_eq!(error.code, "CLI_FAILED");
    }
}
