use std::time::Instant;

use serde::Deserialize;
use tokio::fs;

use crate::assets;
use crate::cli::{self, Kernel};
use crate::error::{ApiError, ApiResult};
use crate::llm;
use crate::models::*;
use crate::store::{self, atomic_write, now_iso, require_project};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssistantOutput {
    #[serde(default)]
    reply: String,
    #[serde(default)]
    edits: Vec<AssistantEdit>,
    #[serde(default)]
    new_assets: Vec<AssistantCreate>,
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
    let data = serde_json::to_vec_pretty(messages)
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
        context.push_str(&format!("简介：{}\n", record.brief.trim()));
    }
    context.push_str("\n# 素材概览\n");
    for kind in ASSET_KINDS {
        context.push_str(&format!(
            "- {}：{} 条\n",
            kind_label(kind),
            grouped.get(kind).map(Vec::len).unwrap_or(0)
        ));
    }
    context.push_str("\n# 可编辑素材\n");
    let mut count = 0;
    for kind in ASSET_KINDS {
        for asset in grouped.get(kind).into_iter().flatten() {
            count += 1;
            context.push_str(&format!(
                "\n## {} 《{}》\nassetId: {}\ntitle: {}\nkind: {}\nstatus: {}\ncontent:\n{}\n",
                kind_label(kind),
                asset.meta.title,
                asset.meta.id,
                asset.meta.title,
                asset.meta.kind,
                asset.meta.status,
                asset.content.trim()
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
{\"reply\":\"给创作者看的回复\",\"edits\":[{\"assetId\":\"已有素材 id\",\"title\":\"可选的新标题\",\"content\":\"完整替换后的正文\",\"summary\":\"一句话改动摘要\"}],\"newAssets\":[{\"kind\":\"card\",\"title\":\"新素材标题\",\"content\":\"完整正文或卡片文案\",\"summary\":\"一句话说明\"}]}\n\n\
# 工具权限\n\
如果当前文本内核是本机 CLI，你拥有完整工具权限：可以直接读取项目外目录（包括 ~/.claude/skills）、访问网络、运行 Shell 和无头浏览器、生成 PNG/JPG 并写入项目。不要声称“权限被拦截”“沙盒之外”“没有浏览器环境”；缺少运行库时先安装或查找可用环境，不要只解释限制。\n\
生成二进制插图时：把文件写入项目的 illustration/ 目录，并创建同名 .md sidecar；frontmatter 至少包含 id、kind: illustration、title、status、tags、createdAt、updatedAt、file。不要把二进制塞进 JSON；在 reply 中报告文件路径。\n\
如果当前是 API 内核，没有工具权限，只能按 JSON 协议返回文本。\n\n\
规则：\n\
1. edits 只能修改上面列出的已有 assetId，禁止编造已有素材的 id。\n\
2. edits.content 必须是完整替换内容，不要给 diff 或片段。只改标题时可省略 content。\n\
3. 当用户明确要求新增素材，或某一类目录为空、确实需要创建时，使用 newAssets；新建素材不要填写 assetId。\n\
4. newAssets 的 kind 只能是 article / script / card；title 和 content 必须完整可保存。\n\
5. 新建文卡图时，content 写卡片主张和补充文案；成品 PNG 可直接用工具生成并写 sidecar，也可以交给「AI 文卡图」。\n\
6. 没有修改或新建时，edits 与 newAssets 都必须是空数组。\n\
7. 不编造事实、来源和数据；信息不足时在 reply 中提问。\n\
8. 直接、具体、克制，使用简体中文。\n\
9. 建议式只限制 edits 是否自动写入；当创作者明确要求执行、生成文件、截图或运行脚本时，应直接使用工具完成并如实报告，不要用“接口只能返回文本”拒绝。\n\n",
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
        for message in history.iter() {
            let role = if message.role == "user" {
                "创作者"
            } else {
                "助手"
            };
            user.push_str(&format!("{role}：{}\n", message.content.trim()));
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

fn parse_output(raw: &str) -> (String, Vec<AssistantEdit>, Vec<AssistantCreate>) {
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
            return (reply, output.edits, output.new_assets);
        }
    }
    (trimmed.to_string(), Vec::new(), Vec::new())
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
    let (reply, edits, new_assets) = parse_output(raw);
    let (applied_edits, mut warnings) = if prepared.mode == "apply" {
        apply_edits(workspace, project_id, &edits).await
    } else {
        (Vec::new(), Vec::new())
    };
    let (applied_creates, create_warnings) = if prepared.mode == "apply" {
        create_assets(workspace, project_id, &new_assets).await
    } else {
        (Vec::new(), Vec::new())
    };
    warnings.extend(create_warnings);
    let applied = !applied_edits.is_empty() || !applied_creates.is_empty();
    let stored_creates = if prepared.mode == "apply" {
        applied_creates
    } else {
        new_assets
    };

    let mut messages = prepared.history;
    messages.push(AssistantMessage {
        id: store::new_id(),
        role: "user".to_string(),
        content: prepared.message,
        created_at: now_iso(),
        edits: Vec::new(),
        new_assets: Vec::new(),
        applied: false,
    });
    messages.push(AssistantMessage {
        id: store::new_id(),
        role: "assistant".to_string(),
        content: reply.clone(),
        created_at: now_iso(),
        edits: edits.clone(),
        new_assets: stored_creates.clone(),
        applied,
    });
    save_history(&prepared.record.dir_path, &messages).await?;
    Ok(AssistantChatResponse {
        reply,
        edits,
        new_assets: stored_creates,
        applied,
        kernel,
        model,
        latency_ms,
        warnings,
        messages,
    })
}

async fn create_assets(
    workspace: &std::path::Path,
    project_id: &str,
    creates: &[AssistantCreate],
) -> (Vec<AssistantCreate>, Vec<String>) {
    let mut created = Vec::new();
    let mut warnings = Vec::new();
    for create in creates.iter() {
        let kind = create.kind.trim().to_ascii_lowercase();
        if !["article", "script", "card"].contains(&kind.as_str()) {
            warnings.push(format!(
                "「{}」的类型 {} 不能由助手新建；只支持文章、脚本、文卡图。",
                create.title.trim(),
                create.kind
            ));
            continue;
        }
        let title = create.title.trim();
        if title.is_empty() {
            warnings.push("新建素材缺少标题，已跳过。".to_string());
            continue;
        }
        if create.content.trim().is_empty() {
            warnings.push(format!("新建素材「{title}」缺少正文，已跳过。"));
            continue;
        }
        match assets::create_asset(
            workspace,
            project_id,
            CreateAsset {
                kind: kind.clone(),
                title: title.to_string(),
                content: create.content.clone(),
                status: None,
                tags: Vec::new(),
                prompt: None,
                source: Some("llm".to_string()),
            },
        )
        .await
        {
            Ok(asset) => {
                let mut next = create.clone();
                next.kind = kind;
                next.title = title.to_string();
                next.asset_id = Some(asset.meta.id);
                created.push(next);
            }
            Err(err) => warnings.push(format!("新建「{title}」失败：{}", err.message)),
        }
    }
    (created, warnings)
}

fn assistant_access_dirs(workspace: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![workspace.to_path_buf()];
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(std::path::PathBuf::from(home));
    }
    dirs.retain(|path| path.is_dir());
    dirs
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
        let access_dirs = assistant_access_dirs(workspace);
        let out = cli::run_with_dirs(
            &prepared.record.dir_path,
            &prepared.config.cli,
            kernel,
            &prompt,
            &access_dirs,
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
        let access_dirs = assistant_access_dirs(workspace);
        let delta_tx = tx.clone();
        let out = cli::run_stream_with_dirs(
            &prepared.record.dir_path,
            &prepared.config.cli,
            kernel,
            &prompt,
            &access_dirs,
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
    if messages[index].edits.is_empty() && messages[index].new_assets.is_empty() {
        return Err(ApiError::bad_request("这条消息没有可采纳的素材改动"));
    }
    let (applied_edits, mut warnings) =
        apply_edits(workspace, project_id, &messages[index].edits).await;
    let (created, create_warnings) =
        create_assets(workspace, project_id, &messages[index].new_assets).await;
    warnings.extend(create_warnings);
    let applied = !applied_edits.is_empty() || !created.is_empty();
    messages[index].applied = applied;
    if !created.is_empty() {
        messages[index].new_assets = created;
    }
    save_history(&record.dir_path, &messages).await?;
    Ok(AssistantApplyResponse {
        applied,
        warnings,
        messages,
    })
}
