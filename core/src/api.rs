use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, Request, State},
    http::{header, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post, put},
    Json, Router,
};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tower::ServiceExt;
use tower_http::services::ServeFile;

use crate::assets;
use crate::assistant;
use crate::card_gen;
use crate::cli;
use crate::context;
use crate::error::{ApiError, ApiResult};
use crate::library;
use crate::library_assistant;
use crate::llm;
use crate::models::*;
use crate::providers;
use crate::store;
use crate::ui;

pub struct AppState {
    workspace: RwLock<PathBuf>,
    pub state_dir: PathBuf,
    pub code_bin: Option<String>,
    pub client: reqwest::Client,
}

impl AppState {
    pub async fn bootstrap(
        workspace_override: Option<PathBuf>,
        state_dir_override: Option<PathBuf>,
    ) -> ApiResult<Arc<Self>> {
        let state_dir = state_dir_override
            .or_else(|| std::env::var_os("MEDIADESK_STATE_DIR").map(PathBuf::from))
            .unwrap_or_else(store::default_state_dir);
        let workspace = if let Some(ws) = workspace_override {
            ws
        } else if let Some(ws) = std::env::var_os("MEDIADESK_WORKSPACE").map(PathBuf::from) {
            ws
        } else if let Some(ws) = store::last_workspace(&state_dir).await {
            ws
        } else {
            store::default_workspace()
        };
        let workspace = store::init_workspace(&workspace).await?;
        if let Err(err) = store::remember_workspace(&state_dir, &workspace).await {
            eprintln!("warning: could not persist last workspace: {err}");
        }
        let client = reqwest::Client::builder()
            .user_agent(format!("MediaDesk/{}", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(15))
            .build()
            .map_err(|e| ApiError::internal(format!("初始化 HTTP 客户端失败：{e}")))?;
        let code_bin = std::env::var("MEDIADESK_CODE_BIN")
            .ok()
            .filter(|s| !s.trim().is_empty());
        Ok(Arc::new(Self {
            workspace: RwLock::new(workspace),
            state_dir,
            code_bin,
            client,
        }))
    }

    pub fn workspace(&self) -> PathBuf {
        self.workspace
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_else(|_| store::default_workspace())
    }

    pub async fn switch_workspace(&self, path: &str) -> ApiResult<PathBuf> {
        let workspace = store::init_workspace(&store::expand_path(path)).await?;
        {
            let mut guard = self
                .workspace
                .write()
                .map_err(|_| ApiError::internal("工作区状态锁被污染"))?;
            *guard = workspace.clone();
        }
        if let Err(err) = store::remember_workspace(&self.state_dir, &workspace).await {
            eprintln!("warning: could not persist last workspace: {err}");
        }
        Ok(workspace)
    }
}

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/workspace", get(get_workspace).post(switch_workspace))
        .route("/api/workspace/reveal", post(reveal_workspace))
        .route("/api/fs/browse", get(browse))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/config/kernel", put(put_kernel_config))
        .route("/api/config/media", put(put_media_config))
        .route("/api/cli/status", get(cli_status))
        .route("/api/llm/providers", get(llm_providers))
        .route("/api/llm/models", post(llm_models))
        .route("/api/llm/test", post(test_llm))
        .route("/api/llm/prompt", post(preview_prompt))
        .route("/api/llm/generate", post(generate))
        .route(
            "/api/library/assistant",
            get(get_library_assistant_history).delete(clear_library_assistant_history),
        )
        .route(
            "/api/library/assistant/stream",
            post(stream_library_assistant),
        )
        .route("/api/skills", get(list_skills).post(create_skill))
        .route("/api/skills/{id}", put(update_skill).delete(delete_skill))
        .route("/api/presets", get(list_presets).post(create_preset))
        .route(
            "/api/presets/{id}",
            put(update_preset).delete(delete_preset),
        )
        .route("/api/ideas", get(list_ideas).post(create_idea))
        .route(
            "/api/ideas/{id}",
            get(get_idea).put(update_idea).delete(delete_idea),
        )
        .route("/api/ideas/{id}/promote", post(promote_idea))
        .route("/api/projects", get(list_projects).post(create_project))
        .route(
            "/api/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route("/api/projects/{id}/rescan", post(rescan_project))
        .route("/api/projects/{id}/vscode", post(open_vscode))
        .route("/api/projects/{id}/context", post(prepare_context))
        .route("/api/projects/{id}/assets", post(create_asset))
        .route(
            "/api/projects/{id}/assets/{asset_id}",
            get(get_asset).put(update_asset).delete(delete_asset),
        )
        .route("/api/projects/{id}/uploads", post(upload_asset))
        .route(
            "/api/projects/{id}/chat",
            get(get_assistant_history)
                .post(chat_with_assistant)
                .delete(clear_assistant_history),
        )
        .route("/api/projects/{id}/chat/stream", post(stream_assistant))
        .route(
            "/api/projects/{id}/chat/apply",
            post(apply_assistant_suggestion),
        )
        .route("/api/projects/{id}/card/plan", post(generate_card_plan))
        .route("/api/search", get(search))
        .route("/files/{*path}", get(serve_file))
        .fallback(ui::fallback)
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(state)
}

// ---------------------------------------------------------------------------
// Workspace / health
// ---------------------------------------------------------------------------

async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "ok": true,
        "name": "媒材工坊 MediaDesk",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn get_workspace(State(state): State<Arc<AppState>>) -> ApiResult<Json<WorkspaceInfo>> {
    let info = store::workspace_info(&state.workspace()).await?;
    Ok(Json(info))
}

async fn switch_workspace(
    State(state): State<Arc<AppState>>,
    Json(req): Json<WorkspaceSwitch>,
) -> ApiResult<Json<WorkspaceInfo>> {
    let workspace = state.switch_workspace(&req.path).await?;
    Ok(Json(store::workspace_info(&workspace).await?))
}

async fn reveal_workspace(State(state): State<Arc<AppState>>) -> ApiResult<Json<SimpleOk>> {
    let path = state.workspace();
    let (program, args): (&str, Vec<String>) = if cfg!(target_os = "macos") {
        ("open", vec![path.to_string_lossy().to_string()])
    } else if cfg!(windows) {
        ("explorer", vec![path.to_string_lossy().to_string()])
    } else {
        ("xdg-open", vec![path.to_string_lossy().to_string()])
    };
    std::process::Command::new(program)
        .args(args)
        .spawn()
        .map_err(|e| ApiError::internal(format!("无法打开文件管理器：{e}")))?;
    Ok(Json(SimpleOk {
        ok: true,
        message: Some(path.to_string_lossy().to_string()),
    }))
}

#[derive(Debug, Deserialize)]
struct BrowseQuery {
    path: Option<String>,
}

async fn browse(Query(query): Query<BrowseQuery>) -> ApiResult<Json<BrowseInfo>> {
    Ok(Json(store::browse_dirs(query.path.as_deref()).await?))
}

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

async fn get_config(State(state): State<Arc<AppState>>) -> ApiResult<Json<ConfigView>> {
    let config = store::load_config(&state.workspace()).await?;
    Ok(Json(ConfigView::from(&config)))
}

fn validate_base_url(url: &str) -> ApiResult<()> {
    let trimmed = url.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(ApiError::bad_request(
            "Base URL 必须以 http:// 或 https:// 开头",
        ));
    }
    if reqwest::Url::parse(trimmed).is_err() {
        return Err(ApiError::bad_request("Base URL 格式不正确"));
    }
    Ok(())
}

async fn cli_status(State(state): State<Arc<AppState>>) -> ApiResult<Json<KernelStatus>> {
    let config = store::load_config(&state.workspace()).await?;
    Ok(Json(cli::status(&config)))
}

fn validate_cli_args(args: &[String]) -> ApiResult<()> {
    if args.len() > 20 {
        return Err(ApiError::bad_request("CLI 参数最多 20 个"));
    }
    for arg in args {
        if arg.chars().count() > 4000 || arg.contains('\0') {
            return Err(ApiError::bad_request("CLI 参数过长或包含非法字符"));
        }
    }
    Ok(())
}

async fn put_kernel_config(
    State(state): State<Arc<AppState>>,
    Json(update): Json<TextKernelUpdate>,
) -> ApiResult<Json<ConfigView>> {
    let kernel = update.kernel.trim();
    if !["api", "claude", "codex", "opencode", "deepseek"].contains(&kernel) {
        return Err(ApiError::bad_request("不支持的文本内核"));
    }
    if !(10..=1800).contains(&update.timeout_secs) {
        return Err(ApiError::bad_request("CLI 超时时间必须在 10–1800 秒之间"));
    }
    validate_cli_args(&update.args)?;
    let workspace = state.workspace();
    let mut config = store::load_config(&workspace).await?;
    config.text_kernel = kernel.to_string();
    config.cli.binary = update
        .binary
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_string);
    config.cli.args = update.args;
    config.cli.timeout_secs = update.timeout_secs;
    store::save_config(&workspace, &config).await?;
    Ok(Json(ConfigView::from(&config)))
}

fn apply_media_profile(
    profile: &mut MediaProfile,
    update: MediaProfileUpdate,
    kind: &str,
) -> ApiResult<()> {
    if !update.base_url.trim().is_empty() {
        validate_base_url(&update.base_url)?;
    }
    let protocol = update.protocol.trim().to_ascii_lowercase();
    let allowed = if kind == "video" {
        ["openai", "task", "seedance"].as_slice()
    } else {
        ["openai"].as_slice()
    };
    if !allowed.contains(&protocol.as_str()) {
        return Err(ApiError::bad_request(format!(
            "{}协议不支持：{}",
            if kind == "video" { "视频" } else { "图像" },
            update.protocol
        )));
    }
    for path in [&update.create_path, &update.status_path] {
        if path.chars().any(char::is_control) {
            return Err(ApiError::bad_request("媒体接口路径不能包含控制字符"));
        }
    }
    profile.base_url = update.base_url.trim().trim_end_matches('/').to_string();
    profile.model = update.model.trim().to_string();
    profile.protocol = protocol;
    profile.create_path = update.create_path.trim().to_string();
    profile.status_path = update.status_path.trim().to_string();
    profile.style = update.style.trim().to_string();
    if let Some(key) = update.api_key {
        profile.api_key = key.trim().to_string();
    }
    Ok(())
}

async fn put_media_config(
    State(state): State<Arc<AppState>>,
    Json(update): Json<MediaGenUpdate>,
) -> ApiResult<Json<ConfigView>> {
    if !(10..=1800).contains(&update.timeout_secs) {
        return Err(ApiError::bad_request("媒体请求超时必须在 10–1800 秒之间"));
    }
    if !(1..=60).contains(&update.poll_interval_secs) {
        return Err(ApiError::bad_request("轮询间隔必须在 1–60 秒之间"));
    }
    if !(30..=3600).contains(&update.video_timeout_secs) {
        return Err(ApiError::bad_request("视频任务超时必须在 30–3600 秒之间"));
    }
    let workspace = state.workspace();
    let mut config = store::load_config(&workspace).await?;
    apply_media_profile(
        &mut config.media_gen.illustration,
        update.illustration,
        "illustration",
    )?;
    apply_media_profile(&mut config.media_gen.video, update.video, "video")?;
    config.media_gen.timeout_secs = update.timeout_secs;
    config.media_gen.poll_interval_secs = update.poll_interval_secs;
    config.media_gen.video_timeout_secs = update.video_timeout_secs;
    store::save_config(&workspace, &config).await?;
    Ok(Json(ConfigView::from(&config)))
}

async fn put_config(
    State(state): State<Arc<AppState>>,
    Json(update): Json<LlmConfigUpdate>,
) -> ApiResult<Json<ConfigView>> {
    let workspace = state.workspace();
    let mut config = store::load_config(&workspace).await?;
    validate_base_url(&update.base_url)?;
    let previous_base = config.llm.base_url.clone();
    let previous_provider = config.llm.provider_id.clone();
    if let Some(provider_id) = update
        .provider_id
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if providers::get(provider_id).is_none() {
            return Err(ApiError::bad_request(format!(
                "未知的服务商：{provider_id}"
            )));
        }
        config.llm.provider_id = provider_id.to_string();
    }
    if let Some(protocol) = update
        .protocol
        .as_deref()
        .map(str::trim)
        .filter(|p| !p.is_empty())
    {
        if providers::Protocol::from_id(protocol).is_none() {
            return Err(ApiError::bad_request(
                "协议必须是 openai / anthropic / responses",
            ));
        }
        config.llm.protocol = protocol.to_ascii_lowercase();
    }
    config.llm.custom_headers = providers::validate_custom_headers(&update.custom_headers)?;
    if update.model.trim().is_empty() {
        return Err(ApiError::bad_request("模型名称不能为空"));
    }
    if !(0.0..=2.0).contains(&update.temperature) {
        return Err(ApiError::bad_request("temperature 必须在 0–2 之间"));
    }
    if !(16..=32768).contains(&update.max_tokens) {
        return Err(ApiError::bad_request("maxTokens 必须在 16–32768 之间"));
    }
    if !(5..=600).contains(&update.timeout_secs) {
        return Err(ApiError::bad_request("超时时间必须在 5–600 秒之间"));
    }
    config.llm.base_url = update.base_url.trim().trim_end_matches('/').to_string();
    if update.api_key.is_none()
        && (config.llm.provider_id != previous_provider
            || config.llm.base_url.trim_end_matches('/') != previous_base.trim_end_matches('/'))
    {
        config.llm.api_key.clear();
    }
    config.llm.model = update.model.trim().to_string();
    config.llm.temperature = update.temperature;
    config.llm.max_tokens = update.max_tokens;
    config.llm.timeout_secs = update.timeout_secs;
    if let Some(key) = update.api_key {
        config.llm.api_key = key.trim().to_string();
    }
    store::save_config(&workspace, &config).await?;
    Ok(Json(ConfigView::from(&config)))
}

// ---------------------------------------------------------------------------
// Skills / presets
// ---------------------------------------------------------------------------

async fn list_skills(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Skill>>> {
    Ok(Json(library::list_skills(&state.workspace()).await?))
}

async fn create_skill(
    State(state): State<Arc<AppState>>,
    Json(input): Json<SkillInput>,
) -> ApiResult<Json<Skill>> {
    Ok(Json(
        library::create_skill(&state.workspace(), input).await?,
    ))
}

async fn update_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<SkillInput>,
) -> ApiResult<Json<Skill>> {
    Ok(Json(
        library::update_skill(&state.workspace(), &id, input).await?,
    ))
}

async fn delete_skill(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    library::delete_skill(&state.workspace(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_presets(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Preset>>> {
    Ok(Json(library::list_presets(&state.workspace()).await?))
}

async fn create_preset(
    State(state): State<Arc<AppState>>,
    Json(input): Json<PresetInput>,
) -> ApiResult<Json<Preset>> {
    Ok(Json(
        library::create_preset(&state.workspace(), input).await?,
    ))
}

async fn update_preset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(input): Json<PresetInput>,
) -> ApiResult<Json<Preset>> {
    Ok(Json(
        library::update_preset(&state.workspace(), &id, input).await?,
    ))
}

async fn delete_preset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    library::delete_preset(&state.workspace(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Ideas
// ---------------------------------------------------------------------------

async fn list_ideas(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<Idea>>> {
    Ok(Json(store::list_ideas(&state.workspace()).await?))
}

async fn create_idea(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateIdea>,
) -> ApiResult<Json<Idea>> {
    let idea = store::create_idea(&state.workspace(), &req.title, &req.content, req.tags).await?;
    Ok(Json(idea))
}

async fn get_idea(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<Idea>> {
    Ok(Json(store::get_idea(&state.workspace(), &id).await?))
}

async fn update_idea(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<UpdateIdea>,
) -> ApiResult<Json<Idea>> {
    Ok(Json(
        store::update_idea(&state.workspace(), &id, patch).await?,
    ))
}

async fn delete_idea(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    store::delete_idea(&state.workspace(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn promote_idea(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<PromoteIdea>,
) -> ApiResult<Json<Project>> {
    Ok(Json(
        store::promote_idea(&state.workspace(), &id, req.title, req.brief).await?,
    ))
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

async fn list_projects(State(state): State<Arc<AppState>>) -> ApiResult<Json<Vec<ProjectSummary>>> {
    Ok(Json(store::project_summaries(&state.workspace()).await?))
}

async fn create_project(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProject>,
) -> ApiResult<Json<Project>> {
    Ok(Json(store::create_project(&state.workspace(), req).await?))
}

async fn get_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<Project>> {
    Ok(Json(store::project_detail(&state.workspace(), &id).await?))
}

async fn update_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(patch): Json<UpdateProject>,
) -> ApiResult<Json<Project>> {
    Ok(Json(
        store::update_project(&state.workspace(), &id, patch).await?,
    ))
}

async fn delete_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    store::delete_project(&state.workspace(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn rescan_project(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<Project>> {
    Ok(Json(store::project_detail(&state.workspace(), &id).await?))
}

async fn open_vscode(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<VscodeResult>> {
    Ok(Json(
        context::launch(&state.workspace(), &id, state.code_bin.as_deref()).await?,
    ))
}

async fn prepare_context(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<ContextSummary>> {
    Ok(Json(context::prepare_only(&state.workspace(), &id).await?))
}

// ---------------------------------------------------------------------------
// Assets
// ---------------------------------------------------------------------------

async fn create_asset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CreateAsset>,
) -> ApiResult<Json<Asset>> {
    Ok(Json(
        assets::create_asset(&state.workspace(), &id, req).await?,
    ))
}

async fn get_asset(
    State(state): State<Arc<AppState>>,
    Path((id, asset_id)): Path<(String, String)>,
) -> ApiResult<Json<Asset>> {
    Ok(Json(
        assets::get_asset(&state.workspace(), &id, &asset_id).await?,
    ))
}

async fn update_asset(
    State(state): State<Arc<AppState>>,
    Path((id, asset_id)): Path<(String, String)>,
    Json(patch): Json<UpdateAsset>,
) -> ApiResult<Json<Asset>> {
    Ok(Json(
        assets::update_asset(&state.workspace(), &id, &asset_id, patch).await?,
    ))
}

async fn delete_asset(
    State(state): State<Arc<AppState>>,
    Path((id, asset_id)): Path<(String, String)>,
) -> ApiResult<StatusCode> {
    assets::delete_asset(&state.workspace(), &id, &asset_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadQuery {
    kind: String,
    asset_id: Option<String>,
    filename: String,
    title: Option<String>,
    group_id: Option<String>,
    group_title: Option<String>,
    group_index: Option<usize>,
    group_total: Option<usize>,
}

const MAX_UPLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

struct TempGuard(PathBuf);
impl Drop for TempGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn upload_asset(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(query): Query<UploadQuery>,
    req: Request,
) -> ApiResult<Response> {
    if let Some(length) = req
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        if length > MAX_UPLOAD_BYTES {
            return Err(ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "UPLOAD_TOO_LARGE",
                "文件超过 2GB 上限",
            ));
        }
    }
    let plan = assets::plan_upload(
        &state.workspace(),
        &id,
        &query.kind,
        query.asset_id.as_deref(),
        &query.filename,
        query.title.as_deref(),
        Some(assets::UploadCardGroup {
            id: query.group_id,
            title: query.group_title,
            index: query.group_index,
            total: query.group_total,
        }),
    )
    .await?;
    let temp_path = plan.temp_path.clone();
    let _guard = TempGuard(temp_path.clone());
    let mut file = tokio::fs::File::create(&temp_path).await?;
    let mut body = req.into_body();
    let mut total: u64 = 0;
    while let Some(frame) = body.frame().await {
        let frame = frame.map_err(|e| ApiError::bad_request(format!("上传读取失败：{e}")))?;
        if let Ok(data) = frame.into_data() {
            total += data.len() as u64;
            if total > MAX_UPLOAD_BYTES {
                drop(file);
                return Err(ApiError::new(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "UPLOAD_TOO_LARGE",
                    "文件超过 2GB 上限",
                ));
            }
            file.write_all(&data).await?;
        }
    }
    file.flush().await?;
    file.sync_all().await?;
    drop(file);
    if total == 0 {
        return Err(ApiError::bad_request("上传内容为空"));
    }
    let asset = assets::finish_upload(&state.workspace(), plan).await?;
    Ok(Json(asset).into_response())
}

// ---------------------------------------------------------------------------
// Search / LLM / files
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> ApiResult<Json<SearchResult>> {
    Ok(Json(store::search(&state.workspace(), &query.q).await?))
}

async fn preview_prompt(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateRequest>,
) -> ApiResult<Json<PromptResult>> {
    Ok(Json(llm::preview(&state.workspace(), req).await?))
}

async fn generate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GenerateRequest>,
) -> ApiResult<Json<GenerateResult>> {
    Ok(Json(
        llm::generate(&state.workspace(), &state.client, req).await?,
    ))
}

async fn llm_providers() -> Json<Vec<providers::ProviderSpec>> {
    Json(providers::catalog())
}

async fn llm_models(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ModelListRequest>,
) -> ApiResult<Json<ModelListResponse>> {
    let workspace = state.workspace();
    let config = store::load_config(&workspace).await?;
    Ok(Json(
        llm::list_models(&state.client, &config.llm, request).await?,
    ))
}

async fn test_llm(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TestLlmRequest>,
) -> ApiResult<Json<LlmTestResult>> {
    Ok(Json(
        llm::test_connection(&state.workspace(), &state.client, req).await?,
    ))
}

async fn get_assistant_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<AssistantHistory>> {
    Ok(Json(
        assistant::load_history(&state.workspace(), &id).await?,
    ))
}

async fn chat_with_assistant(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AssistantChatRequest>,
) -> ApiResult<Json<AssistantChatResponse>> {
    Ok(Json(
        assistant::chat(&state.workspace(), &state.client, &id, req).await?,
    ))
}

async fn stream_assistant(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AssistantChatRequest>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AssistantStreamEvent>();
    let workspace = state.workspace();
    let client = state.client.clone();
    tokio::spawn(async move {
        assistant::chat_stream(&workspace, &client, &id, req, tx).await;
    });
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|event| {
            let data = serde_json::to_string(&event).unwrap_or_else(|_| {
                "{\"type\":\"error\",\"code\":\"STREAM_ENCODE\",\"message\":\"事件编码失败\"}"
                    .to_string()
            });
            (Ok::<_, Infallible>(Event::default().data(data)), rx)
        })
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn apply_assistant_suggestion(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<AssistantApplyRequest>,
) -> ApiResult<Json<AssistantApplyResponse>> {
    Ok(Json(
        assistant::apply_saved_suggestion(&state.workspace(), &id, &req.message_id).await?,
    ))
}

async fn generate_card_plan(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<CardPlanRequest>,
) -> ApiResult<Json<CardPlanResponse>> {
    Ok(Json(
        card_gen::generate_plan(&state.workspace(), &state.client, &id, req).await?,
    ))
}

async fn get_library_assistant_history(
    State(state): State<Arc<AppState>>,
) -> ApiResult<Json<library_assistant::LibraryAssistantHistory>> {
    Ok(Json(
        library_assistant::load_history(&state.workspace()).await?,
    ))
}

async fn clear_library_assistant_history(
    State(state): State<Arc<AppState>>,
) -> ApiResult<StatusCode> {
    library_assistant::clear_history(&state.workspace()).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn stream_library_assistant(
    State(state): State<Arc<AppState>>,
    Json(req): Json<library_assistant::LibraryAssistantRequest>,
) -> Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>> {
    let (tx, rx) =
        tokio::sync::mpsc::unbounded_channel::<library_assistant::LibraryAssistantStreamEvent>();
    let workspace = state.workspace();
    let client = state.client.clone();
    tokio::spawn(async move {
        library_assistant::chat_stream(&workspace, &client, req, tx).await;
    });
    let stream = futures_util::stream::unfold(rx, |mut rx| async move {
        rx.recv().await.map(|event| {
            let data = serde_json::to_string(&event).unwrap_or_else(|_| {
                "{\"type\":\"error\",\"code\":\"STREAM_ENCODE\",\"message\":\"事件编码失败\"}"
                    .to_string()
            });
            (Ok::<_, Infallible>(Event::default().data(data)), rx)
        })
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn clear_assistant_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    assistant::clear_history(&state.workspace(), &id).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn serve_file(
    State(state): State<Arc<AppState>>,
    Path(path): Path<String>,
    req: Request,
) -> Response {
    let rel = path.trim_start_matches('/');
    let workspace = state.workspace();
    let full = match store::resolve_public_file(&workspace, rel).await {
        Ok(full) => full,
        Err(err) => return err.into_response(),
    };
    match ServeFile::new(full).oneshot(req).await {
        Ok(response) => {
            let (parts, body) = response.into_parts();
            (parts, Body::new(body)).into_response()
        }
        Err(err) => ApiError::internal(format!("读取文件失败：{err}")).into_response(),
    }
}
