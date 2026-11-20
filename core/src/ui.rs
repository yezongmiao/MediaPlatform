use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use include_dir::{include_dir, Dir};

static UI: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/../dist");

fn content_type(path: &str) -> &'static str {
    mime_guess::from_path(path)
        .first_raw()
        .unwrap_or("application/octet-stream")
}

fn index_response() -> Response {
    match UI.get_file("index.html") {
        Some(file) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "text/html; charset=utf-8"),
                (header::CACHE_CONTROL, "no-cache"),
            ],
            Body::from(file.contents().to_vec()),
        )
            .into_response(),
        None => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "UI 资源未打包：请先执行 npm run build",
        )
            .into_response(),
    }
}

/// Serve the embedded SPA. Unknown non-API paths fall back to index.html so
/// the History router works on refresh / deep links.
pub async fn fallback(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    if path.starts_with("api/") || path.starts_with("files/") {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            format!("{{\"error\":{{\"code\":\"NOT_FOUND\",\"message\":\"接口不存在：/{path}\"}}}}"),
        )
            .into_response();
    }
    if path.is_empty() {
        return index_response();
    }
    let Some(file) = UI.get_file(path) else {
        return index_response();
    };
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type(path)),
            (header::CACHE_CONTROL, cache),
        ],
        Body::from(file.contents().to_vec()),
    )
        .into_response()
}
