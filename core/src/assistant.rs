use std::time::Instant;

use serde::Deserialize;
use tokio::fs;

use crate::assets;
use crate::cli::{self, Kernel};
use crate::error::{ApiError, ApiResult};
use crate::llm;
use crate::models::*;
use crate::store::{self, atomic_write, now_iso, require_project};

const MAX_MESSAGE_CHARS: usize = 8000;
const MAX_HISTORY: usize = 40;
const MAX_CONTEXT_ASSETS: usize = 24;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantOutput {
    #[serde(default)]
    reply: String,
    #[serde(default)]
    edits: Vec<AssistantEdit>,
}

fn history_path(project_dir: &std::path::Path) -> std::path::PathBuf {
    project_dir.join(".mediadesk/assistant/history.json")
}

pub async fn load_history(
    workspace: &std::path::Path,
    project_id: &str,
) -> ApiResult<AssistantHistory> {
    let record = require_project(workspace, project_id).await?;
    let config = store::load_config(workspace).await?;
    let path = history_path(&record.dir_path);
    let messages = match fs::read_to_string(&path).await {
        Ok(raw) => serde_json::from_str::<Vec<AssistantMessage>>(&raw)
            .map_err(|err| ApiError::internal(format!("项目助手历史记录损坏：{err}")))?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(err.into()),
    };
    Ok(AssistantHistory {
        messages,
        kernel: config.text_kernel,
    })
}

async fn save_history(
    project_dir: &std::path::Path,
    messages: &[AssistantMessage],
) -> ApiResult<()> {
    let path = history_path(project_dir);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let start = messages.len().saturating_sub(MAX_HISTORY);
    let data = serde_json::to_vec_pretty(&messages[start..])
        .map_err(|err| ApiError::internal(format!("序列化助手历史失败：{err}")))?;
    atomic_write(&path, &data).await
}

pub async fn clear_history(workspace: &std::path::Path, project_id: &str) -> ApiResult<()> {
    let record = require_project(workspace, project_id).await?;
    let path = history_path(&record.dir_path);
    match fs::remove_file(&path).await {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn truncate(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push_str("\n…（已截断）");
    }
    out
}

async fn build_context(
    workspace: &std::path::Path,
    project_id: &str,
    mode: &str,
    history: &[AssistantMessage],
) -> ApiResult<(String, String)> {
    let record = require_project(workspace, project_id).await?;
    let grouped = assets::scan_project(workspace, &record, false).await?;
    let mut context = String::new();
    context.push_str(&format!("# 项目\n标题：{}\n", record.meta.title));
    if !record.brief.trim().is_empty() {
        context.push_str(&format!("简介：{}\n", truncate(record.brief.trim(), 1200)));
    }
    context.push_str("\n# 可编辑素材\n");
    let mut count = 0;
    for kind in ASSET_KINDS {
        for asset in grouped.get(kind).into_iter().flatten() {
            if count >= MAX_CONTEXT_ASSETS {
                break;
            }
            count += 1;
            context.push_str(&format!(
                "\n## {} 《{}》\nassetId: {}\ntitle: {}\nkind: {}\nstatus: {}\ncontent:\n{}\n",
                kind_label(kind),
                asset.meta.title,
                asset.meta.id,
                asset.meta.title,
                asset.meta.kind,
                asset.meta.status,
                truncate(asset.content.trim(), 1600)
            ));
        }
    }
    if count == 0 {
        context.push_str("\n当前项目还没有素材。\n");
    }

    let mut system = String::new();
    system.push_str(
        "MEDIADESK_ASSISTANT_PROTOCOL\n\
你是「媒材工坊」的项目助手。你熟悉当前项目与全部素材，可以讨论内容，也可以提出对素材的完整修改。\n\n\
只输出一个 JSON 对象，不要 markdown 代码围栏，不要输出 JSON 以外的文字。格式：\n\
{\"reply\":\"给创作者看的回复\",\"edits\":[{\"assetId\":\"素材 id\",\"title\":\"可选的新标题\",\"content\":\"完整替换后的正文\",\"summary\":\"一句话改动摘要\"}]}\n\n\
规则：\n\
1. 只能修改上面列出的 assetId，禁止编造不存在的 id。\n\
2. content 必须是完整替换内容，不要给 diff 或片段。只改标题时可省略 content。\n\
3. 没有需要修改的素材时，edits 必须为空数组。\n\
4. 不编造事实、来源和数据；信息不足时在 reply 中提问。\n\
5. 直接、具体、克制，使用简体中文。\n\
6. 建议式下，如果创作者表示同意，提醒对方点击「采纳并写入」；不要声称已经写入。\n\n",
    );
    if mode == "apply" {
        system.push_str("当前模式：直接覆盖。服务端会把 edits 写入素材。\n");
    } else {
        system.push_str("当前模式：建议式。服务端只展示 edits，不写入；创作者可在界面点击「采纳并写入」应用这组建议。\n");
    }
    system.push('\n');
    system.push_str(&context);

    let mut user = String::new();
    if !history.is_empty() {
        user.push_str("# 最近对话\n");
        for message in history.iter().rev().take(10).rev() {
            let role = if message.role == "user" {
                "创作者"
            } else {
                "助手"
            };
            user.push_str(&format!(
                "{role}：{}\n",
                truncate(message.content.trim(), 1200)
            ));
        }
        user.push('\n');
    }
    user.push_str(&format!(
        "现在是创作者的新消息：\n{}\n",
        history_user_placeholder()
    ));
    Ok((system, user))
}

fn history_user_placeholder() -> &'static str {
    // Replaced by the actual user message before calling the model.
    "__CURRENT_USER_MESSAGE__"
}

fn parse_output(raw: &str) -> (String, Vec<AssistantEdit>) {
    let trimmed = raw.trim();
    let candidates = [
        trimmed.to_string(),
        trimmed
            .find("```json")
            .and_then(|start| {
                let body_start = start + "```json".len();
                trimmed[body_start..]
                    .find("```")
                    .map(|end| trimmed[body_start..body_start + end].to_string())
            })
            .unwrap_or_default(),
        trimmed
            .find('{')
            .and_then(|start| {
                trimmed
                    .rfind('}')
                    .map(|end| trimmed[start..=end].to_string())
            })
            .unwrap_or_default(),
    ];
    for candidate in candidates {
        if candidate.trim().is_empty() {
            continue;
        }
        if let Ok(output) = serde_json::from_str::<AssistantOutput>(candidate.trim()) {
            let reply = if output.reply.trim().is_empty() {
                "已处理。".to_string()
            } else {
                output.reply.trim().to_string()
            };
            return (reply, output.edits);
        }
    }
    (trimmed.to_string(), Vec::new())
}

async fn apply_edits(
    workspace: &std::path::Path,
    project_id: &str,
    edits: &[AssistantEdit],
) -> (Vec<AssistantEdit>, Vec<String>) {
    let mut applied_edits = Vec::new();
    let mut warnings = Vec::new();
    for edit in edits {
        if edit.content.is_none() && edit.title.is_none() {
            warnings.push(format!("素材 {} 的改动没有内容，已跳过。", edit.asset_id));
            continue;
        }
        match assets::update_asset(
            workspace,
            project_id,
            &edit.asset_id,
            UpdateAsset {
                title: edit.title.clone(),
                content: edit.content.clone(),
                status: None,
                tags: None,
                prompt: None,
            },
        )
        .await
        {
            Ok(_) => applied_edits.push(edit.clone()),
            Err(err) => warnings.push(format!("素材 {} 未更新：{}", edit.asset_id, err.message)),
        }
    }
    (applied_edits, warnings)
}

struct PreparedChat {
    record: store::ProjectRecord,
    config: Config,
    mode: String,
    message: String,
    history: Vec<AssistantMessage>,
    system: String,
    user: String,
}

async fn prepare_chat(
    workspace: &std::path::Path,
    project_id: &str,
    req: AssistantChatRequest,
) -> ApiResult<PreparedChat> {
    let message = req.message.trim();
    if message.is_empty() {
        return Err(ApiError::bad_request("消息不能为空"));
    }
    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(ApiError::bad_request("单条消息过长"));
    }
    let mode = req.mode.trim().to_ascii_lowercase();
    if !["suggest", "apply"].contains(&mode.as_str()) {
        return Err(ApiError::bad_request("助手模式必须是 suggest 或 apply"));
    }
    let record = require_project(workspace, project_id).await?;
    let config = store::load_config(workspace).await?;
    let history = load_history(workspace, project_id).await?.messages;
    let (system, user_template) = build_context(workspace, project_id, &mode, &history).await?;
    let user = user_template.replace(history_user_placeholder(), message);
    Ok(PreparedChat {
        record,
        config,
        mode,
        message: message.to_string(),
        history,
        system,
        user,
    })
}

async fn finalize_chat(
    workspace: &std::path::Path,
    project_id: &str,
    prepared: PreparedChat,
    started: Instant,
    raw: &str,
    model: String,
    kernel: String,
) -> ApiResult<AssistantChatResponse> {
    let latency_ms = started.elapsed().as_millis() as u64;
    let (reply, edits) = parse_output(raw);
    let (applied_edits, warnings) = if prepared.mode == "apply" {
        apply_edits(workspace, project_id, &edits).await
    } else {
        (Vec::new(), Vec::new())
    };
    let applied = !applied_edits.is_empty();

    let mut messages = prepared.history;
    messages.push(AssistantMessage {
        id: store::new_id(),
        role: "user".to_string(),
        content: prepared.message,
        created_at: now_iso(),
        edits: Vec::new(),
        applied: false,
    });
    messages.push(AssistantMessage {
        id: store::new_id(),
        role: "assistant".to_string(),
        content: reply.clone(),
        created_at: now_iso(),
        edits: edits.clone(),
        applied,
    });
    save_history(&prepared.record.dir_path, &messages).await?;
    Ok(AssistantChatResponse {
        reply,
        edits,
        applied,
        kernel,
        model,
        latency_ms,
        warnings,
        messages,
    })
}

pub async fn chat(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    project_id: &str,
    req: AssistantChatRequest,
) -> ApiResult<AssistantChatResponse> {
    let prepared = prepare_chat(workspace, project_id, req).await?;
    let started = Instant::now();
    let (raw, model, kernel) = if prepared.config.text_kernel == "api" {
        let out = llm::complete_text(
            workspace,
            client,
            &prepared.system,
            &prepared.user,
            Some(prepared.config.llm.max_tokens),
        )
        .await?;
        (out.content, out.model, "api".to_string())
    } else {
        let kernel = Kernel::from_id(&prepared.config.text_kernel).ok_or_else(|| {
            ApiError::bad_request(format!("不支持的文本内核：{}", prepared.config.text_kernel))
        })?;
        let prompt = format!("{}\n\n{}", prepared.system, prepared.user);
        let out = cli::run(
            &prepared.record.dir_path,
            &prepared.config.cli,
            kernel,
            &prompt,
        )
        .await?;
        (out.content, out.model, kernel.id().to_string())
    };
    finalize_chat(
        workspace, project_id, prepared, started, &raw, model, kernel,
    )
    .await
}

fn emit(
    tx: &tokio::sync::mpsc::UnboundedSender<AssistantStreamEvent>,
    event: AssistantStreamEvent,
) {
    let _ = tx.send(event);
}

pub async fn chat_stream(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    project_id: &str,
    req: AssistantChatRequest,
    tx: tokio::sync::mpsc::UnboundedSender<AssistantStreamEvent>,
) {
    if let Err(err) = chat_stream_inner(workspace, client, project_id, req, &tx).await {
        emit(
            &tx,
            AssistantStreamEvent::Error {
                code: err.code,
                message: err.message,
            },
        );
    }
}

async fn chat_stream_inner(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    project_id: &str,
    req: AssistantChatRequest,
    tx: &tokio::sync::mpsc::UnboundedSender<AssistantStreamEvent>,
) -> ApiResult<()> {
    emit(
        tx,
        AssistantStreamEvent::Status {
            message: "正在读取项目与素材…".to_string(),
        },
    );
    let prepared = prepare_chat(workspace, project_id, req).await?;
    let kernel_id = if prepared.config.text_kernel == "api" {
        "api"
    } else {
        prepared.config.text_kernel.as_str()
    };
    let kernel_label = if kernel_id == "api" {
        "API 大模型".to_string()
    } else {
        Kernel::from_id(kernel_id)
            .map(|kernel| kernel.label().to_string())
            .unwrap_or_else(|| kernel_id.to_string())
    };
    emit(
        tx,
        AssistantStreamEvent::Status {
            message: format!("正在等待{kernel_label}返回…"),
        },
    );
    let started = Instant::now();
    let (raw, model, kernel) = if prepared.config.text_kernel == "api" {
        let delta_tx = tx.clone();
        let out = llm::stream_text(
            workspace,
            client,
            &prepared.system,
            &prepared.user,
            Some(prepared.config.llm.max_tokens),
            move |delta| {
                let _ = delta_tx.send(AssistantStreamEvent::Delta {
                    text: delta.to_string(),
                });
            },
        )
        .await?;
        (out.content, out.model, "api".to_string())
    } else {
        let kernel = Kernel::from_id(&prepared.config.text_kernel).ok_or_else(|| {
            ApiError::bad_request(format!("不支持的文本内核：{}", prepared.config.text_kernel))
        })?;
        let prompt = format!("{}\n\n{}", prepared.system, prepared.user);
        let delta_tx = tx.clone();
        let out = cli::run_stream(
            &prepared.record.dir_path,
            &prepared.config.cli,
            kernel,
            &prompt,
            move |delta| {
                let _ = delta_tx.send(AssistantStreamEvent::Delta {
                    text: delta.to_string(),
                });
            },
        )
        .await?;
        (out.content, out.model, kernel.id().to_string())
    };
    emit(
        tx,
        AssistantStreamEvent::Status {
            message: "正在整理回复与改动…".to_string(),
        },
    );
    let response = finalize_chat(
        workspace, project_id, prepared, started, &raw, model, kernel,
    )
    .await?;
    emit(tx, AssistantStreamEvent::Done { response });
    Ok(())
}

pub async fn apply_saved_suggestion(
    workspace: &std::path::Path,
    project_id: &str,
    message_id: &str,
) -> ApiResult<AssistantApplyResponse> {
    let record = require_project(workspace, project_id).await?;
    let mut messages = load_history(workspace, project_id).await?.messages;
    let index = messages
        .iter()
        .position(|message| message.id == message_id)
        .ok_or_else(|| ApiError::not_found("找不到这条助手建议，可能已被清空。"))?;
    if messages[index].role != "assistant" {
        return Err(ApiError::bad_request("只能采纳助手消息中的建议"));
    }
    if messages[index].applied {
        return Ok(AssistantApplyResponse {
            applied: true,
            warnings: Vec::new(),
            messages,
        });
    }
    if messages[index].edits.is_empty() {
        return Err(ApiError::bad_request("这条消息没有可采纳的素材改动"));
    }
    let (applied_edits, warnings) =
        apply_edits(workspace, project_id, &messages[index].edits).await;
    let applied = !applied_edits.is_empty();
    messages[index].applied = applied;
    save_history(&record.dir_path, &messages).await?;
    Ok(AssistantApplyResponse {
        applied,
        warnings,
        messages,
    })
}
