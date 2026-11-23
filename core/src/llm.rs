use std::collections::HashSet;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde::Deserialize;
use serde_json::json;
use tokio::fs;

use crate::assets;
use crate::cli::{self, Kernel};
use crate::error::{ApiError, ApiResult};
use crate::library;
use crate::media_gen;
use crate::models::*;
use crate::providers::{self, Protocol};
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

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: serde_json::Value,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct ResponsesUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

#[derive(Debug, Deserialize)]
struct ResponsesResponse {
    #[serde(default)]
    output: serde_json::Value,
    #[serde(default)]
    output_text: Option<String>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<ResponsesUsage>,
}

fn collect_response_text(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(text) => {
            if !text.trim().is_empty() {
                out.push(text.clone());
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_response_text(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            if let Some(text) = map.get("text").and_then(|v| v.as_str()) {
                if !text.trim().is_empty() {
                    out.push(text.to_string());
                }
                return;
            }
            if let Some(content) = map.get("content") {
                collect_response_text(content, out);
            }
            if let Some(output) = map.get("output") {
                collect_response_text(output, out);
            }
        }
        _ => {}
    }
}

fn parse_openai_response(body: &str, fallback_model: &str) -> ApiResult<ChatOutcome> {
    let parsed: ChatResponse = serde_json::from_str(body).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            format!("模型响应不是有效的 OpenAI 兼容格式：{err}"),
        )
    })?;
    let content = parsed
        .choices
        .first()
        .map(|choice| content_to_text(&choice.message.content))
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
        model: parsed.model.unwrap_or_else(|| fallback_model.to_string()),
        usage: usage.unwrap_or_default(),
        latency_ms: 0,
    })
}

fn parse_anthropic_response(body: &str, fallback_model: &str) -> ApiResult<ChatOutcome> {
    let parsed: AnthropicResponse = serde_json::from_str(body).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            format!("Claude 响应格式异常：{err}"),
        )
    })?;
    let content = content_to_text(&parsed.content).trim().to_string();
    if content.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            "Claude 返回了空内容，请重试或更换模型",
        ));
    }
    let usage = parsed.usage.map(|u| Usage {
        prompt_tokens: u.input_tokens,
        completion_tokens: u.output_tokens,
        total_tokens: u.input_tokens + u.output_tokens,
    });
    Ok(ChatOutcome {
        content,
        model: parsed.model.unwrap_or_else(|| fallback_model.to_string()),
        usage: usage.unwrap_or_default(),
        latency_ms: 0,
    })
}

fn parse_responses_response(body: &str, fallback_model: &str) -> ApiResult<ChatOutcome> {
    let parsed: ResponsesResponse = serde_json::from_str(body).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            format!("Responses API 响应格式异常：{err}"),
        )
    })?;
    let content = if let Some(text) = parsed.output_text {
        text
    } else {
        let mut parts = Vec::new();
        collect_response_text(&parsed.output, &mut parts);
        parts.join("")
    };
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            "Responses API 返回了空内容，请重试或更换模型",
        ));
    }
    let usage = parsed.usage.map(|u| Usage {
        prompt_tokens: u.input_tokens,
        completion_tokens: u.output_tokens,
        total_tokens: u.input_tokens + u.output_tokens,
    });
    Ok(ChatOutcome {
        content,
        model: parsed.model.unwrap_or_else(|| fallback_model.to_string()),
        usage: usage.unwrap_or_default(),
        latency_ms: 0,
    })
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
    session: &str,
    system: &str,
    user: &str,
    max_tokens_override: Option<u32>,
) -> ApiResult<ChatOutcome> {
    let url = providers::chat_url(config)?;
    let provider = providers::for_config(config);
    if config.api_key.trim().is_empty()
        && provider.auth != "none"
        && !is_local_host(&config.base_url)
    {
        return Err(ApiError::new(
            axum::http::StatusCode::PRECONDITION_FAILED,
            "LLM_NOT_CONFIGURED",
            "尚未配置大模型 API Key。请到「设置」填写，或使用本地 Ollama / 兼容服务。",
        ));
    }
    if config.model.trim().is_empty() {
        return Err(ApiError::bad_request("模型名称不能为空"));
    }
    let protocol = providers::effective_protocol(config)?;
    let timeout = Duration::from_secs(config.timeout_secs.clamp(5, 600));
    let max_tokens = max_tokens_override.unwrap_or(config.max_tokens.max(16));
    let temperature = config.temperature.clamp(0.0, 2.0);
    let payload = match protocol {
        Protocol::OpenAi => json!({
            "model": config.model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": false
        }),
        Protocol::Anthropic => json!({
            "model": config.model,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
            "temperature": temperature,
            "max_tokens": max_tokens
        }),
        Protocol::Responses => json!({
            "model": config.model,
            "instructions": system,
            "input": user,
            "max_output_tokens": max_tokens
        }),
    };
    let headers = providers::build_headers(config, session)?;
    let started = Instant::now();
    let response = client
        .post(&url)
        .timeout(timeout)
        .headers(headers)
        .json(&payload)
        .send()
        .await
        .map_err(|err| {
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
            404 => "接口路径不存在，请检查 Base URL 与协议",
            429 => "触发限流或余额不足，请稍后再试",
            _ => "模型服务返回错误",
        };
        let mut message = format!("{hint}（HTTP {}）：{}", status.as_u16(), detail);
        if provider.id == "opencode-go"
            && (body_text.contains("MissingSessionID") || body_text.contains("x-opencode-session"))
        {
            message.push_str(
                "\n\nOpenCode Go 要求 x-opencode-session。MediaDesk 已自动附加；如果使用代理，请确认代理没有剥离该请求头。",
            );
        }
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "LLM_REQUEST_FAILED",
            message,
        )
        .with_details(json!({
            "status": status.as_u16(),
            "providerId": provider.id,
            "protocol": protocol.id()
        })));
    }

    let mut outcome = match protocol {
        Protocol::OpenAi => parse_openai_response(&body_text, &config.model)?,
        Protocol::Anthropic => parse_anthropic_response(&body_text, &config.model)?,
        Protocol::Responses => parse_responses_response(&body_text, &config.model)?,
    };
    outcome.latency_ms = started.elapsed().as_millis() as u64;
    Ok(outcome)
}

pub(crate) async fn save_snapshot(
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

async fn generate_api(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    req: GenerateRequest,
    config: &Config,
    compiled: PromptResult,
) -> ApiResult<GenerateResult> {
    let session = req
        .project_id
        .as_deref()
        .map(|project| format!("mediadesk-{project}-{}", req.kind))
        .unwrap_or_else(|| format!("mediadesk-quick-{}", req.kind));
    let outcome = chat_completion(
        client,
        &config.llm,
        &session,
        &compiled.system,
        &compiled.user,
        None,
    )
    .await?;

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
        kernel: "api".to_string(),
        asset: None,
    })
}

async fn generate_cli(
    workspace: &std::path::Path,
    req: GenerateRequest,
    config: &Config,
    compiled: PromptResult,
) -> ApiResult<GenerateResult> {
    let kernel = Kernel::from_id(&config.text_kernel).ok_or_else(|| {
        ApiError::bad_request(format!("不支持的文本内核：{}", config.text_kernel))
    })?;
    let cwd = match req.project_id.as_deref() {
        Some(id) => store::require_project(workspace, id).await?.dir_path,
        None => workspace.to_path_buf(),
    };
    let outcome = cli::run(&cwd, &config.cli, kernel, &compiled.prompt).await?;

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
        usage: Usage::default(),
        latency_ms: outcome.latency_ms,
        prompt: compiled.prompt,
        preset_id: compiled.preset.map(|p| p.id),
        skill_ids: compiled.skills.into_iter().map(|s| s.id).collect(),
        snapshot_path,
        kernel: kernel.id().to_string(),
        asset: None,
    })
}

pub async fn generate(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    req: GenerateRequest,
) -> ApiResult<GenerateResult> {
    let config = store::load_config(workspace).await?;
    if matches!(req.kind.as_str(), "illustration" | "video") {
        let profile = if req.kind == "illustration" {
            &config.media_gen.illustration
        } else {
            &config.media_gen.video
        };
        let compiled = build_media_prompt(workspace, &req, &profile.style).await?;
        let preset_id = compiled.preset.as_ref().map(|p| p.id.clone());
        let skill_ids = compiled
            .skills
            .iter()
            .map(|s| s.id.clone())
            .collect::<Vec<_>>();
        let outcome = media_gen::generate(workspace, client, &config, req, compiled).await?;
        return Ok(GenerateResult {
            content: format!("已生成并保存素材《{}》。", outcome.asset.meta.title),
            model: outcome.model,
            usage: Usage::default(),
            latency_ms: outcome.latency_ms,
            prompt: outcome.prompt,
            preset_id,
            skill_ids,
            snapshot_path: outcome.snapshot_path,
            kernel: "media".to_string(),
            asset: Some(outcome.asset),
        });
    }
    let compiled = build_prompt(workspace, &req).await?;
    if config.text_kernel != "api" {
        generate_cli(workspace, req, &config, compiled).await
    } else {
        generate_api(workspace, client, req, &config, compiled).await
    }
}

fn merge_test_config(stored: &LlmConfig, overrides: &TestLlmRequest) -> LlmConfig {
    let mut config = stored.clone();
    let base_before = config.base_url.clone();
    let provider_before = config.provider_id.clone();
    if let Some(v) = overrides
        .provider_id
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        config.provider_id = v.to_string();
    }
    if let Some(v) = overrides
        .base_url
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        config.base_url = v.to_string();
    }
    if let Some(v) = overrides.api_key.as_deref() {
        config.api_key = v.trim().to_string();
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
    if let Some(v) = overrides
        .protocol
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        config.protocol = v.to_string();
    }
    if let Some(headers) = &overrides.custom_headers {
        config.custom_headers = headers.clone();
    }
    if overrides.api_key.is_none()
        && (config.provider_id != provider_before
            || config.base_url.trim_end_matches('/') != base_before.trim_end_matches('/'))
    {
        config.api_key.clear();
    }
    config
}

fn model_list_from_value(value: &serde_json::Value) -> Vec<ModelInfo> {
    let array = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .or_else(|| value.get("models").and_then(serde_json::Value::as_array));
    let Some(array) = array else {
        return Vec::new();
    };
    let mut seen = HashSet::new();
    let mut models = Vec::new();
    for item in array {
        let id = item
            .get("id")
            .and_then(serde_json::Value::as_str)
            .or_else(|| item.get("model").and_then(serde_json::Value::as_str))
            .or_else(|| item.as_str())
            .map(str::trim)
            .filter(|id| !id.is_empty());
        let Some(id) = id else {
            continue;
        };
        if !seen.insert(id.to_string()) {
            continue;
        }
        let name = item
            .get("name")
            .and_then(serde_json::Value::as_str)
            .or_else(|| item.get("display_name").and_then(serde_json::Value::as_str))
            .or_else(|| item.get("displayName").and_then(serde_json::Value::as_str))
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string);
        models.push(ModelInfo {
            id: id.to_string(),
            name,
        });
        if models.len() >= 600 {
            break;
        }
    }
    models
}

pub async fn list_models(
    client: &reqwest::Client,
    stored: &LlmConfig,
    request: ModelListRequest,
) -> ApiResult<ModelListResponse> {
    let provider_id = request.provider_id.trim();
    let spec = providers::get(provider_id)
        .ok_or_else(|| ApiError::bad_request(format!("未知的服务商：{provider_id}")))?;
    let mut config = stored.clone();
    let provider_changed = config.provider_id != spec.id;
    config.provider_id = spec.id.clone();
    config.base_url = if request.base_url.trim().is_empty() {
        spec.base_url.clone()
    } else {
        request.base_url.trim().trim_end_matches('/').to_string()
    };
    let base_changed = config.base_url != stored.base_url.trim_end_matches('/');
    if let Some(key) = request.api_key {
        config.api_key = key.trim().to_string();
    } else if provider_changed || base_changed {
        // Never send a stored key to a provider or host the user just switched to.
        config.api_key.clear();
    }
    if let Some(protocol) = request
        .protocol
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        config.protocol = protocol.to_string();
    }
    config.custom_headers = request.custom_headers;
    if config.base_url.trim().is_empty() {
        return Err(ApiError::bad_request("请先填写 Base URL"));
    }
    let url = providers::models_url(&config)?;
    let headers = providers::build_headers(&config, "mediadesk-model-list")?;
    let response = client
        .get(&url)
        .timeout(Duration::from_secs(30))
        .headers(headers)
        .send()
        .await
        .map_err(|err| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MODEL_LIST_FAILED",
                if err.is_timeout() {
                    "获取模型列表超时，请检查网络或 Base URL".to_string()
                } else {
                    format!("无法连接模型服务：{err}")
                },
            )
        })?;
    let status = response.status();
    let body = response.text().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_LIST_FAILED",
            format!("读取模型列表失败：{err}"),
        )
    })?;
    if !status.is_success() {
        let hint = match status.as_u16() {
            401 | 403 => "API Key 无效或没有权限",
            404 => "该服务商没有可用的 /models 列表，或路径不正确",
            429 => "请求过于频繁，请稍后重试",
            _ => "模型列表接口返回错误",
        };
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_LIST_FAILED",
            format!(
                "{hint}（HTTP {}）：{}",
                status.as_u16(),
                sanitize_provider_error(&body, &config.api_key)
            ),
        )
        .with_details(json!({
            "status": status.as_u16(),
            "providerId": provider_id,
            "url": url
        })));
    }
    let value: serde_json::Value = serde_json::from_str(&body).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_LIST_FAILED",
            format!("模型列表不是有效 JSON：{err}"),
        )
    })?;
    let models = model_list_from_value(&value);
    if models.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MODEL_LIST_EMPTY",
            "接口没有返回可识别的模型列表，请手动填写模型 ID。",
        )
        .with_details(json!({ "providerId": provider_id, "url": url })));
    }
    Ok(ModelListResponse {
        provider_id: spec.id.clone(),
        models,
        fetched: true,
        note: Some(spec.note),
        docs_url: if spec.docs_url.is_empty() {
            None
        } else {
            Some(spec.docs_url)
        },
    })
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
        "mediadesk-connection-test",
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

pub struct TextCompletion {
    pub content: String,
    pub model: String,
    pub latency_ms: u64,
    pub usage: Usage,
}

/// Complete an arbitrary system/user prompt through the configured HTTP API.
/// Used by the project assistant when the selected kernel is `api`.
pub async fn complete_text(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    system: &str,
    user: &str,
    max_tokens: Option<u32>,
) -> ApiResult<TextCompletion> {
    let config = store::load_config(workspace).await?;
    let outcome = chat_completion(
        client,
        &config.llm,
        "mediadesk-assistant",
        system,
        user,
        max_tokens,
    )
    .await?;
    Ok(TextCompletion {
        content: outcome.content,
        model: outcome.model,
        latency_ms: outcome.latency_ms,
        usage: outcome.usage,
    })
}

pub async fn preview(workspace: &std::path::Path, req: GenerateRequest) -> ApiResult<PromptResult> {
    if matches!(req.kind.as_str(), "illustration" | "video") {
        let config = store::load_config(workspace).await?;
        let profile = if req.kind == "illustration" {
            &config.media_gen.illustration
        } else {
            &config.media_gen.video
        };
        return build_media_prompt(workspace, &req, &profile.style).await;
    }
    build_prompt(workspace, &req).await
}

/// Build the visual prompt used by image/video generation models.
/// Unlike `build_prompt`, this does not ask a text model for prose; it produces
/// a concise visual brief that can be sent directly to a media endpoint.
pub async fn build_media_prompt(
    workspace: &std::path::Path,
    req: &GenerateRequest,
    style: &str,
) -> ApiResult<PromptResult> {
    if !matches!(req.kind.as_str(), "illustration" | "video") {
        return Err(ApiError::bad_request("该类型不使用视觉提示词"));
    }
    let project = match req.project_id.as_deref() {
        Some(id) => Some(store::require_project(workspace, id).await?),
        None => None,
    };
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
    for id in idea_ids.iter().take(4) {
        if let Ok(idea) = store::get_idea(workspace, id).await {
            ideas.push(idea);
        }
    }

    let mut visual = String::new();
    if let Some(project) = &project {
        visual.push_str(&format!("项目：{}\n", project.meta.title));
        if !project.brief.trim().is_empty() {
            visual.push_str(&format!(
                "项目简介：{}\n",
                truncate_chars(project.brief.trim(), 1400)
            ));
        }
        let grouped = assets::scan_project(workspace, project, false).await?;
        let mut existing: Vec<String> = Vec::new();
        for kind in ASSET_KINDS {
            for asset in grouped.get(kind).into_iter().flatten().take(6) {
                existing.push(format!("{}《{}》", kind_label(kind), asset.meta.title));
            }
        }
        if !existing.is_empty() {
            visual.push_str(&format!("已有素材（避免重复）：{}\n", existing.join("、")));
        }
    } else {
        visual.push_str("项目：未立项的快速视觉创作。\n");
    }
    if !ideas.is_empty() {
        visual.push_str("\n灵感来源：\n");
        for idea in ideas {
            visual.push_str(&format!(
                "- {}：{}\n",
                idea.meta.title,
                truncate_chars(idea.content.trim(), 900)
            ));
        }
    }
    let instruction = if req.instruction.trim().is_empty() {
        match req.kind.as_str() {
            "illustration" => "为这个项目创作一张有辨识度的主视觉插画。",
            _ => "为这个项目创作一段有叙事感的短视频。",
        }
    } else {
        req.instruction.trim()
    };
    let style = style.trim();
    if !style.is_empty() {
        visual.push_str(&format!("\n统一风格：{style}\n"));
    }
    match req.kind.as_str() {
        "illustration" => visual.push_str(
            "\n画面要求：完整构图，明确主体、场景、光线、色彩与材质；适合编辑插画或主视觉；不出现文字、水印、logo、畸形人脸。\n",
        ),
        _ => visual.push_str(
            "\n视频要求：主体、动作、场景、镜头运动、光线与节奏要连贯；适合短镜头；不出现文字、水印、logo、畸形人脸。\n",
        ),
    }
    visual.push_str(&format!("\n创作要求：{instruction}\n"));

    let system = "你是视觉导演。根据项目上下文与创作者要求，写清楚可执行的画面描述；不要解释，不要输出代码块。";
    let user = format!(
        "素材类型：{}（{}）\n创作要求：{}\n风格：{}\n",
        kind_label(&req.kind),
        req.kind,
        instruction,
        if style.is_empty() {
            "跟随项目气质"
        } else {
            style
        }
    );
    Ok(PromptResult {
        prompt: visual.trim().to_string(),
        system: system.to_string(),
        user,
        kind: req.kind.clone(),
        preset: None,
        skills: Vec::new(),
    })
}
