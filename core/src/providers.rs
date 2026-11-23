use std::collections::HashSet;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::Serialize;

use crate::error::{ApiError, ApiResult};
use crate::models::{LlmConfig, RequestHeader};

const OPENCODE_SESSION_HEADER: &str = "x-opencode-session";
const PROTECTED_HEADERS: [&str; 5] = [
    "content-type",
    "content-length",
    "host",
    "connection",
    "transfer-encoding",
];

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mark: String,
    pub accent: String,
    pub base_url: String,
    pub protocol: String,
    pub default_model: String,
    pub models_path: String,
    /// bearer | x-api-key | none
    pub auth: String,
    pub requires_session: bool,
    pub note: String,
    pub docs_url: String,
    pub fixed_headers: Vec<RequestHeader>,
    pub fallback_models: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Protocol {
    OpenAi,
    Anthropic,
    Responses,
}

impl Protocol {
    pub fn from_id(id: &str) -> Option<Self> {
        match id.trim().to_ascii_lowercase().as_str() {
            "openai" | "chat" | "chat-completions" => Some(Self::OpenAi),
            "anthropic" | "messages" => Some(Self::Anthropic),
            "responses" | "openai-responses" => Some(Self::Responses),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
            Self::Responses => "responses",
        }
    }
}

fn header(name: &str, value: &str) -> RequestHeader {
    RequestHeader {
        name: name.to_string(),
        value: value.to_string(),
    }
}

pub fn catalog() -> Vec<ProviderSpec> {
    vec![
        ProviderSpec {
            id: "deepseek".into(),
            name: "DeepSeek".into(),
            description: "官方 API，国内网络友好".into(),
            mark: "DS".into(),
            accent: "#4d6bfe".into(),
            base_url: "https://api.deepseek.com/v1".into(),
            protocol: "openai".into(),
            default_model: "deepseek-chat".into(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "DeepSeek 官方 OpenAI 兼容接口，模型列表来自 /models。".into(),
            docs_url: "https://api-docs.deepseek.com/".into(),
            fixed_headers: Vec::new(),
            fallback_models: vec!["deepseek-chat".into(), "deepseek-reasoner".into()],
        },
        ProviderSpec {
            id: "anthropic".into(),
            name: "Claude / Claude Code".into(),
            description: "Anthropic 原生 Messages API".into(),
            mark: "Cl".into(),
            accent: "#d97757".into(),
            base_url: "https://api.anthropic.com".into(),
            protocol: "anthropic".into(),
            default_model: String::new(),
            models_path: "/v1/models".into(),
            auth: "x-api-key".into(),
            requires_session: false,
            note: "使用 Anthropic Messages 协议，自动添加 x-api-key 与 anthropic-version；Claude Code CLI 请在「文本内核」选择。".into(),
            docs_url: "https://docs.anthropic.com/".into(),
            fixed_headers: vec![header("anthropic-version", "2023-06-01")],
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "codex".into(),
            name: "Codex / OpenAI".into(),
            description: "OpenAI Responses API".into(),
            mark: "Cx".into(),
            accent: "#111827".into(),
            base_url: "https://api.openai.com/v1".into(),
            protocol: "responses".into(),
            default_model: String::new(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "使用 Responses API（/responses），适合 Codex 模型；普通 chat 模型可改用「自定义服务」并选择 OpenAI 协议。".into(),
            docs_url: "https://platform.openai.com/docs/api-reference/responses".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "openai".into(),
            name: "OpenAI".into(),
            description: "标准 Chat Completions".into(),
            mark: "OA".into(),
            accent: "#10a37f".into(),
            base_url: "https://api.openai.com/v1".into(),
            protocol: "openai".into(),
            default_model: "gpt-4o-mini".into(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "标准 OpenAI Chat Completions 接口。".into(),
            docs_url: "https://platform.openai.com/docs/api-reference/chat".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "opencode-go".into(),
            name: "OpenCode Go".into(),
            description: "需 x-opencode-session 的托管推理".into(),
            mark: "Go".into(),
            accent: "#2563eb".into(),
            base_url: "https://opencode.ai/inference/go/openai/v1".into(),
            protocol: "openai".into(),
            default_model: "kimi-k3".into(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: true,
            note: "自动添加稳定的 x-opencode-session，避免 MissingSessionID；请求会携带 MediaDesk User-Agent。".into(),
            docs_url: "https://opencode.ai/docs/console/go/".into(),
            fixed_headers: Vec::new(),
            fallback_models: vec!["kimi-k3".into(), "deepseek-v4.1-flash".into()],
        },
        ProviderSpec {
            id: "volcengine".into(),
            name: "字节 · 火山方舟".into(),
            description: "豆包 / Ark API".into(),
            mark: "豆".into(),
            accent: "#1664ff".into(),
            base_url: "https://ark.cn-beijing.volces.com/api/v3".into(),
            protocol: "openai".into(),
            default_model: String::new(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "模型名可填模型 ID 或推理接入点 ID（ep-...）；模型列表来自方舟 /models。".into(),
            docs_url: "https://www.volcengine.com/docs/82379".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "xiaomi".into(),
            name: "小米 MiMo".into(),
            description: "MiMo 开放平台".into(),
            mark: "Mi".into(),
            accent: "#ff6900".into(),
            base_url: "https://api.xiaomimimo.com/v1".into(),
            protocol: "openai".into(),
            default_model: String::new(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "使用 MiMo 的 OpenAI 兼容接口；模型列表来自 /models。".into(),
            docs_url: "https://platform.xiaomimimo.com/".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "openrouter".into(),
            name: "OpenRouter".into(),
            description: "聚合数百个模型".into(),
            mark: "OR".into(),
            accent: "#6366f1".into(),
            base_url: "https://openrouter.ai/api/v1".into(),
            protocol: "openai".into(),
            default_model: "openrouter/auto".into(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "OpenRouter 可选要求 HTTP-Referer / X-Title，已自动附加应用标识。".into(),
            docs_url: "https://openrouter.ai/docs".into(),
            fixed_headers: vec![
                header("HTTP-Referer", "https://github.com/yezongmiao/MediaPlatform"),
                header("X-Title", "MediaDesk"),
            ],
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "kimi".into(),
            name: "Kimi".into(),
            description: "Moonshot / Kimi 开放平台".into(),
            mark: "Ki".into(),
            accent: "#111827".into(),
            base_url: "https://api.moonshot.cn/v1".into(),
            protocol: "openai".into(),
            default_model: String::new(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "Kimi OpenAI 兼容接口；模型列表来自 /models。".into(),
            docs_url: "https://platform.kimi.com/docs/api/overview".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "minimax".into(),
            name: "MiniMax".into(),
            description: "MiniMax 开放平台".into(),
            mark: "MM".into(),
            accent: "#e11d48".into(),
            base_url: "https://api.minimaxi.com/v1".into(),
            protocol: "openai".into(),
            default_model: String::new(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "国际站使用 api.minimaxi.com/v1；国内站可在 Base URL 改为 api.minimax.chat/v1。".into(),
            docs_url: "https://platform.minimaxi.com/document/guides".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "ollama".into(),
            name: "Ollama 本地".into(),
            description: "无需 API Key".into(),
            mark: "Ol".into(),
            accent: "#10b981".into(),
            base_url: "http://localhost:11434/v1".into(),
            protocol: "openai".into(),
            default_model: "qwen2.5:7b".into(),
            models_path: "/models".into(),
            auth: "none".into(),
            requires_session: false,
            note: "本地 Ollama 的 OpenAI 兼容接口，不需要 Key。".into(),
            docs_url: "https://github.com/ollama/ollama/blob/main/docs/openai.md".into(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
        ProviderSpec {
            id: "custom".into(),
            name: "自定义服务".into(),
            description: "任意 OpenAI / Anthropic / Responses 接口".into(),
            mark: "+".into(),
            accent: "#6b7280".into(),
            base_url: String::new(),
            protocol: "openai".into(),
            default_model: String::new(),
            models_path: "/models".into(),
            auth: "bearer".into(),
            requires_session: false,
            note: "兼容任意网关；可切换协议并添加自定义请求头。".into(),
            docs_url: String::new(),
            fixed_headers: Vec::new(),
            fallback_models: Vec::new(),
        },
    ]
}

pub fn get(id: &str) -> Option<ProviderSpec> {
    catalog().into_iter().find(|provider| provider.id == id)
}

fn custom_provider(config: &LlmConfig) -> ProviderSpec {
    ProviderSpec {
        id: "custom".into(),
        name: "自定义服务".into(),
        description: "自定义接口".into(),
        mark: "+".into(),
        accent: "#6b7280".into(),
        base_url: config.base_url.clone(),
        protocol: config.protocol.clone(),
        default_model: config.model.clone(),
        models_path: "/models".into(),
        auth: "bearer".into(),
        requires_session: false,
        note: String::new(),
        docs_url: String::new(),
        fixed_headers: Vec::new(),
        fallback_models: Vec::new(),
    }
}

pub fn for_config(config: &LlmConfig) -> ProviderSpec {
    get(&config.provider_id).unwrap_or_else(|| custom_provider(config))
}

pub fn effective_protocol(config: &LlmConfig) -> ApiResult<Protocol> {
    let spec = for_config(config);
    let id = if spec.id == "custom" {
        config.protocol.as_str()
    } else {
        spec.protocol.as_str()
    };
    Protocol::from_id(id).ok_or_else(|| {
        ApiError::bad_request(format!(
            "未知的模型协议：{id}（可用 openai / anthropic / responses）"
        ))
    })
}

fn validate_base(base_url: &str) -> ApiResult<&str> {
    let base = base_url.trim().trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(ApiError::bad_request(
            "Base URL 必须以 http:// 或 https:// 开头",
        ));
    }
    if reqwest::Url::parse(base).is_err() {
        return Err(ApiError::bad_request("Base URL 格式不正确"));
    }
    Ok(base)
}

fn join(base: &str, path: &str) -> String {
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if base.ends_with("/v1") && path.starts_with("v1/") {
        format!("{base}/{}", path.trim_start_matches("v1/"))
    } else {
        format!("{base}/{path}")
    }
}

pub fn chat_url(config: &LlmConfig) -> ApiResult<String> {
    let base = validate_base(&config.base_url)?;
    let protocol = effective_protocol(config)?;
    let path = match protocol {
        Protocol::OpenAi => {
            if base.ends_with("/chat/completions") {
                return Ok(base.to_string());
            }
            "chat/completions"
        }
        Protocol::Anthropic => {
            if base.ends_with("/messages") {
                return Ok(base.to_string());
            }
            if base.ends_with("/v1") {
                "messages"
            } else {
                "v1/messages"
            }
        }
        Protocol::Responses => {
            if base.ends_with("/responses") {
                return Ok(base.to_string());
            }
            "responses"
        }
    };
    Ok(join(base, path))
}

pub fn models_url(config: &LlmConfig) -> ApiResult<String> {
    let base = validate_base(&config.base_url)?;
    let spec = for_config(config);
    let path = if spec.id == "custom" && effective_protocol(config)? == Protocol::Anthropic {
        "/v1/models"
    } else if spec.models_path.trim().is_empty() {
        "/models"
    } else {
        spec.models_path.as_str()
    };
    if base.ends_with("/models") {
        return Ok(base.to_string());
    }
    Ok(join(base, path))
}

pub fn requires_session(config: &LlmConfig) -> bool {
    let spec = for_config(config);
    spec.requires_session || is_opencode_host(&config.base_url)
}

fn is_opencode_host(base_url: &str) -> bool {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .map(|host| host == "opencode.ai" || host.ends_with(".opencode.ai"))
        .unwrap_or(false)
}

pub fn validate_custom_headers(headers: &[RequestHeader]) -> ApiResult<Vec<RequestHeader>> {
    if headers.len() > 20 {
        return Err(ApiError::bad_request("自定义请求头最多 20 个"));
    }
    let mut seen = HashSet::new();
    let mut cleaned = Vec::new();
    for raw in headers {
        let name = raw.name.trim();
        let value = raw.value.trim();
        if name.is_empty() && value.is_empty() {
            continue;
        }
        if name.is_empty() || value.is_empty() {
            return Err(ApiError::bad_request("自定义请求头的名称和值都不能为空"));
        }
        if name.len() > 64 || value.chars().count() > 4096 {
            return Err(ApiError::bad_request("自定义请求头过长"));
        }
        if value.contains(['\r', '\n']) || name.chars().any(char::is_control) {
            return Err(ApiError::bad_request("自定义请求头不能包含换行或控制字符"));
        }
        let lower = name.to_ascii_lowercase();
        if PROTECTED_HEADERS.contains(&lower.as_str()) {
            return Err(ApiError::bad_request(format!(
                "请求头 `{name}` 由 MediaDesk 管理，不能覆盖"
            )));
        }
        if !name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
        {
            return Err(ApiError::bad_request(format!("请求头名称不合法：{name}")));
        }
        if !seen.insert(lower) {
            return Err(ApiError::bad_request(format!("请求头 `{name}` 重复")));
        }
        HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| ApiError::bad_request(format!("请求头名称不合法：{name}")))?;
        HeaderValue::from_str(value)
            .map_err(|_| ApiError::bad_request(format!("请求头值不合法：{name}")))?;
        cleaned.push(RequestHeader {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
    Ok(cleaned)
}

pub fn build_headers(config: &LlmConfig, session: &str) -> ApiResult<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(
        reqwest::header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    let spec = for_config(config);
    let key = config.api_key.trim();
    if !key.is_empty() {
        let (name, value) = if spec.auth == "x-api-key" {
            ("x-api-key", key.to_string())
        } else {
            ("authorization", format!("Bearer {key}"))
        };
        let name = HeaderName::from_bytes(name.as_bytes())
            .map_err(|_| ApiError::bad_request("认证请求头名称不合法"))?;
        let value = HeaderValue::from_str(&value)
            .map_err(|_| ApiError::bad_request("API Key 包含非法字符"))?;
        headers.insert(name, value);
    }
    for fixed in &spec.fixed_headers {
        let name = HeaderName::from_bytes(fixed.name.as_bytes())
            .map_err(|_| ApiError::bad_request("服务商固定请求头名称不合法"))?;
        let value = HeaderValue::from_str(&fixed.value)
            .map_err(|_| ApiError::bad_request("服务商固定请求头值不合法"))?;
        headers.insert(name, value);
    }
    for custom in validate_custom_headers(&config.custom_headers)? {
        let name = HeaderName::from_bytes(custom.name.as_bytes())
            .map_err(|_| ApiError::bad_request("自定义请求头名称不合法"))?;
        let value = HeaderValue::from_str(&custom.value)
            .map_err(|_| ApiError::bad_request("自定义请求头值不合法"))?;
        headers.insert(name, value);
    }
    if requires_session(config)
        && !headers.contains_key(OPENCODE_SESSION_HEADER)
        && !session.trim().is_empty()
    {
        let value = HeaderValue::from_str(session)
            .map_err(|_| ApiError::bad_request("OpenCode 会话 ID 不合法"))?;
        headers.insert(HeaderName::from_static(OPENCODE_SESSION_HEADER), value);
    }
    Ok(headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::AUTHORIZATION;

    #[test]
    fn catalog_contains_required_providers() {
        for id in [
            "anthropic",
            "codex",
            "opencode-go",
            "volcengine",
            "xiaomi",
            "openrouter",
            "kimi",
            "minimax",
            "custom",
        ] {
            assert!(get(id).is_some(), "{id}");
        }
    }

    #[test]
    fn opencode_session_header_is_automatic() {
        let config = LlmConfig {
            provider_id: "opencode-go".into(),
            base_url: "https://opencode.ai/inference/go/openai/v1".into(),
            api_key: "test".into(),
            ..LlmConfig::default()
        };
        let headers = build_headers(&config, "mediadesk-session-1").unwrap();
        assert_eq!(
            headers.get(OPENCODE_SESSION_HEADER).unwrap(),
            "mediadesk-session-1"
        );
        assert_eq!(headers.get(AUTHORIZATION).unwrap(), "Bearer test");
    }

    #[test]
    fn anthropic_auth_uses_x_api_key_and_version() {
        let config = LlmConfig {
            provider_id: "anthropic".into(),
            base_url: "https://api.anthropic.com".into(),
            api_key: "sk-ant-test".into(),
            ..LlmConfig::default()
        };
        let headers = build_headers(&config, "s").unwrap();
        assert_eq!(headers.get("x-api-key").unwrap(), "sk-ant-test");
        assert_eq!(headers.get("anthropic-version").unwrap(), "2023-06-01");
        assert!(headers.get(AUTHORIZATION).is_none());
        assert_eq!(
            models_url(&config).unwrap(),
            "https://api.anthropic.com/v1/models"
        );
        assert_eq!(
            chat_url(&config).unwrap(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn protocol_urls_are_built() {
        let codex = LlmConfig {
            provider_id: "codex".into(),
            base_url: "https://api.openai.com/v1".into(),
            ..LlmConfig::default()
        };
        assert_eq!(
            chat_url(&codex).unwrap(),
            "https://api.openai.com/v1/responses"
        );
        let openai = LlmConfig {
            provider_id: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            ..LlmConfig::default()
        };
        assert_eq!(
            chat_url(&openai).unwrap(),
            "https://api.openai.com/v1/chat/completions"
        );
    }
}
