use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "INVALID_INPUT", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "NOT_FOUND", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "PATH_FORBIDDEN", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL", message)
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let payload = json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "details": self.details,
            }
        });
        (self.status, Json(payload)).into_response()
    }
}

impl From<std::io::Error> for ApiError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => Self::not_found("文件或目录不存在"),
            std::io::ErrorKind::PermissionDenied => Self::forbidden("没有权限访问该文件或目录"),
            _ => Self::internal(format!("文件操作失败：{err}")),
        }
    }
}
