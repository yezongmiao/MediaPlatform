use std::collections::HashSet;
use std::path::{Path, PathBuf};

use tokio::fs;

use crate::error::{ApiError, ApiResult};
use crate::frontmatter;
use crate::models::*;
use crate::store::{atomic_write, move_to_trash, slugify, validate_title};

pub async fn list_skills(workspace: &Path) -> ApiResult<Vec<Skill>> {
    let dir = workspace.join(".mediadesk/skills");
    let mut out = Vec::new();
    let mut entries = match fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(err) => return Err(err.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        match read_skill_at(&path).await {
            Ok(skill) => out.push(skill),
            Err(err) => eprintln!("skip invalid skill {}: {err}", path.display()),
        }
    }
    out.sort_by(|a, b| a.meta.name.cmp(&b.meta.name));
    Ok(out)
}

async fn read_skill_at(path: &Path) -> ApiResult<Skill> {
    let raw = fs::read_to_string(path).await?;
    let (meta, content) = frontmatter::parse::<SkillMeta>(&raw)
        .map_err(|e| ApiError::internal(format!("{}：{e}", path.display())))?;
    Ok(Skill { meta, content })
}

pub async fn get_skill(workspace: &Path, id: &str) -> ApiResult<Skill> {
    for skill in list_skills(workspace).await? {
        if skill.meta.id == id {
            return Ok(skill);
        }
    }
    Err(ApiError::not_found("技能不存在"))
}

fn validate_skill_input(input: &SkillInput) -> ApiResult<(String, Vec<String>)> {
    let name = validate_title(&input.name)?;
    let applies: Vec<String> = input
        .applies_to
        .iter()
        .filter(|k| is_valid_kind(k))
        .cloned()
        .collect();
    Ok((name, applies))
}

fn unique_file(dir: &Path, base_id: &str) -> PathBuf {
    let mut id = base_id.to_string();
    let mut n = 1;
    while dir.join(format!("{id}.md")).exists() {
        n += 1;
        id = format!("{base_id}-{n}");
    }
    dir.join(format!("{id}.md"))
}

pub async fn create_skill(workspace: &Path, input: SkillInput) -> ApiResult<Skill> {
    let (name, applies) = validate_skill_input(&input)?;
    let dir = workspace.join(".mediadesk/skills");
    fs::create_dir_all(&dir).await?;
    let base = slugify(&format!("skill-{}", input.name));
    let path = unique_file(&dir, &base);
    let id = path.file_stem().unwrap().to_string_lossy().to_string();
    let meta = SkillMeta {
        id,
        name,
        description: input.description.trim().to_string(),
        applies_to: applies,
    };
    let raw = frontmatter::render(&meta, &input.content)
        .map_err(|e| ApiError::internal(format!("保存技能失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_skill_at(&path).await
}

async fn find_skill_path(workspace: &Path, id: &str) -> ApiResult<(PathBuf, Skill)> {
    let dir = workspace.join(".mediadesk/skills");
    let mut entries = fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Ok(skill) = read_skill_at(&path).await {
            if skill.meta.id == id {
                return Ok((path, skill));
            }
        }
    }
    Err(ApiError::not_found("技能不存在"))
}

pub async fn update_skill(workspace: &Path, id: &str, input: SkillInput) -> ApiResult<Skill> {
    let (path, _) = find_skill_path(workspace, id).await?;
    let (name, applies) = validate_skill_input(&input)?;
    let meta = SkillMeta {
        id: id.to_string(),
        name,
        description: input.description.trim().to_string(),
        applies_to: applies,
    };
    let raw = frontmatter::render(&meta, &input.content)
        .map_err(|e| ApiError::internal(format!("保存技能失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_skill_at(&path).await
}

pub async fn delete_skill(workspace: &Path, id: &str) -> ApiResult<()> {
    let (path, _) = find_skill_path(workspace, id).await?;
    move_to_trash(workspace, &path, "skill").await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

pub async fn list_presets(workspace: &Path) -> ApiResult<Vec<Preset>> {
    let dir = workspace.join(".mediadesk/presets");
    let mut out = Vec::new();
    let mut entries = match fs::read_dir(&dir).await {
        Ok(e) => e,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(err) => return Err(err.into()),
    };
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        match read_preset_at(&path).await {
            Ok(preset) => out.push(preset),
            Err(err) => eprintln!("skip invalid preset {}: {err}", path.display()),
        }
    }
    out.sort_by(|a, b| {
        kind_rank(&a.meta.kind)
            .cmp(&kind_rank(&b.meta.kind))
            .then_with(|| a.meta.name.cmp(&b.meta.name))
    });
    Ok(out)
}

fn kind_rank(kind: &str) -> usize {
    ASSET_KINDS.iter().position(|k| *k == kind).unwrap_or(99)
}

async fn read_preset_at(path: &Path) -> ApiResult<Preset> {
    let raw = fs::read_to_string(path).await?;
    let (meta, content) = frontmatter::parse::<PresetMeta>(&raw)
        .map_err(|e| ApiError::internal(format!("{}：{e}", path.display())))?;
    Ok(Preset { meta, content })
}

pub async fn get_preset(workspace: &Path, id: &str) -> ApiResult<Preset> {
    for preset in list_presets(workspace).await? {
        if preset.meta.id == id {
            return Ok(preset);
        }
    }
    Err(ApiError::not_found("预设不存在"))
}

pub async fn preset_for_kind(
    workspace: &Path,
    kind: &str,
    preset_id: Option<&str>,
) -> ApiResult<Option<Preset>> {
    let presets = list_presets(workspace).await?;
    if let Some(id) = preset_id {
        return presets
            .into_iter()
            .find(|p| p.meta.id == id)
            .map(Some)
            .ok_or_else(|| ApiError::not_found("预设不存在"));
    }
    Ok(presets.into_iter().find(|p| p.meta.kind == kind))
}

fn validate_preset_input(input: &PresetInput) -> ApiResult<(String, String, Vec<String>)> {
    let name = validate_title(&input.name)?;
    if !is_valid_kind(&input.kind) {
        return Err(ApiError::bad_request("预设的素材类型不合法"));
    }
    let mut seen = HashSet::new();
    let skills = input
        .skill_ids
        .iter()
        .filter(|&s| seen.insert(s.clone()))
        .cloned()
        .collect();
    Ok((name, input.kind.clone(), skills))
}

pub async fn create_preset(workspace: &Path, input: PresetInput) -> ApiResult<Preset> {
    let (name, kind, skills) = validate_preset_input(&input)?;
    let dir = workspace.join(".mediadesk/presets");
    fs::create_dir_all(&dir).await?;
    let base = slugify(&format!("preset-{}", input.name));
    let path = unique_file(&dir, &base);
    let id = path.file_stem().unwrap().to_string_lossy().to_string();
    let meta = PresetMeta {
        id,
        name,
        description: input.description.trim().to_string(),
        kind,
        skill_ids: skills,
    };
    let raw = frontmatter::render(&meta, &input.content)
        .map_err(|e| ApiError::internal(format!("保存预设失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_preset_at(&path).await
}

async fn find_preset_path(workspace: &Path, id: &str) -> ApiResult<(PathBuf, Preset)> {
    let dir = workspace.join(".mediadesk/presets");
    let mut entries = fs::read_dir(&dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        if let Ok(preset) = read_preset_at(&path).await {
            if preset.meta.id == id {
                return Ok((path, preset));
            }
        }
    }
    Err(ApiError::not_found("预设不存在"))
}

pub async fn update_preset(workspace: &Path, id: &str, input: PresetInput) -> ApiResult<Preset> {
    let (path, _) = find_preset_path(workspace, id).await?;
    let (name, kind, skills) = validate_preset_input(&input)?;
    let meta = PresetMeta {
        id: id.to_string(),
        name,
        description: input.description.trim().to_string(),
        kind,
        skill_ids: skills,
    };
    let raw = frontmatter::render(&meta, &input.content)
        .map_err(|e| ApiError::internal(format!("保存预设失败：{e}")))?;
    atomic_write(&path, raw.as_bytes()).await?;
    read_preset_at(&path).await
}

pub async fn delete_preset(workspace: &Path, id: &str) -> ApiResult<()> {
    let (path, _) = find_preset_path(workspace, id).await?;
    move_to_trash(workspace, &path, "preset").await?;
    Ok(())
}
