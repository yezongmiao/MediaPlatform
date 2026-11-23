use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use chrono::Utc;
use serde_json::{json, Value};
use tokio::fs;

use crate::assets;
use crate::error::{ApiError, ApiResult};
use crate::llm;
use crate::models::*;

const MAX_IMAGE_BYTES: usize = 30 * 1024 * 1024;
const MAX_VIDEO_BYTES: usize = 200 * 1024 * 1024;

pub struct MediaOutcome {
    pub asset: Asset,
    pub prompt: String,
    pub model: String,
    pub latency_ms: u64,
    pub snapshot_path: Option<String>,
}

fn profile_for<'a>(config: &'a MediaGenConfig, kind: &str) -> &'a MediaProfile {
    if kind == "illustration" {
        &config.illustration
    } else {
        &config.video
    }
}

fn is_local_host(url: &str) -> bool {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(|host| host.to_string()))
        .map(|host| {
            host == "localhost"
                || host == "127.0.0.1"
                || host == "::1"
                || host == "0.0.0.0"
                || host.ends_with(".local")
        })
        .unwrap_or(false)
}

fn ensure_configured(profile: &MediaProfile, kind: &str) -> ApiResult<()> {
    if profile.base_url.trim().is_empty() || profile.model.trim().is_empty() {
        return Err(ApiError::new(
            axum::http::StatusCode::PRECONDITION_FAILED,
            "MEDIA_NOT_CONFIGURED",
            format!(
                "尚未配置{}生成模型。请到「设置 → 媒体生成」填写 Base URL 与模型。",
                kind_label(kind)
            ),
        )
        .with_details(json!({ "kind": kind })));
    }
    if profile.api_key.trim().is_empty() && !is_local_host(&profile.base_url) {
        return Err(ApiError::new(
            axum::http::StatusCode::PRECONDITION_FAILED,
            "MEDIA_NOT_CONFIGURED",
            format!("{}生成模型尚未配置 API Key。", kind_label(kind)),
        )
        .with_details(json!({ "kind": kind })));
    }
    Ok(())
}

fn endpoint(base_url: &str, configured_path: &str, default_path: &str) -> ApiResult<String> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(ApiError::bad_request(
            "媒体生成 Base URL 必须以 http:// 或 https:// 开头",
        ));
    }
    let path = if configured_path.trim().is_empty() {
        default_path
    } else {
        configured_path.trim()
    };
    if path.starts_with("http://") || path.starts_with("https://") {
        return Ok(path.to_string());
    }
    Ok(format!("{base}/{}", path.trim_start_matches('/')))
}

fn sanitize(raw: &str, api_key: &str) -> String {
    let mut text = raw.trim().to_string();
    if !api_key.trim().is_empty() {
        text = text.replace(api_key.trim(), "***");
    }
    if text.chars().count() > 700 {
        text = text.chars().take(700).collect::<String>() + "…";
    }
    if text.is_empty() {
        "服务商未返回错误详情".to_string()
    } else {
        text
    }
}

fn media_failure(status: u16, body: &str, api_key: &str) -> ApiError {
    let hint = match status {
        401 | 403 => "API Key 无效或没有权限",
        404 => "媒体接口路径不存在，请检查 Base URL 与创建路径",
        429 => "媒体服务触发限流或余额不足",
        _ => "媒体生成服务返回错误",
    };
    ApiError::new(
        axum::http::StatusCode::BAD_GATEWAY,
        "MEDIA_GEN_FAILED",
        format!("{hint}（HTTP {status}）：{}", sanitize(body, api_key)),
    )
    .with_details(json!({ "status": status }))
}

async fn post_json(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    body: &Value,
    timeout_secs: u64,
) -> ApiResult<Value> {
    let mut request = client
        .post(url)
        .timeout(Duration::from_secs(timeout_secs.clamp(10, 1800)))
        .header("Content-Type", "application/json")
        .json(body);
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }
    let response = request.send().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("无法连接媒体生成服务：{err}"),
        )
    })?;
    let status = response.status();
    let text = response.text().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("读取媒体生成响应失败：{err}"),
        )
    })?;
    if !status.is_success() {
        return Err(media_failure(status.as_u16(), &text, api_key));
    }
    serde_json::from_str(&text).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("媒体服务返回的不是有效 JSON：{err}"),
        )
    })
}

async fn get_json(
    client: &reqwest::Client,
    url: &str,
    api_key: &str,
    timeout_secs: u64,
) -> ApiResult<Value> {
    let mut request = client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs.clamp(10, 1800)));
    if !api_key.trim().is_empty() {
        request = request.bearer_auth(api_key.trim());
    }
    let response = request.send().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("轮询媒体任务失败：{err}"),
        )
    })?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(media_failure(status.as_u16(), &text, api_key));
    }
    serde_json::from_str(&text).map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("媒体任务返回的不是有效 JSON：{err}"),
        )
    })
}

fn string_at<'a>(value: &'a Value, pointers: &[&str]) -> Option<&'a str> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
}

fn find_key_string(value: &Value, keys: &[&str], depth: usize) -> Option<String> {
    if depth > 8 {
        return None;
    }
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(Value::String(text)) = map.get(*key) {
                    if !text.trim().is_empty() {
                        return Some(text.clone());
                    }
                }
            }
            map.values()
                .find_map(|child| find_key_string(child, keys, depth + 1))
        }
        Value::Array(items) => items
            .iter()
            .find_map(|child| find_key_string(child, keys, depth + 1)),
        _ => None,
    }
}

fn find_id(value: &Value) -> Option<String> {
    find_key_string(value, &["task_id", "taskId", "id"], 0)
}

fn find_status(value: &Value) -> Option<String> {
    find_key_string(value, &["status", "state"], 0).map(|s| s.to_ascii_lowercase())
}

fn find_video_url(value: &Value) -> Option<String> {
    let preferred = find_key_string(
        value,
        &[
            "video_url",
            "videoUrl",
            "download_url",
            "downloadUrl",
            "file_url",
            "fileUrl",
        ],
        0,
    );
    preferred
        .or_else(|| find_key_string(value, &["url"], 0))
        .filter(|url| {
            !url.to_ascii_lowercase().contains("thumbnail")
                && !url.to_ascii_lowercase().contains("cover")
        })
}

fn find_image_source(value: &Value) -> Option<(Option<String>, Option<String>)> {
    let b64 = string_at(
        value,
        &[
            "/data/0/b64_json",
            "/data/0/b64Json",
            "/b64_json",
            "/b64Json",
            "/images/0/b64_json",
        ],
    )
    .map(str::to_string);
    let url = string_at(
        value,
        &[
            "/data/0/url",
            "/data/0/image_url",
            "/data/0/imageUrl",
            "/url",
            "/image_url",
            "/imageUrl",
            "/images/0/url",
        ],
    )
    .map(str::to_string);
    if b64.is_some() || url.is_some() {
        Some((url, b64))
    } else {
        None
    }
}

fn decode_data_uri(uri: &str) -> Option<(Vec<u8>, String)> {
    let rest = uri.strip_prefix("data:")?;
    let (meta, data) = rest.split_once(',')?;
    let mime = meta
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .to_string();
    let bytes = if meta.contains(";base64") {
        STANDARD.decode(data).ok()?
    } else {
        percent_encoding::percent_decode_str(data).collect()
    };
    Some((bytes, mime))
}

fn same_origin(a: &str, b: &str) -> bool {
    let (Ok(a), Ok(b)) = (reqwest::Url::parse(a), reqwest::Url::parse(b)) else {
        return false;
    };
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}

async fn download_media(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    url: &str,
    limit: usize,
    timeout_secs: u64,
) -> ApiResult<(Vec<u8>, String)> {
    if url.starts_with("data:") {
        return decode_data_uri(url).ok_or_else(|| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MEDIA_GEN_FAILED",
                "媒体服务返回了无效的 data URI",
            )
        });
    }
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            "媒体服务没有返回可下载的 URL",
        ));
    }
    let mut request = client
        .get(url)
        .timeout(Duration::from_secs(timeout_secs.clamp(10, 1800)));
    if !api_key.trim().is_empty() && same_origin(base_url, url) {
        request = request.bearer_auth(api_key.trim());
    }
    let response = request.send().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("下载生成产物失败：{err}"),
        )
    })?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(media_failure(status.as_u16(), &body, api_key));
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .split(';')
        .next()
        .unwrap_or("application/octet-stream")
        .to_string();
    if let Some(length) = response.content_length() {
        if length as usize > limit {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MEDIA_GEN_FAILED",
                format!(
                    "生成产物过大（{length} bytes），超过 {} MB 限制",
                    limit / 1024 / 1024
                ),
            ));
        }
    }
    let bytes = response.bytes().await.map_err(|err| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("下载生成产物失败：{err}"),
        )
    })?;
    if bytes.len() > limit {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            format!("生成产物过大，超过 {} MB 限制", limit / 1024 / 1024),
        ));
    }
    Ok((bytes.to_vec(), content_type))
}

fn image_extension(bytes: &[u8], content_type: &str) -> &'static str {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
        "png"
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpg"
    } else if bytes.starts_with(b"GIF8") {
        "gif"
    } else if bytes.len() > 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        "webp"
    } else if content_type.contains("jpeg") {
        "jpg"
    } else if content_type.contains("webp") {
        "webp"
    } else if content_type.contains("gif") {
        "gif"
    } else {
        "png"
    }
}

fn video_extension(bytes: &[u8], content_type: &str) -> &'static str {
    if bytes.len() > 12 && &bytes[4..8] == b"ftyp" {
        if bytes[8..12].starts_with(b"qt") {
            "mov"
        } else {
            "mp4"
        }
    } else if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        "webm"
    } else if content_type.contains("quicktime") {
        "mov"
    } else if content_type.contains("webm") {
        "webm"
    } else if content_type.contains("ogg") {
        "ogv"
    } else {
        "mp4"
    }
}

fn title_for(req: &GenerateRequest) -> String {
    let first = req.instruction.trim().lines().next().unwrap_or("").trim();
    let title: String = first.chars().take(32).collect();
    if title.is_empty() {
        format!(
            "{} · {}",
            kind_label(&req.kind),
            Utc::now().format("%m-%d %H:%M")
        )
    } else {
        title
    }
}

async fn image_bytes(
    client: &reqwest::Client,
    profile: &MediaProfile,
    prompt: &str,
    timeout_secs: u64,
) -> ApiResult<(Vec<u8>, String)> {
    let url = endpoint(
        &profile.base_url,
        &profile.create_path,
        "/images/generations",
    )?;
    let body = json!({
        "model": profile.model,
        "prompt": prompt,
        "n": 1,
    });
    let value = post_json(client, &url, &profile.api_key, &body, timeout_secs).await?;
    let Some((source_url, b64)) = find_image_source(&value) else {
        return Err(ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            "图像接口没有返回 url 或 b64_json。请确认接口兼容 OpenAI Images API。",
        )
        .with_details(json!({ "response": sanitize(&value.to_string(), &profile.api_key) })));
    };
    if let Some(b64) = b64 {
        let bytes = STANDARD.decode(b64).map_err(|err| {
            ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MEDIA_GEN_FAILED",
                format!("图像 base64 解码失败：{err}"),
            )
        })?;
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MEDIA_GEN_FAILED",
                "生成的图像超过体积限制",
            ));
        }
        return Ok((bytes, "image/png".to_string()));
    }
    let url = source_url.unwrap();
    download_media(
        client,
        &profile.base_url,
        &profile.api_key,
        &url,
        MAX_IMAGE_BYTES,
        timeout_secs,
    )
    .await
}

async fn video_bytes(
    client: &reqwest::Client,
    config: &MediaGenConfig,
    profile: &MediaProfile,
    prompt: &str,
) -> ApiResult<(Vec<u8>, String)> {
    let protocol = profile.protocol.trim().to_ascii_lowercase();
    if !["openai", "task", "seedance"].contains(&protocol.as_str()) {
        return Err(ApiError::bad_request(
            "视频生成协议必须是 openai、task 或 seedance",
        ));
    }
    let default_create = if protocol == "seedance" {
        "/contents/generations/tasks"
    } else {
        "/videos"
    };
    let create_url = endpoint(&profile.base_url, &profile.create_path, default_create)?;
    let body = if protocol == "seedance" {
        json!({
            "model": profile.model,
            "content": [{ "type": "text", "text": prompt }]
        })
    } else {
        json!({ "model": profile.model, "prompt": prompt })
    };
    let created = post_json(
        client,
        &create_url,
        &profile.api_key,
        &body,
        config.timeout_secs,
    )
    .await?;

    if let Some(url) = find_video_url(&created) {
        return download_media(
            client,
            &profile.base_url,
            &profile.api_key,
            &url,
            MAX_VIDEO_BYTES,
            config.timeout_secs,
        )
        .await;
    }

    let task_id = find_id(&created).ok_or_else(|| {
        ApiError::new(
            axum::http::StatusCode::BAD_GATEWAY,
            "MEDIA_GEN_FAILED",
            "视频接口既没有返回视频 URL，也没有返回任务 id。",
        )
        .with_details(json!({ "response": sanitize(&created.to_string(), &profile.api_key) }))
    })?;

    let status_url = if profile.status_path.trim().is_empty() {
        format!("{create_url}/{task_id}")
    } else {
        endpoint(
            &profile.base_url,
            &profile.status_path.replace("{id}", &task_id),
            "",
        )?
    };
    let deadline = Instant::now() + Duration::from_secs(config.video_timeout_secs.clamp(30, 3600));
    let poll = Duration::from_secs(config.poll_interval_secs.clamp(1, 60));
    let mut last_status = String::new();
    loop {
        if Instant::now() >= deadline {
            return Err(ApiError::new(
                axum::http::StatusCode::GATEWAY_TIMEOUT,
                "MEDIA_GEN_FAILED",
                format!("视频任务 {task_id} 超时（最后状态：{last_status}）。"),
            ));
        }
        tokio::time::sleep(poll).await;
        let value = get_json(client, &status_url, &profile.api_key, config.timeout_secs).await?;
        if let Some(url) = find_video_url(&value) {
            return download_media(
                client,
                &profile.base_url,
                &profile.api_key,
                &url,
                MAX_VIDEO_BYTES,
                config.timeout_secs,
            )
            .await;
        }
        last_status = find_status(&value).unwrap_or_default();
        if ["failed", "error", "cancelled", "canceled"].contains(&last_status.as_str()) {
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MEDIA_GEN_FAILED",
                format!(
                    "视频任务失败：{}",
                    sanitize(&value.to_string(), &profile.api_key)
                ),
            )
            .with_details(json!({ "taskId": task_id, "status": last_status })));
        }
        if ["succeeded", "completed", "success", "done"].contains(&last_status.as_str()) {
            if protocol == "openai" {
                let content_url = format!("{create_url}/{task_id}/content");
                return download_media(
                    client,
                    &profile.base_url,
                    &profile.api_key,
                    &content_url,
                    MAX_VIDEO_BYTES,
                    config.timeout_secs,
                )
                .await;
            }
            return Err(ApiError::new(
                axum::http::StatusCode::BAD_GATEWAY,
                "MEDIA_GEN_FAILED",
                "视频任务完成，但响应中没有视频 URL。请检查状态路径或服务商响应格式。",
            )
            .with_details(json!({ "taskId": task_id, "status": last_status })));
        }
    }
}

pub async fn generate(
    workspace: &std::path::Path,
    client: &reqwest::Client,
    config: &Config,
    req: GenerateRequest,
    compiled: PromptResult,
) -> ApiResult<MediaOutcome> {
    let project_id = req.project_id.as_deref().ok_or_else(|| {
        ApiError::bad_request("生成插图/视频需要先在项目中操作，因为产物要写入项目素材库。")
    })?;
    let profile = profile_for(&config.media_gen, &req.kind);
    ensure_configured(profile, &req.kind)?;
    let started = Instant::now();
    let (bytes, mime) = if req.kind == "illustration" {
        image_bytes(
            client,
            profile,
            &compiled.prompt,
            config.media_gen.timeout_secs,
        )
        .await?
    } else {
        video_bytes(client, &config.media_gen, profile, &compiled.prompt).await?
    };

    let ext = if req.kind == "illustration" {
        image_extension(&bytes, &mime)
    } else {
        video_extension(&bytes, &mime)
    };
    let file_name = format!("generated.{ext}");
    let title = title_for(&req);
    let mut plan = assets::plan_upload(
        workspace,
        project_id,
        &req.kind,
        None,
        &file_name,
        Some(&title),
    )
    .await?;
    fs::write(&plan.temp_path, &bytes)
        .await
        .map_err(|err| ApiError::internal(format!("写入生成媒体临时文件失败：{err}")))?;
    plan.meta.source = Some("media-gen".into());
    plan.meta.prompt = Some(compiled.prompt.clone());
    plan.content = format!(
        "## 生成提示词\n\n{}\n\n- 模型：{}\n- 来源：媒体生成\n",
        compiled.prompt, profile.model
    );
    let asset = assets::finish_upload(workspace, plan).await?;
    let snapshot_path = llm::save_snapshot(
        workspace,
        project_id,
        &req,
        &compiled.prompt,
        &profile.model,
    )
    .await;

    Ok(MediaOutcome {
        asset,
        prompt: compiled.prompt,
        model: profile.model.clone(),
        latency_ms: started.elapsed().as_millis() as u64,
        snapshot_path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_image_urls_and_base64() {
        assert_eq!(
            find_image_source(&json!({"data":[{"url":"https://example.com/a.png"}]})),
            Some((Some("https://example.com/a.png".to_string()), None))
        );
        assert_eq!(
            find_image_source(&json!({"data":[{"b64_json":"aGVsbG8="}]})),
            Some((None, Some("aGVsbG8=".to_string())))
        );
    }

    #[test]
    fn finds_task_ids_and_video_urls() {
        let value = json!({"task_id":"abc","status":"succeeded","content":{"video_url":"https://cdn/x.mp4"}});
        assert_eq!(find_id(&value).as_deref(), Some("abc"));
        assert_eq!(find_status(&value).as_deref(), Some("succeeded"));
        assert_eq!(find_video_url(&value).as_deref(), Some("https://cdn/x.mp4"));
    }

    #[test]
    fn detects_media_extensions() {
        assert_eq!(
            image_extension(&[0x89, b'P', b'N', b'G'], "application/octet-stream"),
            "png"
        );
        assert_eq!(
            video_extension(b"\0\0\0\x18ftypisom", "application/octet-stream"),
            "mp4"
        );
    }
}
