use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::error::{ApiError, ApiResult};
use crate::models::{CliConfig, Config, KernelInfo, KernelStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kernel {
    Claude,
    Opencode,
    Deepseek,
}

impl Kernel {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "claude" => Some(Self::Claude),
            "opencode" => Some(Self::Opencode),
            "deepseek" => Some(Self::Deepseek),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Opencode => "opencode",
            Self::Deepseek => "deepseek",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude Code",
            Self::Opencode => "OpenCode",
            Self::Deepseek => "DeepSeek CLI",
        }
    }

    fn binary_names(self) -> &'static [&'static str] {
        match self {
            Self::Claude => &["claude"],
            Self::Opencode => &["opencode"],
            Self::Deepseek => &["deepseek"],
        }
    }

    fn default_args(self) -> &'static [&'static str] {
        match self {
            // Claude Code print mode reads the prompt from stdin.
            Self::Claude => &["-p"],
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
    find_on_path(kernel.binary_names())
}

pub fn status(config: &Config) -> KernelStatus {
    let env_override = env_override().is_some();
    let kernels = [Kernel::Claude, Kernel::Opencode, Kernel::Deepseek]
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
                cmd.arg("/C").arg(&binary);
                cmd
            } else {
                Command::new(&binary)
            }
        }
        #[cfg(not(windows))]
        {
            Command::new(&binary)
        }
    };
    command
        .args(&args)
        .current_dir(cwd)
        .env("NO_COLOR", "1")
        .env("MEDIADESK_PROJECT_DIR", cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

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
        let detail = detail.chars().take(1200).collect::<String>();
        return Err(ApiError::new(
            StatusCode::BAD_GATEWAY,
            "CLI_FAILED",
            format!(
                "{} 执行失败（退出码 {:?}）：{}",
                kernel.label(),
                output.status.code(),
                if detail.is_empty() {
                    "没有错误输出"
                } else {
                    &detail
                }
            ),
        )
        .with_details(
            serde_json::json!({ "kernel": kernel.id(), "exitCode": output.status.code() }),
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
