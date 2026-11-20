use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use chrono::{SecondsFormat, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde_json::json;
use tokio::fs;
use ulid::Ulid;

use crate::assets;
use crate::error::{ApiError, ApiResult};
use crate::frontmatter;
use crate::models::*;
use crate::seed;

pub const MAX_TITLE_CHARS: usize = 200;
pub const CONFIG_DIR: &str = ".mediadesk";
pub const TRASH_DIR: &str = ".mediadesk/trash";

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
}

pub fn new_id() -> String {
    Ulid::generate().to_string()
}

pub fn excerpt(text: &str, max_chars: usize) -> String {
    let cleaned: String = text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    let mut out: String = cleaned.chars().take(max_chars).collect();
    if cleaned.chars().count() > max_chars {
        out.push('…');
    }
    out
}

pub fn validate_title(title: &str) -> ApiResult<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(ApiError::bad_request("标题不能为空"));
    }
    if title.chars().count() > MAX_TITLE_CHARS {
        return Err(ApiError::bad_request(format!(
            "标题不能超过 {MAX_TITLE_CHARS} 个字符"
        )));
    }
    Ok(title.to_string())
}

pub fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').trim().to_string();
        if tag.is_empty() || tag.chars().count() > 24 {
            continue;
        }
        let key = tag.to_lowercase();
        if seen.insert(key) {
            out.push(tag);
        }
        if out.len() >= 8 {
            break;
        }
    }
    out
}

pub fn validate_status(status: &str, allowed: &[&str]) -> ApiResult<()> {
    if allowed.contains(&status) {
        Ok(())
    } else {
        Err(ApiError::bad_request(format!(
            "状态 `{status}` 不合法，允许值：{}",
            allowed.join(", ")
        )))
    }
}

pub fn slugify(input: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in input.trim().chars().take(64) {
        if ch.is_alphanumeric() {
            for c in ch.to_lowercase() {
                out.push(c);
            }
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let mut slug: String = out.trim_matches('-').chars().take(40).collect();
    slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        slug = "未命名".to_string();
    }
    let upper = slug.to_uppercase();
    let reserved = ["CON", "PRN", "AUX", "NUL"];
    if reserved.contains(&upper.as_str())
        || (upper.len() == 4
            && (upper.starts_with("COM") || upper.starts_with("LPT"))
            && upper[3..4].chars().all(|c| c.is_ascii_digit()))
    {
        slug = format!("_{slug}");
    }
    slug.trim_end_matches(['.', ' ']).to_string()
}

pub fn sanitize_file_stem(input: &str) -> String {
    let without_ext = Path::new(input)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "素材".to_string());
    let mut cleaned: String = without_ext
        .chars()
        .take(60)
        .map(|c| {
            if c.is_control() || "\\/:*?\"<>|".contains(c) {
                '-'
            } else {
                c
            }
        })
        .collect();
    cleaned = cleaned.trim().trim_matches('.').to_string();
    if cleaned.is_empty() {
        "素材".to_string()
    } else {
        cleaned
    }
}

// ---------------------------------------------------------------------------
// Filesystem primitives
// ---------------------------------------------------------------------------

pub async fn atomic_write(path: &Path, bytes: &[u8]) -> ApiResult<()> {
    use tokio::io::AsyncWriteExt;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".into());
    let tmp = path.with_file_name(format!(".{file_name}.tmp-{}", Ulid::generate()));
    {
        let mut f = fs::File::create(&tmp).await?;
        f.write_all(bytes).await?;
        f.sync_all().await?;
    }
    #[cfg(windows)]
    {
        if path.exists() {
            let _ = fs::remove_file(path).await;
        }
    }
    if let Err(err) = fs::rename(&tmp, path).await {
        let _ = fs::remove_file(&tmp).await;
        return Err(ApiError::internal(format!(
            "原子写入失败 {}：{err}",
            path.display()
        )));
    }
    Ok(())
}

pub fn validate_rel(rel: &str) -> ApiResult<PathBuf> {
    if rel.trim().is_empty() {
        return Err(ApiError::bad_request("路径不能为空"));
    }
    if rel.contains('\0') || rel.contains('\\') {
        return Err(ApiError::forbidden("路径包含非法字符"));
    }
    let path = Path::new(rel);
    if path.is_absolute() {
        return Err(ApiError::forbidden("不允许绝对路径"));
    }
    for comp in path.components() {
        match comp {
            Component::Normal(_) => {}
            _ => return Err(ApiError::forbidden("路径包含不允许的片段")),
        }
    }
    Ok(path.to_path_buf())
}

fn ensure_inside(base: &Path, resolved: &Path) -> ApiResult<()> {
    if !resolved.starts_with(base) {
        return Err(ApiError::forbidden("拒绝访问工作区之外的路径"));
    }
    Ok(())
}

pub async fn resolve_existing(base: &Path, rel: &str) -> ApiResult<PathBuf> {
    let rel = validate_rel(rel)?;
    let base = fs::canonicalize(base).await?;
    let joined = base.join(rel);
    let resolved = fs::canonicalize(&joined).await?;
    ensure_inside(&base, &resolved)?;
    Ok(resolved)
}

/// Files exposed to the browser are limited to project media. `.mediadesk`
/// (which contains the API key) and hidden files are never served.
pub async fn resolve_public_file(workspace: &Path, rel: &str) -> ApiResult<PathBuf> {
    let rel_path = validate_rel(rel)?;
    let mut comps = rel_path.components();
    match comps.next() {
        Some(Component::Normal(first)) if first == "projects" => {}
        _ => return Err(ApiError::forbidden("仅允许访问项目素材")),
    }
    for comp in rel_path.components() {
        if let Component::Normal(name) = comp {
            if name.to_string_lossy().starts_with('.') {
                return Err(ApiError::forbidden("不允许访问隐藏文件"));
            }
        }
    }
    let base = fs::canonicalize(workspace).await?;
    let base_projects = fs::canonicalize(base.join("projects")).await?;
    let resolved = fs::canonicalize(base.join(rel_path)).await?;
    ensure_inside(&base_projects, &resolved)?;
    let meta = fs::metadata(&resolved).await?;
    if !meta.is_file() {
        return Err(ApiError::not_found("文件不存在"));
    }
    Ok(resolved)
}

pub fn encode_component(input: &str) -> String {
    utf8_percent_encode(input, NON_ALPHANUMERIC).to_string()
}

pub fn media_url(dir_name: &str, kind: &str, file: &str) -> String {
    format!(
        "/files/projects/{}/{}/{}",
        encode_component(dir_name),
        encode_component(kind),
        encode_component(file)
    )
}

pub fn rel_path(workspace: &Path, path: &Path) -> String {
    path.strip_prefix(workspace)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

// ---------------------------------------------------------------------------
// Workspace + config
// ---------------------------------------------------------------------------

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn default_workspace() -> PathBuf {
    dirs::document_dir()
        .unwrap_or_else(home_dir)
        .join("MediaDesk")
}

pub fn default_state_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(home_dir)
        .join("mediadesk")
}

pub fn expand_path(input: &str) -> PathBuf {
    let trimmed = input.trim();
    if trimmed == "~" {
        return home_dir();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if let Some(rest) = trimmed.strip_prefix("~\\") {
        return home_dir().join(rest);
    }
    PathBuf::from(trimmed)
}

pub async fn remember_workspace(state_dir: &Path, workspace: &Path) -> ApiResult<()> {
    fs::create_dir_all(state_dir).await?;
    let path = state_dir.join("state.json");
    let payload = json!({ "lastWorkspace": workspace.to_string_lossy() });
    atomic_write(
        &path,
        serde_json::to_string_pretty(&payload)
            .map_err(|e| ApiError::internal(e.to_string()))?
            .as_bytes(),
    )
    .await
}

pub async fn last_workspace(state_dir: &Path) -> Option<PathBuf> {
    let path = state_dir.join("state.json");
    let raw = fs::read_to_string(path).await.ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let ws = value.get("lastWorkspace")?.as_str()?.trim();
    if ws.is_empty() {
        None
    } else {
        Some(PathBuf::from(ws))
    }
}

pub async fn init_workspace(path: &Path) -> ApiResult<PathBuf> {
    let expanded = expand_path(&path.to_string_lossy());
    fs::create_dir_all(&expanded).await?;
    let workspace = fs::canonicalize(&expanded).await?;
    for dir in [
        "ideas",
        "projects",
        ".mediadesk",
        ".mediadesk/skills",
        ".mediadesk/presets",
        TRASH_DIR,
    ] {
        fs::create_dir_all(workspace.join(dir)).await?;
    }
    let config_path = workspace.join(CONFIG_DIR).join("config.json");
    if !config_path.exists() {
        let config = Config::default();
        let raw =
            serde_json::to_string_pretty(&config).map_err(|e| ApiError::internal(e.to_string()))?;
        atomic_write(&config_path, raw.as_bytes()).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&config_path, std::fs::Permissions::from_mode(0o600));
        }
    }
    let marker = workspace.join(CONFIG_DIR).join("seed.json");
    if !marker.exists() {
        seed::seed_builtins(&workspace).await?;
    }
    Ok(workspace)
}

pub async fn load_config(workspace: &Path) -> ApiResult<Config> {
    let path = workspace.join(CONFIG_DIR).join("config.json");
    match fs::read_to_string(&path).await {
        Ok(raw) => {
            serde_json::from_str(&raw).map_err(|e| ApiError::internal(format!("配置文件损坏：{e}")))
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(err) => Err(err.into()),
    }
}

pub async fn save_config(workspace: &Path, config: &Config) -> ApiResult<()> {
    let path = workspace.join(CONFIG_DIR).join("config.json");
    let raw =
        serde_json::to_string_pretty(config).map_err(|e| ApiError::internal(e.to_string()))?;
    atomic_write(&path, raw.as_bytes()).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Ideas
// ---------------------------------------------------------------------------

async fn read_idea_at(workspace: &Path, path: &Path) -> ApiResult<Idea> {
    let raw = fs::read_to_string(path).await?;
    let (meta, content) = frontmatter::parse::<IdeaMeta>(&raw)
        .map_err(|e| ApiError::internal(format!("{}：{e}", rel_path(workspace, path))))?;
    Ok(Idea {
        meta,
        excerpt: excerpt(&content, 140),
        content,
        rel_path: rel_path(workspace, path),
    })
}

pub async fn list_ideas(workspace: &Path) -> ApiResult<Vec<Idea>> {
    let dir = workspace.join("ideas");
    let mut ideas = Vec::new();
    let mut entries = match fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(ideas),
        Err(err) => return Err(err.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }
        match read_idea_at(workspace, &path).await {
            Ok(idea) => ideas.push(idea),
            Err(err) => eprintln!("skip invalid idea {}: {err}", path.display()),
        }
    }
    ideas.sort_by(|a, b| {
        b.meta
            .updated_at
            .cmp(&a.meta.updated_at)
            .then_with(|| a.meta.title.cmp(&b.meta.title))
    });
    Ok(ideas)
}

async fn find_idea_file(workspace: &Path, id: &str) -> ApiResult<Option<(PathBuf, Idea)>> {
    let dir = workspace.join("ideas");
    let mut entries = match fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Ok(idea) = read_idea_at(workspace, &path).await {
            if idea.meta.id == id {
                return Ok(Some((path, idea)));
            }
        }
    }
    Ok(None)
}

pub async fn get_idea(workspace: &Path, id: &str) -> ApiResult<Idea> {
    find_idea_file(workspace, id)
        .await?
        .map(|(_, idea)| idea)
        .ok_or_else(|| ApiError::not_found("想法不存在"))
}

pub async fn create_idea(
    workspace: &Path,
    title: &str,
    content: &str,
    tags: Vec<String>,
) -> ApiResult<Idea> {
    let title = validate_title(title)?;
    let id = new_id();
    let meta = IdeaMeta {
        id: id.clone(),
        title,
        status: "inbox".into(),
        tags: normalize_tags(tags),
        created_at: now_iso(),
        updated_at: now_iso(),
        project_id: None,
    };
    let slug = slugify(&meta.title);
    let rel = format!("ideas/{id}-{slug}.md");
    let path = workspace.join(&rel);
    let raw = frontmatter::render(&meta, content)
        .map_err(|e| ApiError::internal(format!("写入想法失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_idea_at(workspace, &path).await
}

pub async fn update_idea(workspace: &Path, id: &str, patch: UpdateIdea) -> ApiResult<Idea> {
    let (path, mut idea) = find_idea_file(workspace, id)
        .await?
        .ok_or_else(|| ApiError::not_found("想法不存在"))?;
    if let Some(title) = patch.title {
        idea.meta.title = validate_title(&title)?;
    }
    if let Some(content) = patch.content {
        idea.content = content;
    }
    if let Some(tags) = patch.tags {
        idea.meta.tags = normalize_tags(tags);
    }
    if let Some(status) = patch.status {
        validate_status(&status, &["inbox", "promoted", "archived"])?;
        idea.meta.status = status;
    }
    idea.meta.updated_at = now_iso();
    let raw = frontmatter::render(&idea.meta, &idea.content)
        .map_err(|e| ApiError::internal(format!("保存想法失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_idea_at(workspace, &path).await
}

pub async fn move_to_trash(workspace: &Path, path: &Path, label: &str) -> ApiResult<PathBuf> {
    let trash = workspace.join(TRASH_DIR);
    fs::create_dir_all(&trash).await?;
    let stamp = Utc::now().format("%Y%m%d-%H%M%S");
    let name = path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".into());
    let target = trash.join(format!("{stamp}-{label}-{name}"));
    fs::rename(path, &target).await?;
    Ok(target)
}

async fn detach_idea_from_other_projects(
    workspace: &Path,
    idea_id: &str,
    keep_project_id: &str,
) -> ApiResult<()> {
    for mut record in list_project_records(workspace).await? {
        if record.meta.id == keep_project_id || !record.meta.idea_ids.iter().any(|id| id == idea_id)
        {
            continue;
        }
        record.meta.idea_ids.retain(|id| id != idea_id);
        record.meta.updated_at = now_iso();
        let raw = frontmatter::render(&record.meta, &record.brief)
            .map_err(|e| ApiError::internal(format!("同步项目失败：{e}")))?;
        atomic_write(&record.dir_path.join("project.md"), raw.as_bytes()).await?;
    }
    Ok(())
}

pub(crate) async fn set_idea_project(
    workspace: &Path,
    idea_id: &str,
    project_id: Option<&str>,
) -> ApiResult<()> {
    if let Some((path, mut idea)) = find_idea_file(workspace, idea_id).await? {
        idea.meta.project_id = project_id.map(str::to_string);
        idea.meta.status = if project_id.is_some() {
            "promoted".into()
        } else {
            "inbox".into()
        };
        idea.meta.updated_at = now_iso();
        let raw = frontmatter::render(&idea.meta, &idea.content)
            .map_err(|e| ApiError::internal(format!("同步想法失败：{e}")))?;
        atomic_write(&path, raw.as_bytes()).await?;
    }
    Ok(())
}

pub async fn delete_idea(workspace: &Path, id: &str) -> ApiResult<()> {
    let (path, idea) = find_idea_file(workspace, id)
        .await?
        .ok_or_else(|| ApiError::not_found("想法不存在"))?;
    if let Some(project_id) = idea.meta.project_id.clone() {
        if let Some(mut project) = find_project_record(workspace, &project_id).await? {
            project.meta.idea_ids.retain(|x| x != id);
            project.meta.updated_at = now_iso();
            let raw = frontmatter::render(&project.meta, &project.brief)
                .map_err(|e| ApiError::internal(e.to_string()))?;
            atomic_write(&project.dir_path.join("project.md"), raw.as_bytes()).await?;
        }
    }
    move_to_trash(workspace, &path, "idea").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ProjectRecord {
    pub dir_name: String,
    pub dir_path: PathBuf,
    pub meta: ProjectMeta,
    pub brief: String,
}

async fn read_project_record(workspace: &Path, dir_path: &Path) -> ApiResult<ProjectRecord> {
    let raw = fs::read_to_string(dir_path.join("project.md")).await?;
    let (meta, brief) = frontmatter::parse::<ProjectMeta>(&raw).map_err(|e| {
        ApiError::internal(format!(
            "{}：{e}",
            rel_path(workspace, &dir_path.join("project.md"))
        ))
    })?;
    Ok(ProjectRecord {
        dir_name: dir_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
        dir_path: dir_path.to_path_buf(),
        meta,
        brief,
    })
}

pub async fn list_project_records(workspace: &Path) -> ApiResult<Vec<ProjectRecord>> {
    let root = workspace.join("projects");
    let mut records = Vec::new();
    let mut entries = match fs::read_dir(&root).await {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(records),
        Err(err) => return Err(err.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path
            .file_name()
            .map(|n| n.to_string_lossy().starts_with('.'))
            .unwrap_or(false)
        {
            continue;
        }
        if !path.join("project.md").exists() {
            continue;
        }
        match read_project_record(workspace, &path).await {
            Ok(record) => records.push(record),
            Err(err) => eprintln!("skip invalid project {}: {err}", path.display()),
        }
    }
    records.sort_by(|a, b| b.meta.updated_at.cmp(&a.meta.updated_at));
    Ok(records)
}

pub async fn find_project_record(workspace: &Path, id: &str) -> ApiResult<Option<ProjectRecord>> {
    for record in list_project_records(workspace).await? {
        if record.meta.id == id {
            return Ok(Some(record));
        }
    }
    Ok(None)
}

pub async fn require_project(workspace: &Path, id: &str) -> ApiResult<ProjectRecord> {
    find_project_record(workspace, id)
        .await?
        .ok_or_else(|| ApiError::not_found("项目不存在"))
}

async fn unique_project_dir(workspace: &Path, base: &str) -> PathBuf {
    let projects = workspace.join("projects");
    let mut name = base.to_string();
    let mut n = 1;
    while projects.join(&name).exists() {
        n += 1;
        name = format!("{base}-{n}");
    }
    projects.join(name)
}

pub async fn create_project(workspace: &Path, req: CreateProject) -> ApiResult<Project> {
    let title = validate_title(&req.title)?;
    let slug = slugify(&title);
    let dir_path = unique_project_dir(workspace, &slug).await;
    for kind in ASSET_KINDS {
        fs::create_dir_all(dir_path.join(kind)).await?;
    }
    fs::create_dir_all(dir_path.join("prompts")).await?;

    let idea_ids = req.idea_ids.into_iter().collect::<HashSet<_>>();
    let mut valid_ideas = Vec::new();
    for idea_id in idea_ids {
        if find_idea_file(workspace, &idea_id).await?.is_some() {
            valid_ideas.push(idea_id);
        }
    }

    let meta = ProjectMeta {
        id: new_id(),
        title,
        status: "active".into(),
        tags: Vec::new(),
        created_at: now_iso(),
        updated_at: now_iso(),
        idea_ids: valid_ideas.clone(),
        skill_ids: req
            .skill_ids
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .collect(),
        preset_ids: req
            .preset_ids
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .collect(),
    };
    let raw = frontmatter::render(&meta, &req.brief)
        .map_err(|e| ApiError::internal(format!("写入项目失败：{e}")))?;
    atomic_write(&dir_path.join("project.md"), raw.as_bytes()).await?;

    for idea_id in &valid_ideas {
        detach_idea_from_other_projects(workspace, idea_id, &meta.id).await?;
        set_idea_project(workspace, idea_id, Some(&meta.id)).await?;
    }

    project_detail(workspace, &meta.id).await
}

pub async fn promote_idea(
    workspace: &Path,
    idea_id: &str,
    title: Option<String>,
    brief: Option<String>,
) -> ApiResult<Project> {
    let idea = get_idea(workspace, idea_id).await?;
    let project_title = title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| idea.meta.title.clone());
    let project_brief = brief.unwrap_or_else(|| idea.content.clone());
    create_project(
        workspace,
        CreateProject {
            title: project_title,
            brief: project_brief,
            idea_ids: vec![idea.meta.id],
            skill_ids: Vec::new(),
            preset_ids: Vec::new(),
        },
    )
    .await
}

pub async fn update_project(
    workspace: &Path,
    id: &str,
    patch: UpdateProject,
) -> ApiResult<Project> {
    let mut record = require_project(workspace, id).await?;
    let old_ideas = record.meta.idea_ids.clone();
    if let Some(title) = patch.title {
        record.meta.title = validate_title(&title)?;
    }
    if let Some(brief) = patch.brief {
        record.brief = brief;
    }
    if let Some(status) = patch.status {
        validate_status(&status, &["active", "paused", "done", "archived"])?;
        record.meta.status = status;
    }
    if let Some(tags) = patch.tags {
        record.meta.tags = normalize_tags(tags);
    }
    if let Some(idea_ids) = patch.idea_ids {
        let mut seen = HashSet::new();
        record.meta.idea_ids = idea_ids
            .into_iter()
            .filter(|x| seen.insert(x.clone()))
            .collect();
    }
    if let Some(skill_ids) = patch.skill_ids {
        record.meta.skill_ids = skill_ids;
    }
    if let Some(preset_ids) = patch.preset_ids {
        record.meta.preset_ids = preset_ids;
    }
    record.meta.updated_at = now_iso();
    let raw = frontmatter::render(&record.meta, &record.brief)
        .map_err(|e| ApiError::internal(format!("保存项目失败：{e}")))?;
    atomic_write(&record.dir_path.join("project.md"), raw.as_bytes()).await?;

    for removed in old_ideas
        .iter()
        .filter(|x| !record.meta.idea_ids.contains(x))
    {
        set_idea_project(workspace, removed, None).await?;
    }
    for added in record
        .meta
        .idea_ids
        .iter()
        .filter(|x| !old_ideas.contains(x))
    {
        if find_idea_file(workspace, added).await?.is_some() {
            detach_idea_from_other_projects(workspace, added, &record.meta.id).await?;
            set_idea_project(workspace, added, Some(&record.meta.id)).await?;
        }
    }

    project_detail(workspace, &record.meta.id).await
}

pub async fn project_summaries(workspace: &Path) -> ApiResult<Vec<ProjectSummary>> {
    let mut out = Vec::new();
    for record in list_project_records(workspace).await? {
        let grouped = assets::scan_project(workspace, &record, false).await?;
        let counts = assets::count_by_kind(&grouped);
        let cover_url = assets::cover_url(&grouped, &record.dir_name);
        out.push(ProjectSummary {
            meta: record.meta.clone(),
            brief_excerpt: excerpt(&record.brief, 160),
            dir_name: record.dir_name.clone(),
            counts,
            cover_url,
        });
    }
    out.sort_by(|a, b| b.meta.updated_at.cmp(&a.meta.updated_at));
    Ok(out)
}

pub async fn project_detail(workspace: &Path, id: &str) -> ApiResult<Project> {
    let record = require_project(workspace, id).await?;
    let grouped = assets::scan_project(workspace, &record, true).await?;
    let counts = assets::count_by_kind(&grouped);
    let cover_url = assets::cover_url(&grouped, &record.dir_name);
    Ok(Project {
        meta: record.meta,
        brief: record.brief,
        dir_name: record.dir_name,
        assets: grouped,
        counts,
        cover_url,
    })
}

pub async fn delete_project(workspace: &Path, id: &str) -> ApiResult<()> {
    let record = require_project(workspace, id).await?;
    let linked = record.meta.idea_ids.clone();
    move_to_trash(workspace, &record.dir_path, "project").await?;
    for idea_id in linked {
        set_idea_project(workspace, &idea_id, None).await?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Search / stats / browse
// ---------------------------------------------------------------------------

pub async fn search(workspace: &Path, query: &str) -> ApiResult<SearchResult> {
    let q = query.trim();
    let mut result = SearchResult {
        query: q.to_string(),
        ideas: Vec::new(),
        projects: Vec::new(),
        assets: Vec::new(),
    };
    if q.is_empty() {
        return Ok(result);
    }
    let needle = q.to_lowercase();

    for idea in list_ideas(workspace).await? {
        if idea.meta.title.to_lowercase().contains(&needle)
            || idea.content.to_lowercase().contains(&needle)
            || idea
                .meta
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
        {
            result.ideas.push(idea);
        }
        if result.ideas.len() >= 20 {
            break;
        }
    }

    let records = list_project_records(workspace).await?;
    let all_ideas = result.ideas.clone();
    for record in &records {
        let linked_idea_hit = record.meta.idea_ids.iter().any(|id| {
            all_ideas.iter().any(|idea| {
                &idea.meta.id == id
                    && (idea.meta.title.to_lowercase().contains(&needle)
                        || idea.content.to_lowercase().contains(&needle))
            })
        });
        if record.meta.title.to_lowercase().contains(&needle)
            || record.brief.to_lowercase().contains(&needle)
            || record
                .meta
                .tags
                .iter()
                .any(|t| t.to_lowercase().contains(&needle))
            || linked_idea_hit
        {
            let grouped = assets::scan_project(workspace, record, false).await?;
            result.projects.push(ProjectSummary {
                meta: record.meta.clone(),
                brief_excerpt: excerpt(&record.brief, 160),
                dir_name: record.dir_name.clone(),
                counts: assets::count_by_kind(&grouped),
                cover_url: assets::cover_url(&grouped, &record.dir_name),
            });
        }
    }

    'outer: for record in &records {
        let grouped = assets::scan_project(workspace, record, false).await?;
        for kind in ASSET_KINDS {
            if let Some(list) = grouped.get(kind) {
                for asset in list {
                    let hit = asset.meta.title.to_lowercase().contains(&needle)
                        || asset.content.to_lowercase().contains(&needle)
                        || asset
                            .meta
                            .tags
                            .iter()
                            .any(|t| t.to_lowercase().contains(&needle))
                        || asset
                            .meta
                            .prompt
                            .as_deref()
                            .map(|p| p.to_lowercase().contains(&needle))
                            .unwrap_or(false);
                    if hit {
                        result.assets.push(SearchAsset {
                            project_id: record.meta.id.clone(),
                            project_title: record.meta.title.clone(),
                            asset: asset.clone(),
                        });
                        if result.assets.len() >= 30 {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }

    Ok(result)
}

fn dir_size_sync(path: &Path) -> u64 {
    let mut total = 0u64;
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() {
            total += dir_size_sync(&entry.path());
        } else if ft.is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

pub async fn workspace_info(workspace: &Path) -> ApiResult<WorkspaceInfo> {
    let ideas = list_ideas(workspace).await?.len();
    let records = list_project_records(workspace).await?;
    let mut assets_count = 0usize;
    for record in &records {
        let grouped = assets::scan_project(workspace, record, false).await?;
        assets_count += grouped.values().map(Vec::len).sum::<usize>();
    }
    let bytes = dir_size_sync(workspace);
    Ok(WorkspaceInfo {
        path: workspace.to_string_lossy().to_string(),
        ideas,
        projects: records.len(),
        assets: assets_count,
        bytes,
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

pub async fn browse_dirs(path: Option<&str>) -> ApiResult<BrowseInfo> {
    let requested = match path.map(str::trim).filter(|p| !p.is_empty()) {
        Some(p) => expand_path(p),
        None => home_dir(),
    };
    let current = fs::canonicalize(&requested)
        .await
        .map_err(|_| ApiError::bad_request("目录不存在或无法访问"))?;
    if !fs::metadata(&current).await?.is_dir() {
        return Err(ApiError::bad_request("目标不是目录"));
    }
    let mut dirs = Vec::new();
    let mut entries = fs::read_dir(&current).await?;
    while let Some(entry) = entries.next_entry().await? {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        if let Ok(meta) = entry.metadata().await {
            if meta.is_dir() {
                dirs.push(DirEntryInfo {
                    name,
                    path: path.to_string_lossy().to_string(),
                });
            }
        }
    }
    dirs.sort_by_key(|a| a.name.to_lowercase());
    let parent = current
        .parent()
        .filter(|p| *p != current)
        .map(|p| p.to_string_lossy().to_string());
    let is_workspace = current.join(CONFIG_DIR).is_dir() && current.join("ideas").is_dir();
    Ok(BrowseInfo {
        path: current.to_string_lossy().to_string(),
        parent,
        dirs,
        is_workspace,
    })
}
