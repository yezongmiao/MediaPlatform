use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const ASSET_KINDS: [&str; 5] = ["article", "script", "illustration", "card", "video"];

pub fn is_valid_kind(kind: &str) -> bool {
    ASSET_KINDS.contains(&kind)
}

pub fn kind_label(kind: &str) -> &'static str {
    match kind {
        "article" => "文章",
        "script" => "脚本",
        "illustration" => "插图",
        "card" => "文卡图",
        "video" => "视频",
        _ => "素材",
    }
}

fn default_idea_status() -> String {
    "inbox".into()
}

fn default_project_status() -> String {
    "active".into()
}

fn default_asset_status() -> String {
    "draft".into()
}

fn default_version() -> u32 {
    1
}

fn default_provider_id() -> String {
    "deepseek".into()
}

fn default_protocol() -> String {
    "openai".into()
}

fn default_text_kernel() -> String {
    "api".into()
}

fn default_cli_timeout() -> u64 {
    240
}

fn default_media_timeout() -> u64 {
    300
}

fn default_poll_interval() -> u64 {
    5
}

fn default_video_timeout() -> u64 {
    600
}

fn default_media_protocol() -> String {
    "openai".into()
}

fn default_base_url() -> String {
    "https://api.deepseek.com/v1".into()
}

fn default_model() -> String {
    "deepseek-chat".into()
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> u32 {
    4096
}

fn default_timeout_secs() -> u64 {
    120
}

fn default_theme() -> String {
    "light".into()
}

// ---------------------------------------------------------------------------
// Ideas
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdeaMeta {
    pub id: String,
    pub title: String,
    #[serde(default = "default_idea_status")]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Idea {
    #[serde(flatten)]
    pub meta: IdeaMeta,
    pub content: String,
    pub excerpt: String,
    pub rel_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIdea {
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIdea {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub status: Option<String>,
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMeta {
    pub id: String,
    pub title: String,
    #[serde(default = "default_project_status")]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub idea_ids: Vec<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub preset_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    #[serde(flatten)]
    pub meta: ProjectMeta,
    pub brief_excerpt: String,
    pub dir_name: String,
    pub counts: BTreeMap<String, usize>,
    pub cover_url: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    #[serde(flatten)]
    pub meta: ProjectMeta,
    pub brief: String,
    pub dir_name: String,
    pub assets: BTreeMap<String, Vec<Asset>>,
    pub counts: BTreeMap<String, usize>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProject {
    pub title: String,
    #[serde(default)]
    pub brief: String,
    #[serde(default)]
    pub idea_ids: Vec<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub preset_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProject {
    pub title: Option<String>,
    pub brief: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
    pub idea_ids: Option<Vec<String>>,
    pub skill_ids: Option<Vec<String>>,
    pub preset_ids: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromoteIdea {
    pub title: Option<String>,
    pub brief: Option<String>,
}

// ---------------------------------------------------------------------------
// Assets
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetMeta {
    pub id: String,
    pub kind: String,
    pub title: String,
    #[serde(default = "default_asset_status")]
    pub status: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// AI 文卡图一次生成的同组 PNG 共享这些字段；普通卡片为空。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_group_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_group_index: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_group_total: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    #[serde(flatten)]
    pub meta: AssetMeta,
    pub content: String,
    pub file_url: Option<String>,
    pub rel_path: String,
    pub project_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAsset {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAsset {
    pub title: Option<String>,
    pub content: Option<String>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
    pub prompt: Option<String>,
}

// ---------------------------------------------------------------------------
// Skills & presets
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SkillCode {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub code: Vec<SkillCode>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    #[serde(flatten)]
    pub meta: SkillMeta,
    pub content: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub kind: String,
    #[serde(default)]
    pub skill_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    #[serde(flatten)]
    pub meta: PresetMeta,
    pub content: String,
}

// ---------------------------------------------------------------------------
// Config & workspace
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RequestHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
    #[serde(default = "default_provider_id")]
    pub provider_id: String,
    #[serde(default = "default_base_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    #[serde(default)]
    pub custom_headers: Vec<RequestHeader>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider_id: default_provider_id(),
            base_url: default_base_url(),
            api_key: String::new(),
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            protocol: default_protocol(),
            custom_headers: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    #[serde(default = "default_theme")]
    pub theme: String,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: default_theme(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CliConfig {
    /// Optional absolute path. Empty means auto-detect from PATH.
    #[serde(default)]
    pub binary: Option<String>,
    /// Extra CLI arguments. `{prompt}` is substituted if present; otherwise stdin receives the prompt.
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default = "default_cli_timeout")]
    pub timeout_secs: u64,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            binary: None,
            args: Vec::new(),
            timeout_secs: default_cli_timeout(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProfile {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub model: String,
    /// `openai` for /images/generations or /videos; `task` for a generic async task endpoint.
    #[serde(default = "default_media_protocol")]
    pub protocol: String,
    /// Optional create path, e.g. `/contents/generations/tasks`; defaults per kind.
    #[serde(default)]
    pub create_path: String,
    /// Task polling path; `{id}` is substituted. Empty means create_path + `/{id}`.
    #[serde(default)]
    pub status_path: String,
    #[serde(default)]
    pub style: String,
}

impl Default for MediaProfile {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            api_key: String::new(),
            model: String::new(),
            protocol: default_media_protocol(),
            create_path: String::new(),
            status_path: String::new(),
            style: String::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaGenConfig {
    #[serde(default)]
    pub illustration: MediaProfile,
    #[serde(default)]
    pub video: MediaProfile,
    #[serde(default = "default_media_timeout")]
    pub timeout_secs: u64,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_video_timeout")]
    pub video_timeout_secs: u64,
}

impl Default for MediaGenConfig {
    fn default() -> Self {
        Self {
            illustration: MediaProfile::default(),
            video: MediaProfile::default(),
            timeout_secs: default_media_timeout(),
            poll_interval_secs: default_poll_interval(),
            video_timeout_secs: default_video_timeout(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProfileView {
    pub base_url: String,
    pub model: String,
    pub protocol: String,
    pub create_path: String,
    pub status_path: String,
    pub style: String,
    pub has_api_key: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaGenView {
    pub illustration: MediaProfileView,
    pub video: MediaProfileView,
    pub timeout_secs: u64,
    pub poll_interval_secs: u64,
    pub video_timeout_secs: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaProfileUpdate {
    pub base_url: String,
    /// Omit to keep the stored key; empty string clears it.
    pub api_key: Option<String>,
    pub model: String,
    pub protocol: String,
    #[serde(default)]
    pub create_path: String,
    #[serde(default)]
    pub status_path: String,
    #[serde(default)]
    pub style: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaGenUpdate {
    pub illustration: MediaProfileUpdate,
    pub video: MediaProfileUpdate,
    pub timeout_secs: u64,
    pub poll_interval_secs: u64,
    pub video_timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelInfo {
    pub id: String,
    pub label: String,
    pub found: bool,
    pub path: Option<String>,
    pub install_hint: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelStatus {
    pub selected: String,
    pub env_override: bool,
    pub kernels: Vec<KernelInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextKernelUpdate {
    pub kernel: String,
    #[serde(default)]
    pub binary: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub timeout_secs: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub appearance: Appearance,
    #[serde(default = "default_text_kernel")]
    pub text_kernel: String,
    #[serde(default)]
    pub cli: CliConfig,
    #[serde(default)]
    pub media_gen: MediaGenConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            llm: LlmConfig::default(),
            appearance: Appearance::default(),
            text_kernel: default_text_kernel(),
            cli: CliConfig::default(),
            media_gen: MediaGenConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigView {
    pub provider_id: String,
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub has_api_key: bool,
    pub protocol: String,
    pub custom_headers: Vec<RequestHeader>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigView {
    pub version: u32,
    pub llm: LlmConfigView,
    pub appearance: Appearance,
    pub text_kernel: String,
    pub cli: CliConfig,
    pub media_gen: MediaGenView,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigUpdate {
    #[serde(default)]
    pub provider_id: Option<String>,
    pub base_url: String,
    /// Omit to keep the stored key; empty string clears it.
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    #[serde(default)]
    pub protocol: Option<String>,
    #[serde(default)]
    pub custom_headers: Vec<RequestHeader>,
}

impl From<&LlmConfig> for LlmConfigView {
    fn from(c: &LlmConfig) -> Self {
        Self {
            provider_id: c.provider_id.clone(),
            base_url: c.base_url.clone(),
            model: c.model.clone(),
            temperature: c.temperature,
            max_tokens: c.max_tokens,
            timeout_secs: c.timeout_secs,
            has_api_key: !c.api_key.trim().is_empty(),
            protocol: c.protocol.clone(),
            custom_headers: c.custom_headers.clone(),
        }
    }
}

impl From<&Config> for ConfigView {
    fn from(c: &Config) -> Self {
        Self {
            version: c.version,
            llm: LlmConfigView::from(&c.llm),
            appearance: c.appearance.clone(),
            text_kernel: c.text_kernel.clone(),
            cli: c.cli.clone(),
            media_gen: MediaGenView {
                illustration: media_profile_view(&c.media_gen.illustration),
                video: media_profile_view(&c.media_gen.video),
                timeout_secs: c.media_gen.timeout_secs,
                poll_interval_secs: c.media_gen.poll_interval_secs,
                video_timeout_secs: c.media_gen.video_timeout_secs,
            },
        }
    }
}

fn media_profile_view(profile: &MediaProfile) -> MediaProfileView {
    MediaProfileView {
        base_url: profile.base_url.clone(),
        model: profile.model.clone(),
        protocol: profile.protocol.clone(),
        create_path: profile.create_path.clone(),
        status_path: profile.status_path.clone(),
        style: profile.style.clone(),
        has_api_key: !profile.api_key.trim().is_empty(),
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceInfo {
    pub path: String,
    pub ideas: usize,
    pub projects: usize,
    pub assets: usize,
    pub bytes: u64,
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSwitch {
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntryInfo {
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowseInfo {
    pub path: String,
    pub parent: Option<String>,
    pub dirs: Vec<DirEntryInfo>,
    pub is_workspace: bool,
}

// ---------------------------------------------------------------------------
// LLM requests / responses
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub project_id: Option<String>,
    #[serde(default)]
    pub idea_ids: Vec<String>,
    pub kind: String,
    pub preset_id: Option<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub instruction: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsedPreset {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptResult {
    pub prompt: String,
    pub system: String,
    pub user: String,
    pub kind: String,
    pub preset: Option<UsedPreset>,
    pub skills: Vec<UsedPreset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateResult {
    pub content: String,
    pub model: String,
    pub usage: Usage,
    pub latency_ms: u64,
    pub prompt: String,
    pub preset_id: Option<String>,
    pub skill_ids: Vec<String>,
    pub snapshot_path: Option<String>,
    /// `api`, `claude`, `opencode`, `deepseek`, or `media`.
    pub kernel: String,
    /// Media generation writes the asset immediately; the UI can open it directly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<Asset>,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CardPlan {
    #[serde(default)]
    pub kicker: String,
    #[serde(default)]
    pub page: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub body: Vec<String>,
    #[serde(default)]
    pub footnote: String,
    #[serde(default)]
    pub source: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPlanRequest {
    #[serde(default)]
    pub source_asset_id: Option<String>,
    #[serde(default)]
    pub instruction: String,
    #[serde(default)]
    pub preset_id: Option<String>,
    #[serde(default)]
    pub skill_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedCard {
    pub plan: CardPlan,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPlanResponse {
    pub cards: Vec<GeneratedCard>,
    pub model: String,
    pub latency_ms: u64,
    pub source_chars: usize,
    pub target_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmTestResult {
    pub ok: bool,
    pub message: String,
    pub latency_ms: u64,
    pub model: String,
}

// ---------------------------------------------------------------------------
// Context / search
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VscodeResult {
    pub ok: bool,
    pub command: String,
    pub project_path: String,
    pub files_written: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderResult {
    pub ok: bool,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DshResult {
    pub ok: bool,
    pub url: String,
    pub workspace: String,
    pub command: String,
    pub files_written: Vec<String>,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchAsset {
    pub project_id: String,
    pub project_title: String,
    pub asset: Asset,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub query: String,
    pub ideas: Vec<Idea>,
    pub projects: Vec<ProjectSummary>,
    pub assets: Vec<SearchAsset>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub code: Vec<SkillCode>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub kind: String,
    #[serde(default)]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub content: String,
}

fn default_assistant_mode() -> String {
    "suggest".into()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantEdit {
    pub asset_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantCreate {
    /// Filled by the server after the asset is actually created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AssistantGroup {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub asset_ids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<AssistantEdit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub new_assets: Vec<AssistantCreate>,
    #[serde(default)]
    pub applied: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantHistory {
    pub messages: Vec<AssistantMessage>,
    pub kernel: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantChatRequest {
    pub message: String,
    #[serde(default = "default_assistant_mode")]
    pub mode: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantChatResponse {
    pub reply: String,
    pub edits: Vec<AssistantEdit>,
    pub new_assets: Vec<AssistantCreate>,
    #[serde(default)]
    pub groups: Vec<AssistantGroup>,
    pub applied: bool,
    pub kernel: String,
    pub model: String,
    pub latency_ms: u64,
    pub warnings: Vec<String>,
    pub messages: Vec<AssistantMessage>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantApplyRequest {
    pub message_id: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantApplyResponse {
    pub applied: bool,
    pub warnings: Vec<String>,
    pub messages: Vec<AssistantMessage>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantStatus {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_secs: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum AssistantStreamEvent {
    Status { message: String },
    Delta { text: String },
    Done { response: AssistantChatResponse },
    Error { code: String, message: String },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListResponse {
    pub provider_id: String,
    pub models: Vec<ModelInfo>,
    pub fetched: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs_url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestLlmRequest {
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout_secs: Option<u64>,
    pub protocol: Option<String>,
    pub custom_headers: Option<Vec<RequestHeader>>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListRequest {
    pub provider_id: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub protocol: Option<String>,
    #[serde(default)]
    pub custom_headers: Vec<RequestHeader>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SimpleOk {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VscodeErrorDetails {
    pub project_path: String,
    pub manual_command: String,
    pub files_written: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextSummary {
    pub project_path: String,
    pub files_written: Vec<String>,
}
