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

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMeta {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub applies_to: Vec<String>,
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

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfig {
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
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: default_base_url(),
            api_key: String::new(),
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
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
pub struct Config {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub appearance: Appearance,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            llm: LlmConfig::default(),
            appearance: Appearance::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigView {
    pub base_url: String,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
    pub has_api_key: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigView {
    pub version: u32,
    pub llm: LlmConfigView,
    pub appearance: Appearance,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LlmConfigUpdate {
    pub base_url: String,
    /// Omit to keep the stored key; empty string clears it.
    pub api_key: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub timeout_secs: u64,
}

impl From<&LlmConfig> for LlmConfigView {
    fn from(c: &LlmConfig) -> Self {
        Self {
            base_url: c.base_url.clone(),
            model: c.model.clone(),
            temperature: c.temperature,
            max_tokens: c.max_tokens,
            timeout_secs: c.timeout_secs,
            has_api_key: !c.api_key.trim().is_empty(),
        }
    }
}

impl From<&Config> for ConfigView {
    fn from(c: &Config) -> Self {
        Self {
            version: c.version,
            llm: LlmConfigView::from(&c.llm),
            appearance: c.appearance.clone(),
        }
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

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestLlmRequest {
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub timeout_secs: Option<u64>,
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
