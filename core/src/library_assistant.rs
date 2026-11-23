use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::fs;

use crate::cli::{self, Kernel};
use crate::error::{ApiError, ApiResult};
use crate::library;
use crate::llm;
use crate::models::*;
use crate::store::{self, atomic_write, now_iso};

const MAX_MESSAGE_CHARS: usize = 8000;
const MAX_HISTORY: usize = 40;
const MAX_ACTIONS: usize = 16;
const MAX_CONTENT_CHARS: usize = 50_000;
const MAX_ITEM_CHARS: usize = 4_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum LibraryAction {
    #[serde(rename = "createSkill")]
    CreateSkill {
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default, rename = "appliesTo")]
        applies_to: Vec<String>,
        #[serde(default)]
        content: String,
    },
    #[serde(rename = "updateSkill")]
    UpdateSkill {
        id: String,
        name: String,
        #[serde(default)]
        description: String,
        #[serde(default, rename = "appliesTo")]
        applies_to: Vec<String>,
        #[serde(default)]
        content: String,
    },
    #[serde(rename = "deleteSkill")]
    DeleteSkill { id: String },
    #[serde(rename = "createPreset")]
    CreatePreset {
        name: String,
        #[serde(default)]
        description: String,
        kind: String,
        #[serde(default, rename = "skillIds")]
        skill_ids: Vec<String>,
        #[serde(default)]
        content: String,
    },
    #[serde(rename = "updatePreset")]
    UpdatePreset {
        id: String,
        name: String,
        #[serde(default)]
        description: String,
        kind: String,
        #[serde(default, rename = "skillIds")]
        skill_ids: Vec<String>,
        #[serde(default)]
        content: String,
    },
    #[serde(rename = "deletePreset")]
    DeletePreset { id: String },
}

impl LibraryAction {
    fn label(&self) -> String {
        match self {
            Self::CreateSkill { name, .. } => format!("创建技能《{}》", name.trim()),
            Self::UpdateSkill { name, .. } => format!("更新技能《{}》", name.trim()),
            Self::DeleteSkill { id } => format!("删除技能 {id}"),
            Self::CreatePreset { name, .. } => format!("创建预设《{}》", name.trim()),
            Self::UpdatePreset { name, .. } => format!("更新预设《{}》", name.trim()),
            Self::DeletePreset { id } => format!("删除预设 {id}"),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryActionResult {
    pub action: String,
    pub ok: bool,
    pub message: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAssistantMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<LibraryAction>,
    #[serde(default)]
    pub applied: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAssistantHistory {
    pub messages: Vec<LibraryAssistantMessage>,
    pub kernel: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAssistantRequest {
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryAssistantResponse {
    pub reply: String,
    pub actions: Vec<LibraryAction>,
    pub results: Vec<LibraryActionResult>,
    pub warnings: Vec<String>,
    pub applied: bool,
    pub kernel: String,
    pub model: String,
    pub latency_ms: u64,
    pub messages: Vec<LibraryAssistantMessage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum LibraryAssistantStreamEvent {
    Status { message: String },
    Delta { text: String },
    Done { response: LibraryAssistantResponse },
    Error { code: String, message: String },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LibraryAssistantOutput {
    #[serde(default)]
    reply: String,
    #[serde(default)]
    actions: Vec<LibraryAction>,
}

fn history_path(workspace: &std::path::Path) -> std::path::PathBuf {
    workspace.join(".mediadesk/assistant/library-history.json")
}

pub async fn load_history(workspace: &std::path::Path) -> ApiResult<LibraryAssistantHistory> {
    let config = store::load_config(workspace).await?;
    let path = history_path(workspace);
    let messages = match fs::read_to_string(&path).await {
        Ok(raw) => serde_json::from_str::<Vec<LibraryAssistantMessage>>(&raw)
            .map_err(|err| ApiError::internal(format!("技能与预设助手历史损坏：{err}")))?,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(err) => return Err(err.into()),
    };
    Ok(LibraryAssistantHistory {
        messages,
        kernel: config.text_kernel,
    })
}

async fn save_history(
    workspace: &std::path::Path,
    messages: &[LibraryAssistantMessage],
) -> ApiResult<()> {
    let path = history_path(workspace);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let start = messages.len().saturating_sub(MAX_HISTORY);
    let data = serde_json::to_vec_pretty(&messages[start..])
        .map_err(|err| ApiError::internal(format!("序列化技能助手历史失败：{err}")))?;
    atomic_write(&path, &data).await
}

pub async fn clear_history(workspace: &std::path::Path) -> ApiResult<()> {
    match fs::remove_file(history_path(workspace)).await {
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
    history: &[LibraryAssistantMessage],
    message: &str,
) -> ApiResult<(String, String)> {
    let skills = library::list_skills(workspace).await?;
    let presets = library::list_presets(workspace).await?;
    let mut context = String::new();
    context.push_str(&format!(
        "# 当前技能库\n共 {} 个技能，{} 个预设。\n\n",
        skills.len(),
        presets.len()
    ));
    context.push_str("## 技能\n");
    for skill in &skills {
        context.push_str(&format!(
            "\n### 《{}》\nid: {}\nname: {}\ndescription: {}\nappliesTo: {}\ncontent:\n{}\n",
            skill.meta.name,
            skill.meta.id,
            skill.meta.name,
            skill.meta.description,
            skill.meta.applies_to.join(", "),
            truncate(skill.content.trim(), MAX_ITEM_CHARS)
        ));
    }
    if skills.is_empty() {
        context.push_str("当前没有技能。\n");
    }
    context.push_str("\n## 预设\n");
    for preset in &presets {
        context.push_str(&format!(
            "\n### 《{}》\nid: {}\nname: {}\nkind: {}\ndescription: {}\nskillIds: {}\ncontent:\n{}\n",
            preset.meta.name,
            preset.meta.id,
            preset.meta.name,
            preset.meta.kind,
            preset.meta.description,
            preset.meta.skill_ids.join(", "),
            truncate(preset.content.trim(), MAX_ITEM_CHARS)
        ));
    }
    if presets.is_empty() {
        context.push_str("当前没有预设。\n");
    }

    let system = "MEDIADESK_LIBRARY_PROTOCOL\n\
你是「媒材工坊」的技能与预设管理助手。你只管理技能与预设，不读取、不修改任何项目或项目素材。\n\n\
只输出一个 JSON 对象，不要 markdown 代码围栏，不要输出 JSON 以外的文字。格式：\n\
{\"reply\":\"给创作者看的回复\",\"actions\":[{\"type\":\"createSkill\",\"name\":\"技能名\",\"description\":\"一句话描述\",\"appliesTo\":[\"article\"],\"content\":\"完整技能正文\"},{\"type\":\"updateSkill\",\"id\":\"已有技能 id\",\"name\":\"新名称\",\"description\":\"...\",\"appliesTo\":[],\"content\":\"完整正文\"},{\"type\":\"deleteSkill\",\"id\":\"已有技能 id\"},{\"type\":\"createPreset\",\"name\":\"预设名\",\"description\":\"...\",\"kind\":\"article\",\"skillIds\":[],\"content\":\"完整预设正文\"},{\"type\":\"updatePreset\",\"id\":\"已有预设 id\",\"name\":\"...\",\"description\":\"...\",\"kind\":\"article\",\"skillIds\":[],\"content\":\"完整正文\"},{\"type\":\"deletePreset\",\"id\":\"已有预设 id\"}]}\n\n\
规则：\n\
1. 服务端会直接执行 actions；不要声称没有执行。你没有项目上下文，不要读取或编造项目素材。\n\
2. 只能使用当前技能库中真实存在的 id 做 update/delete，禁止编造 id。\n\
3. updateSkill / updatePreset 必须提供完整替换字段；content 必须是完整正文，不是 diff。\n\
4. 创建预设的 kind 只能是 article / script / illustration / card / video；skillIds 只能引用当前已有的技能 id。\n\
5. 只有创作者明确要求时才删除或批量修改；不要擅自清空技能库。\n\
6. 一次最多 16 个 action；没有动作时 actions 必须是空数组。\n\
7. 每条 content 的字数控制在合理范围；技能是方法论，预设是某类素材的创作规范。\n\
8. 直接、具体、克制，使用简体中文。\n\n";
    let mut user = String::new();
    if !history.is_empty() {
        user.push_str("# 最近对话\n");
        for item in history.iter().rev().take(10).rev() {
            let role = if item.role == "user" {
                "创作者"
            } else {
                "助手"
            };
            user.push_str(&format!(
                "{role}：{}\n",
                truncate(item.content.trim(), 1000)
            ));
        }
        user.push('\n');
    }
    user.push_str(&format!("# 新消息\n{message}\n"));
    Ok((format!("{system}{context}"), user))
}

fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return Some(trimmed.to_string());
    }
    if let Some(start) = trimmed.find("```json") {
        let body = &trimmed[start + 7..];
        if let Some(end) = body.find("```") {
            let candidate = body[..end].trim();
            if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                return Some(candidate.to_string());
            }
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    (end > start).then(|| trimmed[start..=end].to_string())
}

fn parse_output(raw: &str) -> (String, Vec<LibraryAction>) {
    let Some(json) = extract_json(raw) else {
        return (raw.trim().to_string(), Vec::new());
    };
    match serde_json::from_str::<LibraryAssistantOutput>(&json) {
        Ok(output) => {
            let reply = if output.reply.trim().is_empty() {
                "已处理。".to_string()
            } else {
                output.reply.trim().to_string()
            };
            (
                reply,
                output.actions.into_iter().take(MAX_ACTIONS).collect(),
            )
        }
        Err(_) => (raw.trim().to_string(), Vec::new()),
    }
}

fn content_too_long(content: &str) -> bool {
    content.chars().count() > MAX_CONTENT_CHARS
}

async fn apply_action(workspace: &std::path::Path, action: &LibraryAction) -> LibraryActionResult {
    let label = action.label();
    let result = match action {
        LibraryAction::CreateSkill {
            name,
            description,
            applies_to,
            content,
        } => {
            if content_too_long(content) {
                Err(ApiError::bad_request("技能正文过长"))
            } else {
                library::create_skill(
                    workspace,
                    SkillInput {
                        name: name.clone(),
                        description: description.clone(),
                        applies_to: applies_to.clone(),
                        content: content.clone(),
                    },
                )
                .await
                .map(|_| "技能已创建".to_string())
            }
        }
        LibraryAction::UpdateSkill {
            id,
            name,
            description,
            applies_to,
            content,
        } => {
            if content_too_long(content) {
                Err(ApiError::bad_request("技能正文过长"))
            } else {
                library::update_skill(
                    workspace,
                    id,
                    SkillInput {
                        name: name.clone(),
                        description: description.clone(),
                        applies_to: applies_to.clone(),
                        content: content.clone(),
                    },
                )
                .await
                .map(|_| "技能已更新".to_string())
            }
        }
        LibraryAction::DeleteSkill { id } => library::delete_skill(workspace, id)
            .await
            .map(|_| "技能已移入回收站".to_string()),
        LibraryAction::CreatePreset {
            name,
            description,
            kind,
            skill_ids,
            content,
        } => {
            if content_too_long(content) {
                Err(ApiError::bad_request("预设正文过长"))
            } else {
                library::create_preset(
                    workspace,
                    PresetInput {
                        name: name.clone(),
                        description: description.clone(),
                        kind: kind.clone(),
                        skill_ids: skill_ids.clone(),
                        content: content.clone(),
                    },
                )
                .await
                .map(|_| "预设已创建".to_string())
            }
        }
        LibraryAction::UpdatePreset {
            id,
            name,
            description,
            kind,
            skill_ids,
            content,
        } => {
            if content_too_long(content) {
                Err(ApiError::bad_request("预设正文过长"))
            } else {
                library::update_preset(
                    workspace,
                    id,
                    PresetInput {
                        name: name.clone(),
                        description: description.clone(),
                        kind: kind.clone(),
                        skill_ids: skill_ids.clone(),
                        content: content.clone(),
                    },
                )
                .await
                .map(|_| "预设已更新".to_string())
            }
        }
        LibraryAction::DeletePreset { id } => library::delete_preset(workspace, id)
            .await
            .map(|_| "预设已移入回收站".to_string()),
    };
    match result {
        Ok(message) => LibraryActionResult {
            action: label,
            ok: true,
            message,
        },
        Err(err) => LibraryActionResult {
            action: label,
            ok: false,
            message: err.message,
        },
    }
}

async fn finalize(
    workspace: &std::path::Path,
    message: String,
    history: Vec<LibraryAssistantMessage>,
    kernel: String,
    model: String,
    started: Instant,
    raw: &str,
) -> ApiResult<LibraryAssistantResponse> {
    let (reply, actions) = parse_output(raw);
    let mut results = Vec::new();
    for action in &actions {
        results.push(apply_action(workspace, action).await);
    }
    let warnings: Vec<String> = results
        .iter()
        .filter(|result| !result.ok)
        .map(|result| format!("{}：{}", result.action, result.message))
        .collect();
    let applied = results.iter().any(|result| result.ok);
    let mut messages = history;
    messages.push(LibraryAssistantMessage {
        id: store::new_id(),
        role: "user".to_string(),
        content: message,
        created_at: now_iso(),
        actions: Vec::new(),
        applied: false,
    });
    messages.push(LibraryAssistantMessage {
        id: store::new_id(),
        role: "assistant".to_string(),
        content: reply.clone(),
        created_at: now_iso(),
        actions: actions.clone(),
        applied,
    });
    save_history(workspace, &messages).await?;
    Ok(LibraryAssistantResponse {
        reply,
        actions,
        results,
        warnings,
        applied,
        kernel,
        model,
        latency_ms: started.elapsed().as_millis() as u64,
        messages,
    })
}

struct Prepared {
    config: Config,
    kernel_id: String,
    history: Vec<LibraryAssistantMessage>,
    system: String,
    user: String,
    message: String,
}

async fn prepare(workspace: &std::path::Path, req: LibraryAssistantRequest) -> ApiResult<Prepared> {
    let message = req.message.trim().to_string();
    if message.is_empty() {
        return Err(ApiError::bad_request("消息不能为空"));
    }
    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(ApiError::bad_request("单条消息过长"));
    }
    let config = store::load_config(workspace).await?;
    let history = load_history(workspace).await?.messages;
    let (system, user) = build_context(workspace, &history, &message).await?;
    Ok(Prepared {
        kernel_id: config.text_kernel.clone(),
        config,
        history,
        system,
        user,
        message,
    })
}

fn status_label(kernel: &str) -> String {
    if kernel == "api" {
        "API 大模型".to_string()
    } else {
        Kernel::from_id(kernel)
            .map(|item| item.label().to_string())
            .unwrap_or_else(|| kernel.to_string())
    }
}

fn emit(
    tx: &tokio::sync::mpsc::UnboundedSender<LibraryAssistantStreamEvent>,
    event: LibraryAssistantStreamEvent,
) {
    let _ = tx.send(event);
}

pub async fn chat_stream(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    req: LibraryAssistantRequest,
    tx: tokio::sync::mpsc::UnboundedSender<LibraryAssistantStreamEvent>,
) {
    if let Err(err) = chat_stream_inner(workspace, client, req, &tx).await {
        emit(
            &tx,
            LibraryAssistantStreamEvent::Error {
                code: err.code,
                message: err.message,
            },
        );
    }
}

async fn chat_stream_inner(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    req: LibraryAssistantRequest,
    tx: &tokio::sync::mpsc::UnboundedSender<LibraryAssistantStreamEvent>,
) -> ApiResult<()> {
    emit(
        tx,
        LibraryAssistantStreamEvent::Status {
            message: "正在读取技能与预设…".to_string(),
        },
    );
    let prepared = prepare(workspace, req).await?;
    emit(
        tx,
        LibraryAssistantStreamEvent::Status {
            message: format!("正在等待{}返回…", status_label(&prepared.kernel_id)),
        },
    );
    let started = Instant::now();
    let (raw, model) = if prepared.config.text_kernel == "api" {
        let delta_tx = tx.clone();
        let out = llm::stream_text(
            workspace,
            client,
            &prepared.system,
            &prepared.user,
            Some(prepared.config.llm.max_tokens),
            move |delta| {
                let _ = delta_tx.send(LibraryAssistantStreamEvent::Delta {
                    text: delta.to_string(),
                });
            },
        )
        .await?;
        (out.content, out.model)
    } else {
        let kernel = Kernel::from_id(&prepared.config.text_kernel).ok_or_else(|| {
            ApiError::bad_request(format!("不支持的文本内核：{}", prepared.config.text_kernel))
        })?;
        let prompt = format!("{}\n\n{}", prepared.system, prepared.user);
        let delta_tx = tx.clone();
        let out = cli::run_stream(
            workspace,
            &prepared.config.cli,
            kernel,
            &prompt,
            move |delta| {
                let _ = delta_tx.send(LibraryAssistantStreamEvent::Delta {
                    text: delta.to_string(),
                });
            },
        )
        .await?;
        (out.content, out.model)
    };
    emit(
        tx,
        LibraryAssistantStreamEvent::Status {
            message: "正在执行技能与预设变更…".to_string(),
        },
    );
    let response = finalize(
        workspace,
        prepared.message,
        prepared.history,
        prepared.kernel_id,
        model,
        started,
        &raw,
    )
    .await?;
    emit(tx, LibraryAssistantStreamEvent::Done { response });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_library_actions() {
        let raw = r#"{"reply":"好","actions":[{"type":"createSkill","name":"标题实验","appliesTo":["article"],"content":"一次给六个标题"}]}"#;
        let (reply, actions) = parse_output(raw);
        assert_eq!(reply, "好");
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], LibraryAction::CreateSkill { .. }));
    }

    #[test]
    fn parses_fenced_actions() {
        let raw = "说明\n```json\n{\"reply\":\"删除\",\"actions\":[{\"type\":\"deleteSkill\",\"id\":\"skill-x\"}]}\n```";
        let (_, actions) = parse_output(raw);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], LibraryAction::DeleteSkill { .. }));
    }
}
