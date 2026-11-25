use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use tokio::fs;

use crate::error::{ApiError, ApiResult};
use crate::frontmatter;
use crate::models::*;
use crate::store::{
    atomic_write, media_url, normalize_tags, now_iso, rel_path, require_project, slugify,
    validate_status, validate_title, ProjectRecord,
};

const IMAGE_EXTS: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "avif"];
const VIDEO_EXTS: &[&str] = &["mp4", "webm", "mov", "m4v", "ogv"];

pub fn is_media_kind(kind: &str) -> bool {
    matches!(kind, "illustration" | "card" | "video")
}

pub fn allowed_exts(kind: &str) -> &'static [&'static str] {
    match kind {
        "illustration" | "card" => IMAGE_EXTS,
        "video" => VIDEO_EXTS,
        _ => &[],
    }
}

pub fn is_allowed_media(kind: &str, file_name: &str) -> bool {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    allowed_exts(kind).contains(&ext.as_str())
}

fn extension_of(file_name: &str) -> Option<String> {
    let ext = Path::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext.is_empty() || ext.len() > 6 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        None
    } else {
        Some(ext)
    }
}

fn safe_base_name(name: &str) -> Option<String> {
    if name.is_empty() || name.contains('\0') {
        return None;
    }
    let p = Path::new(name);
    if p.components().count() != 1 {
        return None;
    }
    let file = p.file_name()?.to_string_lossy().to_string();
    if file.starts_with('.') || file.contains('/') || file.contains('\\') {
        return None;
    }
    Some(file)
}

fn pretty_title(file_name: &str) -> String {
    let stem = Path::new(file_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| file_name.to_string());
    let stripped = if stem.len() > 27
        && stem.is_char_boundary(26)
        && stem[..26].chars().all(|c| c.is_ascii_alphanumeric())
        && stem.as_bytes().get(26) == Some(&b'-')
    {
        stem[27..].to_string()
    } else {
        stem
    };
    let title = stripped.replace(['-', '_'], " ").trim().to_string();
    if title.is_empty() {
        "未命名素材".to_string()
    } else {
        title
    }
}

fn modified_iso(meta: &std::fs::Metadata) -> String {
    meta.modified()
        .ok()
        .map(|t| {
            let dt: DateTime<Utc> = t.into();
            dt.to_rfc3339_opts(SecondsFormat::Secs, true)
        })
        .unwrap_or_else(now_iso)
}

fn make_asset(
    meta: AssetMeta,
    content: String,
    content_path: &Path,
    kind_dir: &str,
    project_id: &str,
    project_dir: &str,
    available_media: &HashSet<String>,
) -> Asset {
    let file = meta
        .file
        .as_deref()
        .and_then(safe_base_name)
        .filter(|f| available_media.contains(f))
        .or_else(|| {
            if is_media_kind(&meta.kind) {
                let stem = content_path.file_stem()?.to_string_lossy().to_string();
                available_media
                    .iter()
                    .find(|f| {
                        Path::new(f)
                            .file_stem()
                            .map(|s| s.to_string_lossy() == stem)
                            .unwrap_or(false)
                    })
                    .cloned()
            } else {
                None
            }
        });
    let file_url = file.as_ref().map(|f| media_url(project_dir, kind_dir, f));
    Asset {
        meta,
        content,
        file_url,
        rel_path: String::new(),
        project_id: project_id.to_string(),
    }
}

async fn read_asset_at(
    workspace: &Path,
    record: &ProjectRecord,
    kind: &str,
    path: &Path,
    available_media: &HashSet<String>,
) -> ApiResult<Asset> {
    let raw = fs::read_to_string(path).await?;
    let (mut meta, content) = frontmatter::parse::<AssetMeta>(&raw)
        .map_err(|e| ApiError::internal(format!("{}：{e}", path.display())))?;
    if !is_valid_kind(&meta.kind) {
        meta.kind = kind.to_string();
    }
    let mut asset = make_asset(
        meta,
        content,
        path,
        kind,
        &record.meta.id,
        &record.dir_name,
        available_media,
    );
    asset.rel_path = rel_path(workspace, path);
    Ok(asset)
}

fn can_serve(kind: &str, name: &str) -> bool {
    is_media_kind(kind) && is_allowed_media(kind, name)
}

pub async fn scan_kind(
    workspace: &Path,
    record: &ProjectRecord,
    kind: &str,
    adopt: bool,
) -> ApiResult<Vec<Asset>> {
    let dir = record.dir_path.join(kind);
    let mut md_files: Vec<PathBuf> = Vec::new();
    let mut media: HashMap<String, String> = HashMap::new();
    let mut media_set: HashSet<String> = HashSet::new();
    let mut existing_stems: HashSet<String> = HashSet::new();

    match fs::read_dir(&dir).await {
        Ok(mut entries) => {
            while let Some(entry) = entries.next_entry().await? {
                let Some(ft) = entry.file_type().await.ok() else {
                    continue;
                };
                if !ft.is_file() {
                    continue;
                }
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with('.') || name.ends_with(".part") {
                    continue;
                }
                if name.ends_with(".md") {
                    let stem = name.trim_end_matches(".md").to_string();
                    existing_stems.insert(stem.clone());
                    md_files.push(entry.path());
                } else if can_serve(kind, &name) {
                    let stem = Path::new(&name)
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    media.insert(stem, name.clone());
                    media_set.insert(name);
                }
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }

    let mut assets = Vec::new();
    for path in md_files {
        match read_asset_at(workspace, record, kind, &path, &media_set).await {
            Ok(asset) => assets.push(asset),
            Err(err) => eprintln!("skip invalid asset {}: {err}", path.display()),
        }
    }

    for (stem, file_name) in media {
        if existing_stems.contains(&stem) {
            continue;
        }
        let media_path = dir.join(&file_name);
        let meta = std::fs::metadata(&media_path).ok();
        let created = meta.as_ref().map(modified_iso).unwrap_or_else(now_iso);
        let title = pretty_title(&file_name);
        if adopt {
            let meta = AssetMeta {
                id: crate::store::new_id(),
                kind: kind.to_string(),
                title: title.clone(),
                status: "draft".into(),
                tags: Vec::new(),
                created_at: created.clone(),
                updated_at: created.clone(),
                file: Some(file_name.clone()),
                prompt: None,
                source: Some("manual".into()),
                ..AssetMeta::default()
            };
            let sidecar = dir.join(format!("{stem}.md"));
            let raw = frontmatter::render(&meta, "")
                .map_err(|e| ApiError::internal(format!("收养素材失败：{e}")))?;
            atomic_write(&sidecar, raw.as_bytes()).await?;
            let mut asset = Asset {
                meta,
                content: String::new(),
                file_url: Some(media_url(&record.dir_name, kind, &file_name)),
                rel_path: rel_path(workspace, &sidecar),
                project_id: record.meta.id.clone(),
            };
            asset.meta.kind = kind.to_string();
            assets.push(asset);
        } else {
            assets.push(Asset {
                meta: AssetMeta {
                    id: format!("orphan-{stem}"),
                    kind: kind.to_string(),
                    title,
                    status: "draft".into(),
                    tags: Vec::new(),
                    created_at: created.clone(),
                    updated_at: created,
                    file: Some(file_name.clone()),
                    prompt: None,
                    source: Some("manual".into()),
                    ..AssetMeta::default()
                },
                content: String::new(),
                file_url: Some(media_url(&record.dir_name, kind, &file_name)),
                rel_path: String::new(),
                project_id: record.meta.id.clone(),
            });
        }
    }

    assets.sort_by(|a, b| b.meta.updated_at.cmp(&a.meta.updated_at));
    Ok(assets)
}

pub async fn scan_project(
    workspace: &Path,
    record: &ProjectRecord,
    adopt: bool,
) -> ApiResult<BTreeMap<String, Vec<Asset>>> {
    let mut map = BTreeMap::new();
    for kind in ASSET_KINDS {
        let assets = scan_kind(workspace, record, kind, adopt).await?;
        map.insert(kind.to_string(), assets);
    }
    Ok(map)
}

pub fn count_by_kind(grouped: &BTreeMap<String, Vec<Asset>>) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for kind in ASSET_KINDS {
        counts.insert(
            kind.to_string(),
            grouped.get(kind).map(Vec::len).unwrap_or(0),
        );
    }
    counts
}

pub fn cover_url(grouped: &BTreeMap<String, Vec<Asset>>, _dir_name: &str) -> Option<String> {
    for kind in ["illustration", "card"] {
        if let Some(assets) = grouped.get(kind) {
            for asset in assets {
                if let Some(url) = &asset.file_url {
                    return Some(url.clone());
                }
            }
        }
    }
    None
}

// ---------------------------------------------------------------------------
// CRUD
// ---------------------------------------------------------------------------

struct AssetLocation {
    path: PathBuf,
    kind: String,
    meta: AssetMeta,
    content: String,
}

async fn find_asset_file(
    workspace: &Path,
    project_id: &str,
    asset_id: &str,
) -> ApiResult<(ProjectRecord, AssetLocation)> {
    let record = require_project(workspace, project_id).await?;
    for kind in ASSET_KINDS {
        let dir = record.dir_path.join(kind);
        let mut entries = match fs::read_dir(&dir).await {
            Ok(e) => e,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => return Err(err.into()),
        };
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(raw) = fs::read_to_string(&path).await else {
                continue;
            };
            let Ok((meta, content)) = frontmatter::parse::<AssetMeta>(&raw) else {
                continue;
            };
            if meta.id == asset_id {
                return Ok((
                    record,
                    AssetLocation {
                        path,
                        kind: kind.to_string(),
                        meta,
                        content,
                    },
                ));
            }
        }
    }
    Err(ApiError::not_found("素材不存在"))
}

async fn read_asset_by_id(workspace: &Path, project_id: &str, asset_id: &str) -> ApiResult<Asset> {
    let (record, location) = find_asset_file(workspace, project_id, asset_id).await?;
    let dir = location
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| record.dir_path.join(&location.kind));
    let mut media_set = HashSet::new();
    if let Ok(mut entries) = fs::read_dir(&dir).await {
        while let Some(entry) = entries.next_entry().await.ok().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if can_serve(&location.kind, &name) {
                media_set.insert(name);
            }
        }
    }
    let mut asset = read_asset_at(
        workspace,
        &record,
        &location.kind,
        &location.path,
        &media_set,
    )
    .await?;
    asset.meta = location.meta;
    asset.content = location.content;
    Ok(asset)
}

pub async fn get_asset(workspace: &Path, project_id: &str, asset_id: &str) -> ApiResult<Asset> {
    read_asset_by_id(workspace, project_id, asset_id).await
}

pub async fn create_asset(
    workspace: &Path,
    project_id: &str,
    req: CreateAsset,
) -> ApiResult<Asset> {
    if !is_valid_kind(&req.kind) {
        return Err(ApiError::bad_request("素材类型不合法"));
    }
    let record = require_project(workspace, project_id).await?;
    let title = validate_title(&req.title)?;
    let status = req.status.unwrap_or_else(|| "draft".into());
    validate_status(&status, &["draft", "review", "ready", "published"])?;
    let meta = AssetMeta {
        id: crate::store::new_id(),
        kind: req.kind.clone(),
        title,
        status,
        tags: normalize_tags(req.tags),
        created_at: now_iso(),
        updated_at: now_iso(),
        file: None,
        prompt: req.prompt.filter(|p| !p.trim().is_empty()),
        source: Some(match req.source.as_deref() {
            Some("llm") => "llm".into(),
            _ => "manual".into(),
        }),
        ..AssetMeta::default()
    };
    fs::create_dir_all(record.dir_path.join(&req.kind)).await?;
    let slug = slugify(&meta.title);
    let path = record
        .dir_path
        .join(&req.kind)
        .join(format!("{}-{slug}.md", meta.id));
    let raw = frontmatter::render(&meta, &req.content)
        .map_err(|e| ApiError::internal(format!("创建素材失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_asset_by_id(workspace, project_id, &meta.id).await
}

pub async fn update_asset(
    workspace: &Path,
    project_id: &str,
    asset_id: &str,
    patch: UpdateAsset,
) -> ApiResult<Asset> {
    let (record, mut location) = find_asset_file(workspace, project_id, asset_id).await?;
    if let Some(title) = patch.title {
        location.meta.title = validate_title(&title)?;
    }
    if let Some(content) = patch.content {
        location.content = content;
    }
    if let Some(status) = patch.status {
        validate_status(&status, &["draft", "review", "ready", "published"])?;
        location.meta.status = status;
    }
    if let Some(tags) = patch.tags {
        location.meta.tags = normalize_tags(tags);
    }
    if let Some(prompt) = patch.prompt {
        location.meta.prompt = if prompt.trim().is_empty() {
            None
        } else {
            Some(prompt)
        };
    }
    location.meta.updated_at = now_iso();
    let raw = frontmatter::render(&location.meta, &location.content)
        .map_err(|e| ApiError::internal(format!("保存素材失败：{e}")))?;
    atomic_write(&location.path, raw.as_bytes()).await?;
    drop(record);
    read_asset_by_id(workspace, project_id, asset_id).await
}

/// Attach a generated image to a logical batch group. The same metadata is
/// reused by card groups and illustration groups.
pub async fn set_asset_group(
    workspace: &Path,
    project_id: &str,
    asset_id: &str,
    group_id: &str,
    group_title: &str,
    index: usize,
    total: usize,
) -> ApiResult<()> {
    let (_, mut location) = find_asset_file(workspace, project_id, asset_id).await?;
    location.meta.card_group_id = Some(group_id.to_string());
    location.meta.card_group_title = Some(group_title.to_string());
    location.meta.card_group_index = Some(index);
    location.meta.card_group_total = Some(total);
    location.meta.updated_at = now_iso();
    let raw = frontmatter::render(&location.meta, &location.content)
        .map_err(|e| ApiError::internal(format!("保存素材分组失败：{e}")))?;
    atomic_write(&location.path, raw.as_bytes()).await
}

pub async fn delete_asset(workspace: &Path, project_id: &str, asset_id: &str) -> ApiResult<()> {
    let (record, location) = find_asset_file(workspace, project_id, asset_id).await?;
    let dir = location
        .path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| record.dir_path.join(&location.kind));
    let mut media_to_trash: Option<PathBuf> = None;
    if let Some(file) = location.meta.file.as_deref().and_then(safe_base_name) {
        let candidate = dir.join(file);
        if candidate.exists() {
            media_to_trash = Some(candidate);
        }
    }
    if let Some(media) = media_to_trash {
        crate::store::move_to_trash(workspace, &media, "asset-media").await?;
    }
    crate::store::move_to_trash(workspace, &location.path, "asset").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Upload plan (body streaming lives in the HTTP layer)
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct UploadPlan {
    pub project_id: String,
    pub dir_name: String,
    pub kind: String,
    pub meta: AssetMeta,
    pub content: String,
    pub md_path: PathBuf,
    pub media_path: PathBuf,
    pub temp_path: PathBuf,
    pub replaced: Option<PathBuf>,
}

#[derive(Clone, Debug, Default)]
pub struct UploadCardGroup {
    pub id: Option<String>,
    pub title: Option<String>,
    pub index: Option<usize>,
    pub total: Option<usize>,
}

pub async fn plan_upload(
    workspace: &Path,
    project_id: &str,
    kind: &str,
    asset_id: Option<&str>,
    file_name: &str,
    title: Option<&str>,
    card_group: Option<UploadCardGroup>,
) -> ApiResult<UploadPlan> {
    if !is_media_kind(kind) {
        return Err(ApiError::bad_request("该素材类型不支持上传文件"));
    }
    let file_name =
        safe_base_name(file_name).ok_or_else(|| ApiError::bad_request("文件名不合法"))?;
    let ext = extension_of(&file_name).ok_or_else(|| ApiError::bad_request("文件缺少扩展名"))?;
    if !allowed_exts(kind).contains(&ext.as_str()) {
        return Err(ApiError::bad_request(format!(
            "不支持的文件类型 .{ext}，允许：{}",
            allowed_exts(kind).join(", ")
        )));
    }
    let record = require_project(workspace, project_id).await?;
    let dir = record.dir_path.join(kind);
    fs::create_dir_all(&dir).await?;

    if let Some(asset_id) = asset_id {
        let (_, location) = find_asset_file(workspace, project_id, asset_id).await?;
        if location.kind != kind {
            return Err(ApiError::bad_request("素材类型与上传目录不一致"));
        }
        let stem = location
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| asset_id.to_string());
        let media_name = format!("{stem}.{ext}");
        let media_path = dir.join(&media_name);
        let replaced = location
            .meta
            .file
            .as_deref()
            .and_then(safe_base_name)
            .map(|f| dir.join(f))
            .filter(|p| p.exists() && *p != media_path);
        let resolved_title = match title.map(str::trim).filter(|t| !t.is_empty()) {
            Some(t) => validate_title(t)?,
            None => location.meta.title.clone(),
        };
        let mut meta = location.meta;
        meta.title = resolved_title;
        meta.file = Some(media_name);
        if let Some(group) = card_group.as_ref().filter(|group| {
            group.id.is_some()
                || group.title.is_some()
                || group.index.is_some()
                || group.total.is_some()
        }) {
            meta.card_group_id = group.id.clone().filter(|v| !v.trim().is_empty());
            meta.card_group_title = group.title.clone().filter(|v| !v.trim().is_empty());
            meta.card_group_index = group.index;
            meta.card_group_total = group.total;
        }
        meta.updated_at = now_iso();
        return Ok(UploadPlan {
            project_id: project_id.to_string(),
            dir_name: record.dir_name,
            kind: kind.to_string(),
            meta,
            content: location.content,
            md_path: location.path,
            media_path,
            temp_path: dir.join(format!(".upload-{}.part", crate::store::new_id())),
            replaced,
        });
    }

    let title = match title.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => validate_title(t)?,
        None => validate_title(&pretty_title(&file_name))?,
    };
    let id = crate::store::new_id();
    let slug = slugify(&title);
    let stem = format!("{id}-{slug}");
    let media_name = format!("{stem}.{ext}");
    let meta = AssetMeta {
        id: id.clone(),
        kind: kind.to_string(),
        title,
        status: "draft".into(),
        tags: Vec::new(),
        created_at: now_iso(),
        updated_at: now_iso(),
        file: Some(media_name),
        prompt: None,
        source: Some("manual".into()),
        card_group_id: card_group
            .as_ref()
            .and_then(|group| group.id.clone())
            .filter(|v| !v.trim().is_empty()),
        card_group_title: card_group
            .as_ref()
            .and_then(|group| group.title.clone())
            .filter(|v| !v.trim().is_empty()),
        card_group_index: card_group.as_ref().and_then(|group| group.index),
        card_group_total: card_group.as_ref().and_then(|group| group.total),
    };
    Ok(UploadPlan {
        project_id: project_id.to_string(),
        dir_name: record.dir_name,
        kind: kind.to_string(),
        meta,
        content: String::new(),
        md_path: dir.join(format!("{stem}.md")),
        media_path: dir.join(format!("{stem}.{ext}")),
        temp_path: dir.join(format!(".upload-{id}.part")),
        replaced: None,
    })
}

pub async fn finish_upload(workspace: &Path, plan: UploadPlan) -> ApiResult<Asset> {
    if let Some(replaced) = &plan.replaced {
        if replaced.exists() {
            crate::store::move_to_trash(workspace, replaced, "replaced-media").await?;
        }
    }
    if plan.media_path.exists() {
        let _ = fs::remove_file(&plan.media_path).await;
    }
    fs::rename(&plan.temp_path, &plan.media_path).await?;
    let raw = frontmatter::render(&plan.meta, &plan.content)
        .map_err(|e| ApiError::internal(format!("保存上传素材失败：{e}")))?;
    if let Err(err) = atomic_write(&plan.md_path, raw.as_bytes()).await {
        let _ = fs::remove_file(&plan.media_path).await;
        return Err(err);
    }
    read_asset_by_id(workspace, &plan.project_id, &plan.meta.id).await
}
