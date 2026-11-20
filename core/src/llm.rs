use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::assets;
use crate::error::{ApiError, ApiResult};
use crate::library;
use crate::models::*;
use crate::store::{self, atomic_write, now_iso};

const KIND_DEFAULT_INSTRUCTION: &str = "请围绕项目主题创作一条高质量素材。";

fn kind_output_rules(kind: &str) -> &'static str {
    match kind {
        "article" => {
            "输出一篇可直接发布的 Markdown 文章。一级标题为文章标题，使用二级标题分节；不要用代码围栏包裹整篇内容。给出 3 个标题候选，正文后附「自检：」一行。"
        }
        "script" => {
            "输出一份可直接口播的 Markdown 脚本。包含 3 个标题候选、3 秒钩子、正文与镜头/停顿提示；单句不超过 30 字。"
        }
        "illustration" => {
            "输出 3 组插图提示词。每组包含：中文画面说明、English prompt、Negative prompt、推荐比例、标题安全区建议。不要生成图片，不要输出代码围栏。"
        }
        "card" => {
            "输出 5 组文卡图文案。每组包含：主张（8–18 字）、补充（12–30 字）、视觉建议、推荐比例（3:4 或 1:1）。不要写空泛鸡汤，不要伪造出处。"
        }
        "video" => {
            "输出一份视频分镜脚本。包含 3 个标题候选、一句话内容承诺、分镜表（镜号/时长/画面/口播/字幕/素材）、素材清单。不要虚构数据与采访。"
        }
        _ => "输出可直接使用的 Markdown 内容。",
    }
}

fn truncate_chars(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push_str("\n…（内容过长已截断）");
    }
    out
}

/// Compile project + preset + skills into one deterministic prompt.
pub async fn build_prompt(
    workspace: &std::path::Path,
    req: &GenerateRequest,
) -> ApiResult<PromptResult> {
    if !is_valid_kind(&req.kind) {
        return Err(ApiError::bad_request("素材类型不合法"));
    }

    let project = match req.project_id.as_deref() {
        Some(id) => Some(store::require_project(workspace, id).await?),
        None => None,
    };

    let preset = library::preset_for_kind(workspace, &req.kind, req.preset_id.as_deref()).await?;
    if let Some(p) = &preset {
        if p.meta.kind != req.kind {
            return Err(ApiError::bad_request("所选预设与素材类型不匹配"));
        }
    }

    let mut skill_ids: Vec<String> = if req.skill_ids.is_empty() {
        preset
            .as_ref()
            .map(|p| p.meta.skill_ids.clone())
            .unwrap_or_default()
    } else {
        req.skill_ids.clone()
    };
    let mut seen = HashSet::new();
    skill_ids.retain(|id| seen.insert(id.clone()));

    let all_skills = library::list_skills(workspace).await?;
    let mut used_skills: Vec<Skill> = Vec::new();
    for id in &skill_ids {
        if let Some(skill) = all_skills.iter().find(|s| &s.meta.id == id) {
            used_skills.push(skill.clone());
        }
    }

    let mut idea_ids: Vec<String> = project
        .as_ref()
        .map(|p| p.meta.idea_ids.clone())
        .unwrap_or_default();
    for id in &req.idea_ids {
        if !idea_ids.contains(id) {
            idea_ids.push(id.clone());
        }
    }
    let mut ideas: Vec<Idea> = Vec::new();
    for id in idea_ids.iter().take(6) {
        if let Ok(idea) = store::get_idea(workspace, id).await {
            ideas.push(idea);
        }
    }

    let mut context = String::new();
    if let Some(project) = &project {
        context.push_str(&format!("## 项目\n标题：{}\n", project.meta.title));
        if !project.brief.trim().is_empty() {
            context.push_str(&format!(
                "项目简介：{}\n",
                truncate_chars(project.brief.trim(), 1600)
            ));
        }
        let grouped = assets::scan_project(workspace, project, false).await?;
        context.push_str("\n## 已有素材（仅标题，避免重复）\n");
        for kind in ASSET_KINDS {
            let titles: Vec<String> = grouped
                .get(kind)
                .map(|list| {
                    list.iter()
                        .take(12)
                        .map(|a| format!("《{}》", a.meta.title))
                        .collect()
                })
                .unwrap_or_default();
            context.push_str(&format!(
                "- {}（{}）：{}\n",
                kind_label(kind),
                grouped.get(kind).map(Vec::len).unwrap_or(0),
                if titles.is_empty() {
                    "暂无".to_string()
                } else {
                    titles.join("、")
                }
            ));
        }
    } else {
        context.push_str("## 项目\n本次为未立项的快速创作，请仅基于下面提供的灵感与要求。\n");
    }

    if !ideas.is_empty() {
        context.push_str("\n## 灵感来源\n");
        for idea in ideas {
            context.push_str(&format!(
                "### {}\n标签：{}\n{}\n\n",
                idea.meta.title,
                if idea.meta.tags.is_empty() {
                    "无".to_string()
                } else {
                    idea.meta.tags.join("、")
                },
                truncate_chars(idea.content.trim(), 2200)
            ));
        }
    }

    let mut system = String::new();
    system.push_str(
        "你是「媒材工坊」的资深自媒体创作助手，服务于一位个人创作者。你的任务是把项目背景、灵感、创作预设与技能转化为可直接使用的素材。\n\n必须遵守：\n1. 只使用提供的上下文，不编造事实、数据、采访、名人名言或来源。\n2. 信息不足时，在正文末尾列出最多 3 个需要创作者补充的问题，而不是虚构。\n3. 使用简体中文（图像提示词可附英文），语气自然、具体、克制。\n4. 直接输出最终内容，不解释创作过程，不说「作为 AI」。\n\n",
    );
    system.push_str("# 项目与灵感上下文\n");
    system.push_str(&context);
    if let Some(preset) = &preset {
        system.push_str(&format!(
            "\n# 创作预设：{}\n{}\n",
            preset.meta.name,
            truncate_chars(preset.content.trim(), 12000)
        ));
    }
    if !used_skills.is_empty() {
        system.push_str("\n# 已加载技能\n");
        for skill in &used_skills {
            system.push_str(&format!(
                "\n## {}\n{}\n",
                skill.meta.name,
                truncate_chars(skill.content.trim(), 7000)
            ));
        }
    }
    system.push_str("\n# 本次输出规范\n");
    system.push_str(kind_output_rules(&req.kind));
    system.push('\n');

    let instruction = if req.instruction.trim().is_empty() {
        KIND_DEFAULT_INSTRUCTION
    } else {
        req.instruction.trim()
    };
    let user = format!(
        "# 本次任务\n素材类型：{}（{}）\n创作者要求：{}\n\n请直接输出可保存进素材库的内容。",
        kind_label(&req.kind),
        req.kind,
        instruction
    );
    let prompt = format!(
        "# 系统提示词\n\n{}\n\n---\n\n# 用户输入\n\n{}\n",
        system.trim(),
        user.trim()
    );

    Ok(PromptResult {
        prompt,
        system,
        user,
        kind: req.kind.clone(),
        preset: preset.as_ref().map(|p| UsedPreset {
            id: p.meta.id.clone(),
            name: p.meta.name.clone(),
        }),
        skills: used_skills
            .iter()
            .map(|s| UsedPreset {
                id: s.meta.id.clone(),
                name: s.meta.name.clone(),
            })
            .collect(),
    })
}

fn chat_url(base_url: &str) -> ApiResult<String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(ApiError::bad_request(
            "Base URL 必须以 http:// 或 https:// 开头",
        ));
    }
    if base.ends_with("/chat/completions") {
        Ok(base.to_string())
    } else {
        Ok(format!("{base}/chat/completions"))
    }
}

fn is_local_host(base_url: &str) -> bool {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|u| u.host_str().map(|h| h.to_string()))
        .map(|h| {
            h == "localhost"
                || h == "127.0.0.1"
                || h == "::1"
                || h == "0.0.0.0"
                || h.ends_with(".local")
        })
        .unwrap_or(false)
}

fn sanitize_provider_error(raw: &str, api_key: &str) -> String {
    let mut text = raw.trim().to_string();
    if !api_key.is_empty() {
        text = text.replace(api_key, "***");
    }
    if text.chars().count() > 600 {
        text = text.chars().take(600).collect::<String>() + "…";
    }
    if text.is_empty() {
        "服务商未返回错误详情".to_string()
    } else {
        text
    }
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    #[serde(default)]
    content: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

fn content_to_text(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(parts) => parts
            .iter()
            .filter_map(|p| {
                p.get("text")
                    .and_then(|t| t.as_str())
                    .map(str::to_string)
                    .or_else(|| p.as_str().map(str::to_string))
            })
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

struct ChatOutcome {
    content: String,
    model: String,
    usage: Usage,
    latency_ms: u64,
}

async fn chat_completion(
    client: &reqwest::Client,
    config: &LlmConfig,
    system: &str,
    user: &str,
    max_tokens_override: Option<u32>,
) -> ApiResult<ChatOutcome> {
    let url = chat_url(&config.base_url)?;
    if config.api_key.trim().is_empty() && !is_local_host(&config.base_url) {
        return Err(ApiError::new(
            axum::http::StatusCode::PRECONDITION_FAILED,
            "LLM_NOT_CONFIGURED",
            "尚未配置大模型 API Key。请到「设置」填写，或使用本地 Ollama / 兼容服务。",
        ));
    }
    if config.model.trim().is_empty() {
        return Err(ApiError::bad_request("模型名称不能为空"));
    }
    let timeout = Duration::from_secs(config.timeout_secs.clamp(5, 600));
    let payload = json!({
        "model": config.model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "temperature": config.temperature.clamp(0.0, 2.0),
        "max_tokens": max_tokens_override.unwrap_or(config.max_tokens.max(16)),
        "stream": false
    });
    let started = Instant::now();
    let mut request = client
        .post(&url)
        .timeout(timeout)
        .header("Content-Type", "application/json")
        .json(&payload);
    if !config.api_key.trim().is_empty() {
        request = request.bearer_auth(config.api_key.trim());
    }
    let response = request.send().await.map_err(|err| {
        let reason = if err.is_timeout() {
            "请求超时，请检查网络或调大超时时间".to_string()
        } else if err.is_connect() {
            "无法连接到模型服务，请检查 Base URL 与网络".to_string()
        } else {
            format!("请求失败：{err}")
        };
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            reason,
        )
    })?;

    let status = response.status();
    let body_text = response.text().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            format!("读取模型响应失败：{err}"),
        )
    })?;

    if !status.is_success() {
        let detail = sanitize_provider_error(&body_text, &config.api_key);
        let hint = match status.as_u16() {
            401 | 403 => "API Key 无效或没有权限",
            404 => "接口路径不存在，请检查 Base URL（通常以 /v1 结尾）",
            429 => "触发限流或余额不足，请稍后再试",
            _ => "模型服务返回错误",
        };
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            format!("{hint}（HTTP {}）：{}", status.as_u16(), detail),
        )
        .with_details(json!({ "status": status.as_u16() })));
    }

    let parsed: ChatResponse = serde_json::from_str(&body_text).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            format!("模型响应不是有效的 OpenAI 兼容格式：{err}"),
        )
    })?;
    let content = parsed
        .choices
        .first()
        .map(|c| content_to_text(&c.message.content))
        .unwrap_or_default()
        .trim()
        .to_string();
    if content.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            "模型返回了空内容，请重试或更换模型",
        ));
    }
    let usage = parsed.usage.map(|u| Usage {
        prompt_tokens: u.prompt_tokens,
        completion_tokens: u.completion_tokens,
        total_tokens: u.total_tokens,
    });
    Ok(ChatOutcome {
        content,
        model: parsed.model.unwrap_or_else(|| config.model.clone()),
        usage: usage.unwrap_or_default(),
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

async fn save_snapshot(
    workspace: &std::path::Path,
    project_id: &str,
    req: &GenerateRequest,
    prompt: &str,
    model: &str,
) -> Option<String> {
    let record = store::require_project(workspace, project_id).await.ok()?;
    let dir = record.dir_path.join("prompts");
    fs::create_dir_all(&dir).await.ok()?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let base = format!("{stamp}-{}", req.kind);
    let mut path = dir.join(format!("{base}.md"));
    let mut n = 1;
    while path.exists() {
        n += 1;
        path = dir.join(format!("{base}-{n}.md"));
    }
    let body = format!(
        "# Prompt 快照 · {} / {}\n\n- 项目：{}\n- 素材类型：{}\n- 模型：{}\n- 生成时间：{}\n\n## 完整提示词\n\n{}\n",
        record.meta.title,
        kind_label(&req.kind),
        record.meta.title,
        req.kind,
        model,
        now_iso(),
        prompt
    );
    if atomic_write(&path, body.as_bytes()).await.is_err() {
        return None;
    }
    Some(store::rel_path(workspace, &path))
}

pub async fn generate(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    req: GenerateRequest,
) -> ApiResult<GenerateResult> {
    let config = store::load_config(workspace).await?;
    let compiled = build_prompt(workspace, &req).await?;
    let outcome =
        chat_completion(client, &config.llm, &compiled.system, &compiled.user, None).await?;

    let snapshot_path = if let Some(project_id) = req.project_id.as_deref() {
        save_snapshot(
            workspace,
            project_id,
            &req,
            &compiled.prompt,
            &outcome.model,
        )
        .await
    } else {
        None
    };

    Ok(GenerateResult {
        content: outcome.content,
        model: outcome.model,
        usage: outcome.usage,
        latency_ms: outcome.latency_ms,
        prompt: compiled.prompt,
        preset_id: compiled.preset.map(|p| p.id),
        skill_ids: compiled.skills.into_iter().map(|s| s.id).collect(),
        snapshot_path,
    })
}

fn merge_test_config(stored: &LlmConfig, overrides: &TestLlmRequest) -> LlmConfig {
    let mut config = stored.clone();
    if let Some(v) = overrides
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        config.base_url = v.to_string();
    }
    if let Some(v) = overrides.api_key.as_deref() {
        if !v.trim().is_empty() {
            config.api_key = v.trim().to_string();
        }
    }
    if let Some(v) = overrides
        .model
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        config.model = v.to_string();
    }
    if let Some(v) = overrides.temperature {
        config.temperature = v;
    }
    if let Some(v) = overrides.max_tokens {
        config.max_tokens = v;
    }
    if let Some(v) = overrides.timeout_secs {
        config.timeout_secs = v;
    }
    config
}

pub async fn test_connection(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    overrides: TestLlmRequest,
) -> ApiResult<LlmTestResult> {
    let config = store::load_config(workspace).await?;
    let effective = merge_test_config(&config.llm, &overrides);
    let outcome = chat_completion(
        client,
        &effective,
        "你是一个连接测试助手。无论收到什么，只回复 pong。",
        "ping",
        Some(16),
    )
    .await?;
    Ok(LlmTestResult {
        ok: true,
        message: format!("连接成功：{}", truncate_chars(&outcome.content, 80)),
        latency_ms: outcome.latency_ms,
        model: outcome.model,
    })
}

pub async fn preview(workspace: &std::path::Path, req: GenerateRequest) -> ApiResult<PromptResult> {
    build_prompt(workspace, &req).await
}
