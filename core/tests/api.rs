use std::path::{Path, PathBuf};

use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use mediadesk_core::api::{self, AppState};
use mediadesk_core::{context, store};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::net::TcpListener;

struct TestServer {
    base: String,
    client: reqwest::Client,
    workspace: TempDir,
    state_dir: TempDir,
}

impl TestServer {
    async fn spawn() -> Self {
        Self::spawn_with_state(|_| {}).await
    }

    async fn spawn_with_state(configure: impl FnOnce(&Path)) -> Self {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let state_dir = tempfile::tempdir().expect("state tempdir");
        configure(workspace.path());
        let state = AppState::bootstrap(
            Some(workspace.path().to_path_buf()),
            Some(state_dir.path().to_path_buf()),
        )
        .await
        .expect("bootstrap");
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let router = api::router(state);
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        let base = format!("http://{addr}");
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("client");
        for _ in 0..50 {
            if client
                .get(format!("{base}/api/health"))
                .send()
                .await
                .is_ok()
            {
                return Self {
                    base,
                    client,
                    workspace,
                    state_dir,
                };
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        panic!("server did not start");
    }

    fn ws(&self) -> &Path {
        self.workspace.path()
    }

    async fn call(&self, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
        let mut request = self
            .client
            .request(method.parse().unwrap(), format!("{}{}", self.base, path));
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await.expect("request");
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        let value = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        (status, value)
    }

    async fn create_idea(&self, title: &str, content: &str) -> Value {
        let (status, body) = self
            .call(
                "POST",
                "/api/ideas",
                Some(json!({"title": title, "content": content, "tags": ["测试"]})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }

    async fn create_project(&self, title: &str, idea_ids: Vec<Value>) -> Value {
        let ids: Vec<String> = idea_ids
            .into_iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let (status, body) = self
            .call(
                "POST",
                "/api/projects",
                Some(json!({"title": title, "brief": "测试项目简介", "ideaIds": ids})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        body
    }
}

fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                out.push(path);
            }
        }
    }
    out
}

async fn mock_llm() -> (String, tokio::task::JoinHandle<()>) {
    async fn handler(headers: HeaderMap, Json(payload): Json<Value>) -> Response {
        let auth = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if auth == "Bearer bad-key" {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": {"message": "invalid api key bad-key"}})),
            )
                .into_response();
        }
        let instruction = payload
            .get("messages")
            .and_then(|m| m.as_array())
            .and_then(|m| m.last())
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("");
        Json(json!({
            "id": "mock-chat",
            "model": "mock-model-1",
            "choices": [{"index": 0, "message": {"role": "assistant", "content": format!("# Mock 生成结果\n\n收到指令：{instruction}\n\n这是端到端测试内容。")}}],
            "usage": {"prompt_tokens": 11, "completion_tokens": 22, "total_tokens": 33}
        }))
        .into_response()
    }
    let app = Router::new().route("/v1/chat/completions", post(handler));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}/v1"), handle)
}

async fn mock_media() -> (String, tokio::task::JoinHandle<()>) {
    async fn image() -> Response {
        let bytes = include_bytes!("fixtures/test.png");
        Json(json!({"data": [{"b64_json": STANDARD.encode(bytes)}]})).into_response()
    }

    async fn create_video() -> Response {
        Json(json!({"id": "task-1", "status": "queued"})).into_response()
    }

    async fn task_status(State(base): State<String>) -> Response {
        Json(json!({
            "id": "task-1",
            "status": "completed",
            "video_url": format!("{base}/clip.mp4")
        }))
        .into_response()
    }

    async fn create_seedance() -> Response {
        Json(json!({"id": "seedance-1", "status": "queued"})).into_response()
    }

    async fn seedance_status(State(base): State<String>) -> Response {
        Json(json!({
            "id": "seedance-1",
            "status": "succeeded",
            "content": {"video_url": format!("{base}/clip.mp4")}
        }))
        .into_response()
    }

    async fn clip() -> Response {
        (
            [(axum::http::header::CONTENT_TYPE, "video/mp4")],
            include_bytes!("fixtures/clip.mp4").to_vec(),
        )
            .into_response()
    }

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base = format!("http://{addr}");
    let app = Router::new()
        .route("/v1/images/generations", post(image))
        .route("/v1/videos", post(create_video))
        .route("/v1/videos/task-1", get(task_status))
        .route("/v1/contents/generations/tasks", post(create_seedance))
        .route(
            "/v1/contents/generations/tasks/seedance-1",
            get(seedance_status),
        )
        .route("/clip.mp4", get(clip))
        .with_state(base);
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}/v1"), handle)
}

// ---------------------------------------------------------------------------

#[tokio::test]
async fn workspace_bootstraps_with_seed_and_masked_config() {
    let server = TestServer::spawn().await;
    let (status, health) = server.call("GET", "/api/health", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(health["ok"], true);

    let (_, workspace) = server.call("GET", "/api/workspace", None).await;
    assert_eq!(workspace["ideas"], 0);
    assert_eq!(workspace["projects"], 0);

    let (_, skills) = server.call("GET", "/api/skills", None).await;
    assert_eq!(skills.as_array().unwrap().len(), 7);
    let (_, presets) = server.call("GET", "/api/presets", None).await;
    assert_eq!(presets.as_array().unwrap().len(), 5);

    let (_, config) = server.call("GET", "/api/config", None).await;
    assert!(config["llm"].get("apiKey").is_none());
    assert_eq!(config["llm"]["hasApiKey"], false);
    assert!(server.ws().join(".mediadesk/config.json").exists());
    assert!(server.ws().join(".mediadesk/seed.json").exists());
    assert!(server.ws().join(".gitignore").exists());

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(server.ws().join(".mediadesk/config.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}

#[tokio::test]
async fn idea_crud_promote_delete_and_trash() {
    let server = TestServer::spawn().await;
    let idea = server
        .create_idea("关于长期创作的想法", "正文第一行\n\n---\n正文里也有分隔线")
        .await;
    let idea_id = idea["id"].as_str().unwrap().to_string();
    assert_eq!(idea["status"], "inbox");
    assert!(idea["content"]
        .as_str()
        .unwrap()
        .contains("正文里也有分隔线"));

    let files = walk_files(&server.ws().join("ideas"));
    assert_eq!(files.len(), 1);
    assert!(files[0].to_string_lossy().contains("关于长期创作的想法"));

    let (status, updated) = server
        .call(
            "PUT",
            &format!("/api/ideas/{idea_id}"),
            Some(json!({"title": "改过的标题", "tags": ["新标签"], "content": "新正文"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "改过的标题");
    assert_eq!(updated["tags"][0], "新标签");

    let (status, project) = server
        .call(
            "POST",
            &format!("/api/ideas/{idea_id}/promote"),
            Some(json!({"title": "从想法立项", "brief": "立项简介"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{project}");
    let project_id = project["id"].as_str().unwrap().to_string();
    assert_eq!(project["ideaIds"][0], idea_id);
    assert_eq!(project["counts"]["article"], 0);
    assert!(project["assets"].get("video").is_some());
    assert!(server.ws().join("projects/从想法立项/project.md").exists());

    let (_, idea_after) = server
        .call("GET", &format!("/api/ideas/{idea_id}"), None)
        .await;
    assert_eq!(idea_after["status"], "promoted");
    assert_eq!(idea_after["projectId"], project_id);

    let (status, _) = server
        .call("DELETE", &format!("/api/projects/{project_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, idea_restored) = server
        .call("GET", &format!("/api/ideas/{idea_id}"), None)
        .await;
    assert_eq!(idea_restored["status"], "inbox");
    assert!(idea_restored["projectId"].is_null());
    let trash = walk_files(&server.ws().join(".mediadesk/trash"));
    assert!(trash
        .iter()
        .any(|p| p.to_string_lossy().contains("从想法立项")));

    let (status, _) = server
        .call("DELETE", &format!("/api/ideas/{idea_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = server
        .call("GET", &format!("/api/ideas/{idea_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(walk_files(&server.ws().join(".mediadesk/trash"))
        .iter()
        .any(|p| p.to_string_lossy().contains("关于长期创作的想法")));
}

#[tokio::test]
async fn project_and_all_asset_kinds() {
    let server = TestServer::spawn().await;
    let idea = server.create_idea("素材测试", "做一个完整的项目").await;
    let project = server
        .create_project("五类素材项目", vec![idea["id"].clone()])
        .await;
    let project_id = project["id"].as_str().unwrap().to_string();

    let mut ids = Vec::new();
    for (kind, title) in [
        ("article", "测试文章"),
        ("script", "测试脚本"),
        ("illustration", "测试插图提示词"),
        ("card", "测试文卡文案"),
        ("video", "测试视频笔记"),
    ] {
        let (status, asset) = server
            .call(
                "POST",
                &format!("/api/projects/{project_id}/assets"),
                Some(json!({"kind": kind, "title": title, "content": format!("{title} 内容"), "source": "manual"})),
            )
            .await;
        assert_eq!(status, StatusCode::OK, "{kind}: {asset}");
        assert_eq!(asset["kind"], kind);
        ids.push((kind, asset["id"].as_str().unwrap().to_string()));
    }

    let (_, detail) = server
        .call("GET", &format!("/api/projects/{project_id}"), None)
        .await;
    for (kind, _) in &ids {
        assert_eq!(detail["counts"][kind], 1, "{kind}");
    }

    let article_id = &ids[0].1;
    let (status, updated) = server
        .call(
            "PUT",
            &format!("/api/projects/{project_id}/assets/{article_id}"),
            Some(json!({"title": "文章改名", "content": "# 新正文", "status": "ready", "tags": ["定稿"]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["title"], "文章改名");
    assert_eq!(updated["status"], "ready");
    let article_files = walk_files(&server.ws().join("projects/五类素材项目/article"));
    assert_eq!(article_files.len(), 1);

    let script_id = &ids[1].1;
    let (status, _) = server
        .call(
            "DELETE",
            &format!("/api/projects/{project_id}/assets/{script_id}"),
            None,
        )
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, detail) = server
        .call("GET", &format!("/api/projects/{project_id}"), None)
        .await;
    assert_eq!(detail["counts"]["script"], 0);

    let (status, _) = server
        .call(
            "POST",
            &format!("/api/projects/{project_id}/assets"),
            Some(json!({"kind": "unknown", "title": "非法"})),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn upload_range_adoption_and_public_file_safety() {
    let server = TestServer::spawn().await;
    let project = server.create_project("上传测试", vec![]).await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let png = include_bytes!("fixtures/test.png");

    let upload_url = format!(
        "/api/projects/{project_id}/uploads?kind=illustration&filename=%E5%B0%81%E9%9D%A2.png&title=%E5%B0%81%E9%9D%A2"
    );
    let response = server
        .client
        .post(format!("{}{}", server.base, upload_url))
        .header("Content-Type", "image/png")
        .body(png.to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let asset: Value = response.json().await.unwrap();
    let file_url = asset["fileUrl"].as_str().unwrap().to_string();
    assert!(asset["kind"] == "illustration");
    assert!(asset["file"].as_str().unwrap().ends_with(".png"));

    let raw = server
        .client
        .get(format!("{}{}", server.base, file_url))
        .send()
        .await
        .unwrap();
    assert_eq!(raw.status(), StatusCode::OK);
    assert_eq!(raw.headers()["content-type"], "image/png");
    assert_eq!(raw.bytes().await.unwrap().as_ref(), png);

    let ranged = server
        .client
        .get(format!("{}{}", server.base, file_url))
        .header("Range", "bytes=0-15")
        .send()
        .await
        .unwrap();
    assert_eq!(ranged.status(), StatusCode::PARTIAL_CONTENT);
    assert!(ranged.headers().contains_key("content-range"));
    assert_eq!(ranged.bytes().await.unwrap().len(), 16);

    let bad = server
        .client
        .post(format!(
            "{}/api/projects/{project_id}/uploads?kind=illustration&filename=note.txt",
            server.base
        ))
        .body("not an image")
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);

    // Manually drop a file into the project; rescan must adopt it.
    let illustration_dir = walk_files(&server.ws().join("projects"))
        .iter()
        .find_map(|p| {
            p.parent()
                .map(Path::to_path_buf)
                .filter(|p| p.ends_with("illustration"))
        })
        .expect("illustration dir");
    std::fs::write(illustration_dir.join("外部手绘.jpg"), png).unwrap();
    let (status, rescanned) = server
        .call(
            "POST",
            &format!("/api/projects/{project_id}/rescan"),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{rescanned}");
    let adopted = rescanned["assets"]["illustration"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["title"] == "外部手绘")
        .expect("adopted asset");
    let adopted_url = adopted["fileUrl"].as_str().unwrap();
    let raw = server
        .client
        .get(format!("{}{}", server.base, adopted_url))
        .send()
        .await
        .unwrap();
    assert_eq!(raw.status(), StatusCode::OK);
    assert!(illustration_dir.join("外部手绘.md").exists());

    // Traversal through the public file resolver is rejected.
    let err = store::resolve_public_file(server.ws(), "projects/../../.mediadesk/config.json")
        .await
        .unwrap_err();
    assert_eq!(err.code, "PATH_FORBIDDEN");
    let err = store::resolve_public_file(server.ws(), ".mediadesk/config.json")
        .await
        .unwrap_err();
    assert_eq!(err.code, "PATH_FORBIDDEN");

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let leak = illustration_dir.join("leak.png");
        symlink(server.ws().join(".mediadesk/config.json"), &leak).unwrap();
        let err =
            store::resolve_public_file(server.ws(), "projects/上传测试/illustration/leak.png")
                .await
                .unwrap_err();
        assert_eq!(err.code, "PATH_FORBIDDEN");
        let _ = std::fs::remove_file(leak);
    }

    // Video upload + range.
    let video = include_bytes!("fixtures/clip.mp4");
    let response = server
        .client
        .post(format!(
            "{}/api/projects/{project_id}/uploads?kind=video&filename=clip.mp4",
            server.base
        ))
        .body(video.to_vec())
        .send()
        .await
        .unwrap();
    let video_asset: Value = response.json().await.unwrap();
    let video_url = video_asset["fileUrl"].as_str().unwrap();
    let ranged = server
        .client
        .get(format!("{}{}", server.base, video_url))
        .header("Range", "bytes=10-99")
        .send()
        .await
        .unwrap();
    assert_eq!(ranged.status(), StatusCode::PARTIAL_CONTENT);
    assert_eq!(ranged.bytes().await.unwrap().len(), 90);
}

#[tokio::test]
async fn llm_prompt_generation_errors_and_snapshot() {
    let server = TestServer::spawn().await;
    let idea = server
        .create_idea("AI 与个人创作", "我真正关心的是普通人如何用 AI 放大表达。")
        .await;
    let project = server
        .create_project("AI 创作项目", vec![idea["id"].clone()])
        .await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let (llm_url, _mock) = mock_llm().await;

    // Preview does not need a key and includes context, preset, skill.
    let payload = json!({
        "projectId": project_id,
        "kind": "article",
        "presetId": null,
        "skillIds": [],
        "instruction": "写一篇测试文章"
    });
    let (status, prompt) = server
        .call("POST", "/api/llm/prompt", Some(payload.clone()))
        .await;
    assert_eq!(status, StatusCode::OK, "{prompt}");
    let text = prompt["prompt"].as_str().unwrap();
    assert!(text.contains("AI 创作项目"));
    assert!(text.contains("普通人如何用 AI 放大表达"));
    assert!(text.contains("公众号深度长文"));
    assert!(text.contains("强钩子开头"));
    assert!(text.contains("写一篇测试文章"));

    // Remote provider without key -> actionable 412.
    let (status, error) = server
        .call(
            "PUT",
            "/api/config",
            Some(json!({
                "baseUrl": "https://api.example.com/v1",
                "apiKey": "",
                "model": "demo",
                "temperature": 0.7,
                "maxTokens": 1000,
                "timeoutSecs": 30
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{error}");
    let (status, error) = server
        .call("POST", "/api/llm/generate", Some(payload.clone()))
        .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(error["error"]["code"], "LLM_NOT_CONFIGURED");

    // Mock provider success.
    let (status, config) = server
        .call(
            "PUT",
            "/api/config",
            Some(json!({
                "baseUrl": llm_url,
                "apiKey": "good-key",
                "model": "mock-model",
                "temperature": 0.4,
                "maxTokens": 512,
                "timeoutSecs": 20
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{config}");
    assert_eq!(config["llm"]["hasApiKey"], true);
    assert!(config["llm"].get("apiKey").is_none());

    let (status, generated) = server
        .call("POST", "/api/llm/generate", Some(payload.clone()))
        .await;
    assert_eq!(status, StatusCode::OK, "{generated}");
    assert_eq!(generated["model"], "mock-model-1");
    assert_eq!(generated["usage"]["totalTokens"], 33);
    assert!(generated["content"]
        .as_str()
        .unwrap()
        .contains("Mock 生成结果"));
    let snapshot = generated["snapshotPath"].as_str().unwrap();
    assert!(server.ws().join(snapshot).exists());
    let snapshot_text = std::fs::read_to_string(server.ws().join(snapshot)).unwrap();
    assert!(snapshot_text.contains("完整提示词"));
    assert!(snapshot_text.contains("AI 创作项目"));

    // Bad key maps to a clean provider error without leaking the key.
    let (status, _) = server
        .call(
            "PUT",
            "/api/config",
            Some(json!({
                "baseUrl": llm_url,
                "apiKey": "bad-key",
                "model": "mock-model",
                "temperature": 0.4,
                "maxTokens": 512,
                "timeoutSecs": 20
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, error) = server
        .call("POST", "/api/llm/generate", Some(payload))
        .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(error["error"]["code"], "LLM_REQUEST_FAILED");
    let message = error["error"]["message"].as_str().unwrap();
    assert!(!message.contains("bad-key"));
    assert!(message.contains("401"));
}

#[tokio::test]
async fn text_generation_routes_through_cli_kernel() {
    let server = TestServer::spawn().await;
    let idea = server
        .create_idea("CLI 内核测试", "用本地 CLI 生成内容")
        .await;
    let project = server
        .create_project("CLI 内核项目", vec![idea["id"].clone()])
        .await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let dir_name = project["dirName"].as_str().unwrap().to_string();
    let cli_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mock-cli.sh");
    assert!(cli_path.is_file());

    let (status, config) = server
        .call(
            "PUT",
            "/api/config/kernel",
            Some(json!({
                "kernel": "claude",
                "binary": cli_path.to_string_lossy(),
                "args": [],
                "timeoutSecs": 60
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{config}");
    assert_eq!(config["textKernel"], "claude");

    let (status, status_body) = server.call("GET", "/api/cli/status", None).await;
    assert_eq!(status, StatusCode::OK);
    let claude = status_body["kernels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|kernel| kernel["id"] == "claude")
        .unwrap();
    assert_eq!(claude["found"], true);
    assert!(claude["path"].as_str().unwrap().ends_with("mock-cli.sh"));

    let payload = json!({
        "projectId": project_id,
        "kind": "article",
        "presetId": null,
        "skillIds": [],
        "instruction": "用 CLI 写一段测试"
    });
    let (status, generated) = server
        .call("POST", "/api/llm/generate", Some(payload))
        .await;
    assert_eq!(status, StatusCode::OK, "{generated}");
    assert_eq!(generated["kernel"], "claude");
    assert_eq!(generated["model"], "claude-cli");
    let content = generated["content"].as_str().unwrap();
    assert!(content.contains("Mock CLI 生成结果"), "{content}");
    let expected_cwd = std::fs::canonicalize(server.ws().join("projects").join(&dir_name)).unwrap();
    assert!(
        content.contains(&format!("工作目录：{}", expected_cwd.display())),
        "{content}"
    );
    let snapshot = generated["snapshotPath"].as_str().unwrap();
    assert!(server.ws().join(snapshot).exists());

    // Missing binaries produce an actionable CLI_NOT_FOUND.
    let (status, _) = server
        .call(
            "PUT",
            "/api/config/kernel",
            Some(json!({
                "kernel": "claude",
                "binary": "/definitely/not/a/cli",
                "args": [],
                "timeoutSecs": 30
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    let (status, error) = server
        .call(
            "POST",
            "/api/llm/generate",
            Some(json!({
                "projectId": project_id,
                "kind": "script",
                "presetId": null,
                "skillIds": [],
                "instruction": "测试缺失 CLI"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(error["error"]["code"], "CLI_NOT_FOUND");
    assert!(error["error"]["message"]
        .as_str()
        .unwrap()
        .contains("claude"));

    let (_, status_body) = server.call("GET", "/api/cli/status", None).await;
    let claude = status_body["kernels"]
        .as_array()
        .unwrap()
        .iter()
        .find(|kernel| kernel["id"] == "claude")
        .unwrap();
    assert_eq!(claude["found"], false);
}

#[tokio::test]
async fn media_generation_saves_illustration_and_video() {
    let server = TestServer::spawn().await;
    let idea = server
        .create_idea("视觉测试", "暖纸色、克制的编辑视觉")
        .await;
    let project = server
        .create_project("视觉项目", vec![idea["id"].clone()])
        .await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let dir_name = project["dirName"].as_str().unwrap().to_string();
    let (media_url, _mock) = mock_media().await;

    let (status, config) = server
        .call(
            "PUT",
            "/api/config/media",
            Some(json!({
                "illustration": {
                    "baseUrl": media_url,
                    "apiKey": "media-secret",
                    "model": "mock-image",
                    "protocol": "openai",
                    "createPath": "",
                    "statusPath": "",
                    "style": "暖纸色编辑插画"
                },
                "video": {
                    "baseUrl": media_url,
                    "apiKey": "media-secret",
                    "model": "mock-video",
                    "protocol": "openai",
                    "createPath": "/videos",
                    "statusPath": "",
                    "style": "克制的纪实镜头"
                },
                "timeoutSecs": 30,
                "pollIntervalSecs": 1,
                "videoTimeoutSecs": 60
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{config}");
    assert!(config["mediaGen"]["illustration"].get("apiKey").is_none());
    assert_eq!(config["mediaGen"]["illustration"]["hasApiKey"], true);

    let (status, illustration) = server
        .call(
            "POST",
            "/api/llm/generate",
            Some(json!({
                "projectId": project_id,
                "kind": "illustration",
                "presetId": null,
                "skillIds": [],
                "instruction": "晨光下的书桌"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{illustration}");
    assert_eq!(illustration["kernel"], "media");
    assert_eq!(illustration["model"], "mock-image");
    let asset = &illustration["asset"];
    assert_eq!(asset["kind"], "illustration");
    assert!(
        asset["file"].as_str().unwrap().ends_with(".png"),
        "{illustration}"
    );
    assert!(server
        .ws()
        .join(asset["relPath"].as_str().unwrap())
        .exists());
    let raw = server
        .client
        .get(format!(
            "{}{}",
            server.base,
            asset["fileUrl"].as_str().unwrap()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(raw.status(), StatusCode::OK);
    assert!(raw.bytes().await.unwrap().len() > 20);
    let snapshot = illustration["snapshotPath"].as_str().unwrap();
    assert!(server.ws().join(snapshot).exists());

    let (status, video) = server
        .call(
            "POST",
            "/api/llm/generate",
            Some(json!({
                "projectId": project_id,
                "kind": "video",
                "presetId": null,
                "skillIds": [],
                "instruction": "一段书桌前的慢镜头"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{video}");
    assert_eq!(video["kernel"], "media");
    assert_eq!(video["model"], "mock-video");
    assert!(video["asset"]["file"].as_str().unwrap().ends_with(".mp4"));
    assert!(server
        .ws()
        .join(video["asset"]["relPath"].as_str().unwrap())
        .exists());

    // Both generated files are real project assets and carry media-gen provenance.
    let project_dir = server.ws().join("projects").join(&dir_name);
    let image_files = walk_files(&project_dir.join("illustration"))
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("png"))
        .count();
    assert!(image_files >= 1);
    let video_files = walk_files(&project_dir.join("video"))
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("mp4"))
        .count();
    assert!(video_files >= 1);
    let sidecar = std::fs::read_to_string(
        std::path::Path::new(server.ws()).join(asset["relPath"].as_str().unwrap()),
    )
    .unwrap();
    assert!(sidecar.contains("media-gen"));
    assert!(sidecar.contains("晨光下的书桌"));
}

#[tokio::test]
async fn assistant_suggests_and_applies_asset_edits() {
    let server = TestServer::spawn().await;
    let idea = server.create_idea("助手测试", "需要一位懂项目的助手").await;
    let project = server
        .create_project("助手项目", vec![idea["id"].clone()])
        .await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let (status, asset) = server
        .call(
            "POST",
            &format!("/api/projects/{project_id}/assets"),
            Some(json!({
                "kind": "article",
                "title": "原始文章",
                "content": "# 原始文章\n\n这是原始正文。"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{asset}");
    let asset_id = asset["id"].as_str().unwrap().to_string();

    let cli_path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mock-cli.sh");
    let (status, _) = server
        .call(
            "PUT",
            "/api/config/kernel",
            Some(json!({
                "kernel": "claude",
                "binary": cli_path.to_string_lossy(),
                "args": [],
                "timeoutSecs": 60
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, suggested) = server
        .call(
            "POST",
            &format!("/api/projects/{project_id}/chat"),
            Some(json!({
                "message": "帮我把正文改得更紧凑。",
                "mode": "suggest"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{suggested}");
    assert_eq!(suggested["kernel"], "claude");
    assert_eq!(suggested["applied"], false);
    assert_eq!(suggested["edits"][0]["assetId"], asset_id);
    assert_eq!(suggested["messages"].as_array().unwrap().len(), 2);
    let (_, original) = server
        .call(
            "GET",
            &format!("/api/projects/{project_id}/assets/{asset_id}"),
            None,
        )
        .await;
    assert!(original["content"].as_str().unwrap().contains("原始正文"));

    let (status, applied) = server
        .call(
            "POST",
            &format!("/api/projects/{project_id}/chat"),
            Some(json!({
                "message": "直接覆盖它。",
                "mode": "apply"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["applied"], true);
    assert_eq!(applied["messages"].as_array().unwrap().len(), 4);
    let (_, updated) = server
        .call(
            "GET",
            &format!("/api/projects/{project_id}/assets/{asset_id}"),
            None,
        )
        .await;
    assert!(updated["content"].as_str().unwrap().contains("助手修订版"));

    let (status, history) = server
        .call("GET", &format!("/api/projects/{project_id}/chat"), None)
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(history["messages"].as_array().unwrap().len(), 4);
    assert_eq!(history["kernel"], "claude");
    let history_file = server
        .ws()
        .join("projects")
        .join(project["dirName"].as_str().unwrap())
        .join(".mediadesk/assistant/history.json");
    assert!(history_file.exists());

    let (status, _) = server
        .call("DELETE", &format!("/api/projects/{project_id}/chat"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert!(!history_file.exists());
    let (_, history) = server
        .call("GET", &format!("/api/projects/{project_id}/chat"), None)
        .await;
    assert_eq!(history["messages"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn media_generation_requires_configuration() {
    let server = TestServer::spawn().await;
    let project = server.create_project("未配置媒体项目", vec![]).await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let (status, error) = server
        .call(
            "POST",
            "/api/llm/generate",
            Some(json!({
                "projectId": project_id,
                "kind": "illustration",
                "presetId": null,
                "skillIds": [],
                "instruction": "测试未配置"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(error["error"]["code"], "MEDIA_NOT_CONFIGURED");
}

#[tokio::test]
async fn vscode_context_files_and_launch_fallback() {
    let server = TestServer::spawn().await;
    let idea = server
        .create_idea("VSCode 测试", "为外部 AI 准备上下文")
        .await;
    let project = server
        .create_project("VSCode 项目", vec![idea["id"].clone()])
        .await;
    let project_id = project["id"].as_str().unwrap().to_string();
    let skills = server.call("GET", "/api/skills", None).await.1;
    let chosen_skill = skills[0]["id"].as_str().unwrap().to_string();
    let (status, _) = server
        .call(
            "PUT",
            &format!("/api/projects/{project_id}"),
            Some(json!({"skillIds": [chosen_skill.clone()]})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (status, summary) = server
        .call(
            "POST",
            &format!("/api/projects/{project_id}/context"),
            Some(json!({})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{summary}");
    let dir_name = project["dirName"].as_str().unwrap();
    let project_dir = server.ws().join("projects").join(dir_name);
    for file in [
        "AGENTS.md",
        "CLAUDE.md",
        "README.md",
        ".vscode/settings.json",
        ".vscode/tasks.json",
        ".mediadesk/context.json",
        "prompts/README.md",
    ] {
        assert!(project_dir.join(file).exists(), "missing {file}");
    }
    let agents = std::fs::read_to_string(project_dir.join("AGENTS.md")).unwrap();
    assert!(agents.contains("VSCode 项目"));
    assert!(agents.contains("为外部 AI 准备上下文"));
    assert!(agents.contains("协作规则"));
    let skill_files: Vec<_> = walk_files(&project_dir.join("skills"));
    assert_eq!(skill_files.len(), 1, "only selected skill should be copied");
    assert!(skill_files[0].to_string_lossy().contains(&chosen_skill));
    let copied_presets = walk_files(&project_dir.join("presets")).len();
    assert_eq!(copied_presets, 5);
    let context_json: Value = serde_json::from_str(
        &std::fs::read_to_string(project_dir.join(".mediadesk/context.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(context_json["schemaVersion"], 1);
    assert_eq!(context_json["project"]["title"], "VSCode 项目");

    #[cfg(unix)]
    {
        let result = context::launch(server.ws(), &project_id, Some("/usr/bin/true"))
            .await
            .expect("fake code launch");
        assert!(result.ok);
        assert!(result.files_written.len() > 10);
        let missing = context::launch(
            server.ws(),
            &project_id,
            Some("/definitely/not/a/real/code-binary"),
        )
        .await
        .unwrap_err();
        assert_eq!(missing.code, "VSCODE_NOT_FOUND");
        let details = missing.details.unwrap();
        assert!(details["manualCommand"].as_str().unwrap().contains("code"));
        assert!(details["filesWritten"].as_array().unwrap().len() > 10);
    }
}

#[tokio::test]
async fn search_workspace_switch_and_browse() {
    let server = TestServer::spawn().await;
    let idea = server
        .create_idea("独一无二的关键词星云", "这是搜索测试的正文")
        .await;
    let _ = server
        .create_project("搜索测试项目", vec![idea["id"].clone()])
        .await;

    let (status, result) = server
        .call(
            "GET",
            "/api/search?q=%E7%8B%AC%E4%B8%80%E6%97%A0%E4%BA%8C",
            None,
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(result["ideas"].as_array().unwrap().len(), 1);
    assert_eq!(result["projects"].as_array().unwrap().len(), 1);

    let (status, browse) = server.call("GET", "/api/fs/browse", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(browse["dirs"].is_array());

    let second = tempfile::tempdir().unwrap();
    let (status, switched) = server
        .call(
            "POST",
            "/api/workspace",
            Some(json!({"path": second.path().to_string_lossy()})),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{switched}");
    assert_eq!(switched["projects"], 0);
    assert_eq!(switched["ideas"], 0);
    assert!(second.path().join(".mediadesk/config.json").exists());
    let state_raw = std::fs::read_to_string(server.state_dir.path().join("state.json")).unwrap();
    assert!(state_raw.contains("lastWorkspace"));

    let (status, other) = server
        .call(
            "POST",
            "/api/workspace",
            Some(json!({"path": server.ws().to_string_lossy()})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(other["ideas"], 1);
}

#[tokio::test]
async fn skill_and_preset_crud() {
    let server = TestServer::spawn().await;
    let (status, skill) = server
        .call(
            "POST",
            "/api/skills",
            Some(json!({
                "name": "测试技能ABC",
                "description": "描述",
                "appliesTo": ["article", "unknown"],
                "content": "# 方法\n写具体一点。"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{skill}");
    let skill_id = skill["id"].as_str().unwrap().to_string();
    assert_eq!(skill["appliesTo"].as_array().unwrap().len(), 1);

    let (status, updated) = server
        .call(
            "PUT",
            &format!("/api/skills/{skill_id}"),
            Some(json!({"name": "测试技能改名", "description": "新描述", "appliesTo": ["script"], "content": "新内容"})),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(updated["name"], "测试技能改名");

    let (status, preset) = server
        .call(
            "POST",
            "/api/presets",
            Some(json!({
                "name": "测试预设XYZ",
                "description": "预设描述",
                "kind": "script",
                "skillIds": [skill_id.clone()],
                "content": "# 结构\n开头、正文、结尾"
            })),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{preset}");
    let preset_id = preset["id"].as_str().unwrap().to_string();
    assert_eq!(preset["kind"], "script");

    let (status, _) = server
        .call("DELETE", &format!("/api/skills/{skill_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (status, _) = server
        .call("DELETE", &format!("/api/presets/{preset_id}"), None)
        .await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    let (_, skills) = server.call("GET", "/api/skills", None).await;
    assert_eq!(skills.as_array().unwrap().len(), 7);
    let (_, presets) = server.call("GET", "/api/presets", None).await;
    assert_eq!(presets.as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn linking_an_idea_to_another_project_removes_stale_links() {
    let server = TestServer::spawn().await;
    let idea = server.create_idea("跨项目想法", "只能属于一个项目").await;
    let idea_id = idea["id"].as_str().unwrap().to_string();
    let project_a = server
        .create_project("项目 A", vec![idea["id"].clone()])
        .await;
    let project_b = server
        .create_project("项目 B", vec![idea["id"].clone()])
        .await;
    let (_, detail_a) = server
        .call(
            "GET",
            &format!("/api/projects/{}", project_a["id"].as_str().unwrap()),
            None,
        )
        .await;
    assert!(detail_a["ideaIds"].as_array().unwrap().is_empty());
    let (_, detail_b) = server
        .call(
            "GET",
            &format!("/api/projects/{}", project_b["id"].as_str().unwrap()),
            None,
        )
        .await;
    assert_eq!(detail_b["ideaIds"].as_array().unwrap().len(), 1);
    let (_, refreshed_idea) = server
        .call("GET", &format!("/api/ideas/{idea_id}"), None)
        .await;
    assert_eq!(
        refreshed_idea["projectId"], project_b["id"],
        "idea should point at the newest project"
    );
}
