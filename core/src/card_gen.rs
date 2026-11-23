use std::time::Instant;

use crate::assets;
use crate::cli::{self, Kernel};
use crate::error::{ApiError, ApiResult};
use crate::llm;
use crate::media_gen;
use crate::models::*;
use crate::store::{self, require_project};

const MAX_SOURCE_CHARS: usize = 16_000;
const MAX_BODY_ITEMS: usize = 4;

fn build_prompt(
    project_title: &str,
    project_brief: &str,
    source: &str,
    instruction: &str,
) -> (String, String) {
    let system = "MEDIADESK_CARD_PROTOCOL\n\
你是中文视觉编辑，负责把文章做成一张 1080×1440 的编辑式文卡图。只输出一个 JSON 对象，不要 markdown 围栏，不要 JSON 以外文字。\n\
格式：{\"kicker\":\"左上角短标记，如 前202 / 2026·11\",\"page\":\"右上角页码，如 04 / 04\",\"title\":\"主标题\",\"body\":[\"正文第一段\",\"正文第二段\"],\"footnote\":\"图注或一句注脚\",\"source\":\"底图/资料来源\",\"imagePrompt\":\"无文字的编辑插画或地图提示词\"}\n\
规则：\n\
1. 标题 6–22 个汉字，像杂志标题，不写空泛口号。\n\
2. body 2–4 段，每段 12–55 个汉字，可直接排版；必须来自原文事实，不编造人物、时间、数字或来源。\n\
3. kicker 是时间/章节/主题短标记；page 可留空；footnote 是图片说明或补充注脚；source 只写原文或用户提供的来源，没有就留空。\n\
4. imagePrompt 描述一张无文字、无字幕、无 logo 的横向编辑插画或地图，构图留白，适合放在卡片上半部分；不要要求模型生成文字。\n\
5. 信息不足时在 kicker 或 footnote 中保持克制，不要编造。\n";
    let mut user = String::new();
    user.push_str(&format!(
        "# 项目\n标题：{project_title}\n简介：{project_brief}\n\n"
    ));
    user.push_str("# 取材内容\n");
    user.push_str(source);
    user.push_str("\n\n# 额外要求\n");
    if instruction.trim().is_empty() {
        user.push_str("无额外要求，按原文生成一张克制的编辑式文卡图。\n");
    } else {
        user.push_str(instruction.trim());
        user.push('\n');
    }
    (system.to_string(), user)
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
    if end > start {
        Some(trimmed[start..=end].to_string())
    } else {
        None
    }
}

fn clean_text(value: &str, max: usize) -> String {
    let value = value.trim();
    let mut out: String = value.chars().take(max).collect();
    if value.chars().count() > max {
        out.push('…');
    }
    out
}

fn clean_plan(mut plan: CardPlan) -> ApiResult<CardPlan> {
    plan.kicker = clean_text(&plan.kicker, 24);
    plan.page = clean_text(&plan.page, 12);
    plan.title = clean_text(&plan.title, 40);
    plan.footnote = clean_text(&plan.footnote, 90);
    plan.source = clean_text(&plan.source, 90);
    plan.body = plan
        .body
        .into_iter()
        .map(|line| clean_text(&line, 120))
        .filter(|line| !line.is_empty())
        .take(MAX_BODY_ITEMS)
        .collect();
    plan.image_prompt = plan
        .image_prompt
        .map(|prompt| clean_text(&prompt, 700))
        .filter(|prompt| !prompt.is_empty());
    if plan.title.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型没有返回文卡图主标题，请重试。",
        ));
    }
    if plan.body.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型没有返回文卡图正文，请重试。",
        ));
    }
    Ok(plan)
}

async fn call_text_model(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    project_dir: &std::path::Path,
    system: &str,
    user: &str,
) -> ApiResult<(String, String)> {
    let config = store::load_config(workspace).await?;
    if config.text_kernel == "api" {
        let out = llm::complete_text(workspace, client, system, user, Some(config.llm.max_tokens))
            .await?;
        Ok((out.content, out.model))
    } else {
        let kernel = Kernel::from_id(&config.text_kernel).ok_or_else(|| {
            ApiError::bad_request(format!("不支持的文本内核：{}", config.text_kernel))
        })?;
        let prompt = format!("{system}\n\n{user}");
        let out = cli::run(project_dir, &config.cli, kernel, &prompt).await?;
        Ok((out.content, out.model))
    }
}

pub async fn generate_plan(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    project_id: &str,
    req: CardPlanRequest,
) -> ApiResult<CardPlanResponse> {
    let record = require_project(workspace, project_id).await?;
    let started = Instant::now();
    let (source_title, source_content) = if let Some(asset_id) = req
        .source_asset_id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    {
        let asset = assets::get_asset(workspace, project_id, asset_id).await?;
        if !["article", "script", "card"].contains(&asset.meta.kind.as_str()) {
            return Err(ApiError::bad_request(
                "只能从文章、脚本或文卡图文案中提取文卡内容",
            ));
        }
        (asset.meta.title, asset.content)
    } else {
        (record.meta.title.clone(), record.brief.clone())
    };
    if source_content.trim().is_empty() {
        return Err(ApiError::bad_request(
            "取材内容为空，请先写一段文章或补充项目简介。",
        ));
    }
    let source: String = source_content.chars().take(MAX_SOURCE_CHARS).collect();
    let (system, user) = build_prompt(
        &record.meta.title,
        record.brief.trim(),
        &format!("《{source_title}》\n{source}"),
        &req.instruction,
    );
    let (raw, model) = call_text_model(workspace, client, &record.dir_path, &system, &user).await?;
    let json = extract_json(&raw).ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型没有返回可解析的文卡图 JSON，请重试。",
        )
    })?;
    let plan: CardPlan = serde_json::from_str(&json).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            format!("文卡图方案格式不正确：{err}"),
        )
    })?;
    let plan = clean_plan(plan)?;

    let mut image_url = None;
    let mut image_asset_id = None;
    let mut image_warning = None;
    if req.with_image {
        if let Some(image_prompt) = plan.image_prompt.clone() {
            let config = store::load_config(workspace).await?;
            let image_prompt = format!(
                "编辑插画，无文字、无字幕、无 logo。{image_prompt}\n风格：纸质纹理、克制配色、大留白、适合作为 1080×1440 文卡图上部的横向画面。"
            );
            let generate_req = GenerateRequest {
                project_id: Some(project_id.to_string()),
                idea_ids: Vec::new(),
                kind: "illustration".to_string(),
                preset_id: None,
                skill_ids: Vec::new(),
                instruction: format!("{} · AI 文卡底图", plan.title),
            };
            let compiled = PromptResult {
                prompt: image_prompt,
                system: String::new(),
                user: String::new(),
                kind: "illustration".to_string(),
                preset: None,
                skills: Vec::new(),
            };
            match media_gen::generate(workspace, client, &config, generate_req, compiled).await {
                Ok(outcome) => {
                    image_url = outcome.asset.file_url.clone();
                    image_asset_id = Some(outcome.asset.meta.id.clone());
                    if image_url.is_none() {
                        image_warning = Some("配图已生成，但没有可预览的文件 URL。".to_string());
                    }
                }
                Err(err) => image_warning = Some(err.message),
            }
        } else {
            image_warning = Some("模型没有给出配图提示词，已生成纯文字版。".to_string());
        }
    }

    Ok(CardPlanResponse {
        plan,
        image_url,
        image_asset_id,
        image_warning,
        model,
        latency_ms: started.elapsed().as_millis() as u64,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fenced_json() {
        let raw = "说明\n```json\n{\"title\":\"标题\",\"body\":[\"正文\"]}\n```\n";
        let plan: CardPlan = serde_json::from_str(&extract_json(raw).unwrap()).unwrap();
        assert_eq!(plan.title, "标题");
    }

    #[test]
    fn rejects_empty_title_and_body() {
        assert!(clean_plan(CardPlan::default()).is_err());
        let plan = CardPlan {
            title: "标题".into(),
            body: vec!["正文".into()],
            ..CardPlan::default()
        };
        assert_eq!(clean_plan(plan).unwrap().title, "标题");
    }
}
