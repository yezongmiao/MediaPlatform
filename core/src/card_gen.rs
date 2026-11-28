use std::time::Instant;

use crate::assets;
use crate::cli::{self, Kernel};
use crate::error::{ApiError, ApiResult};
use crate::library;
use crate::llm;
use crate::models::*;
use crate::store::{self, require_project};

const MAX_SOURCE_CHARS: usize = 48_000;
const MAX_BODY_ITEMS: usize = 4;
const CHARS_PER_CARD: usize = 500;

fn desired_count(chars: usize) -> usize {
    chars.div_ceil(CHARS_PER_CARD).max(1)
}

fn build_prompt(
    project_title: &str,
    project_brief: &str,
    source: &str,
    instruction: &str,
    count: usize,
    controls: &str,
) -> (String, String) {
    let system = "MEDIADESK_CARD_PROTOCOL\n\
你是中文视觉编辑，负责把文章做成一组 1080×1440 的编辑式文卡图。只输出一个 JSON 对象，不要 markdown 围栏，不要 JSON 以外文字。\n\
输出格式：{\"cards\":[{\"kicker\":\"左上角短标记，如 前202 / 2026·11\",\"page\":\"右上角页码，如 04 / 04\",\"title\":\"主标题\",\"body\":[\"正文第一段\",\"正文第二段\"],\"footnote\":\"图注或一句注脚\",\"source\":\"底图/资料来源\"}]}\n\
规则：\n\
1. 根据文章长度和内容生成指定数量的卡片；每张卡片对应文章中的一个独立观点/段落，不要重复同一句。\n\
2. 标题 6–22 个汉字，像杂志标题，不写空泛口号。\n\
3. body 2–4 段，每段 12–55 个汉字，可直接排版；必须来自原文事实，不编造人物、时间、数字或来源。\n\
4. kicker 是时间/章节/主题短标记；page 从 01 开始编号，例如 01 / 03；footnote 是图片说明或补充注脚；source 只写原文或用户提供的来源，没有就留空。\n\
5. 卡片只使用文字排版，不生成、不调用任何图像模型；视觉层次由 kicker、标题、正文、注脚和来源完成。\n\
6. 卡片之间要有顺序和区分度，不要只是换标题。信息不足时保持克制，不要编造。\n";
    let system = if controls.trim().is_empty() {
        system.to_string()
    } else {
        format!("{system}\n# 已加载预设与技能\n{controls}\n")
    };
    let mut user = String::new();
    user.push_str(&format!(
        "# 项目\n标题：{project_title}\n简介：{project_brief}\n\n# 取材内容\n{source}\n\n"
    ));
    user.push_str(&format!(
        "# 生成数量\n请生成 {count} 张卡片。\n\n# 额外要求\n"
    ));
    if instruction.trim().is_empty() {
        user.push_str("无额外要求，按原文生成克制的编辑式文卡图。\n");
    } else {
        user.push_str(instruction.trim());
        user.push('\n');
    }
    (system, user)
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
    if plan.title.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型返回的卡片缺少主标题，请重试。",
        ));
    }
    if plan.body.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型返回的卡片缺少正文，请重试。",
        ));
    }
    Ok(plan)
}

fn parse_plans(raw: &str, expected: usize) -> ApiResult<(Vec<CardPlan>, Option<String>)> {
    let json = extract_json(raw).ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型没有返回可解析的文卡图 JSON，请重试。",
        )
    })?;
    let value: serde_json::Value = serde_json::from_str(&json).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            format!("文卡图方案格式不正确：{err}"),
        )
    })?;
    let raw_cards = if let Some(cards) = value.get("cards").and_then(|v| v.as_array()) {
        cards.clone()
    } else if let Some(cards) = value.as_array() {
        cards.clone()
    } else if value.get("title").is_some() {
        vec![value.clone()]
    } else {
        Vec::new()
    };
    if raw_cards.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "模型没有返回 cards 数组，请重试。",
        ));
    }

    let mut plans = Vec::new();
    let mut seen_titles = std::collections::HashSet::new();
    for raw_card in raw_cards {
        let plan: CardPlan = serde_json::from_value(raw_card).map_err(|err| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "CARD_PLAN_INVALID",
                format!("文卡图方案格式不正确：{err}"),
            )
        })?;
        let plan = clean_plan(plan)?;
        if !seen_titles.insert(plan.title.clone()) {
            continue;
        }
        plans.push(plan);
        if plans.len() >= expected {
            break;
        }
    }
    if plans.is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "CARD_PLAN_INVALID",
            "没有生成可用的文卡图方案，请重试。",
        ));
    }
    let warning = (plans.len() < expected).then(|| {
        format!(
            "文章长度建议 {expected} 张，模型返回了 {} 张可用方案。",
            plans.len()
        )
    });
    Ok((plans, warning))
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
    let source_chars = source_content.chars().count();
    let expected = desired_count(source_chars);
    let source: String = source_content.chars().take(MAX_SOURCE_CHARS).collect();
    let preset = library::preset_for_kind(workspace, "card", req.preset_id.as_deref()).await?;
    if let Some(preset) = &preset {
        if preset.meta.kind != "card" {
            return Err(ApiError::bad_request("所选预设与文卡图类型不匹配"));
        }
    }
    let mut skill_ids = if req.skill_ids.is_empty() {
        preset
            .as_ref()
            .map(|p| p.meta.skill_ids.clone())
            .unwrap_or_default()
    } else {
        req.skill_ids.clone()
    };
    let mut seen = std::collections::HashSet::new();
    skill_ids.retain(|id| seen.insert(id.clone()));
    let all_skills = library::list_skills(workspace).await?;
    let used_skills: Vec<Skill> = skill_ids
        .iter()
        .filter_map(|id| {
            all_skills
                .iter()
                .find(|skill| &skill.meta.id == id)
                .cloned()
        })
        .collect();
    let mut controls = String::new();
    if let Some(preset) = &preset {
        if !preset.content.trim().is_empty() {
            controls.push_str(&format!(
                "预设《{}》：\n{}\n",
                preset.meta.name,
                preset.content.trim().chars().take(2600).collect::<String>()
            ));
        }
    }
    for skill in &used_skills {
        controls.push_str(&format!(
            "技能《{}》：\n{}\n",
            skill.meta.name,
            skill.content.trim().chars().take(1600).collect::<String>()
        ));
    }
    let (system, user) = build_prompt(
        &record.meta.title,
        record.brief.trim(),
        &format!("《{source_title}》\n{source}"),
        &req.instruction,
        expected,
        &controls,
    );
    let (raw, model) = call_text_model(workspace, client, &record.dir_path, &system, &user).await?;
    let (plans, mut warning) = parse_plans(&raw, expected)?;

    let mut cards = Vec::new();
    for plan in plans {
        cards.push(GeneratedCard { plan });
    }

    let warning = warning.take();
    Ok(CardPlanResponse {
        cards,
        model,
        latency_ms: started.elapsed().as_millis() as u64,
        source_chars,
        target_count: expected,
        warning,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_fenced_json() {
        let raw = "说明\n```json\n{\"cards\":[{\"title\":\"标题\",\"body\":[\"正文\"]}]}\n```\n";
        let (plans, _) = parse_plans(raw, 1).unwrap();
        assert_eq!(plans[0].title, "标题");
    }

    #[test]
    fn chooses_count_from_article_length_without_a_default_cap() {
        assert_eq!(desired_count(1), 1);
        assert_eq!(desired_count(500), 1);
        assert_eq!(desired_count(501), 2);
        assert_eq!(desired_count(99_999), 200);
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
