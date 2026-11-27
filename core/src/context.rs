use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use tokio::fs;

use crate::assets;
use crate::error::{ApiError, ApiResult};
use crate::library;
use crate::models::*;
use crate::store::{self, atomic_write, now_iso, rel_path, require_project, slugify};

const MARKER: &str = "<!-- generated: mediadesk-context -->";

#[derive(Clone, Debug)]
pub struct PreparedContext {
    pub project_path: PathBuf,
    pub files: Vec<String>,
}

fn truncate(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push_str("\n…（已截断）");
    }
    out
}

fn json_pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

fn frontmatter_schema() -> &'static str {
    r#"---
id: 01JXXXX...           # ULID，全局唯一，不要修改已有 ID
kind: article            # article | script | illustration | card | video
title: 素材标题
status: draft            # draft | review | ready | published
tags: [标签一, 标签二]
createdAt: 2026-01-01T00:00:00Z
updatedAt: 2026-01-01T00:00:00Z
file: cover.png          # 可选，仅图片/视频素材，指向同目录媒体文件名
prompt: 生成提示词         # 可选
source: manual           # manual | llm
---

这里是 Markdown 正文。图片/视频素材的备注、文案或提示词写在这里。
"#
}

pub async fn prepare(workspace: &Path, project_id: &str) -> ApiResult<PreparedContext> {
    let record = require_project(workspace, project_id).await?;
    let grouped = assets::scan_project(workspace, &record, true).await?;
    let counts = assets::count_by_kind(&grouped);
    let ideas = store::list_ideas(workspace).await?;
    let linked_ideas: Vec<&Idea> = ideas
        .iter()
        .filter(|i| record.meta.idea_ids.contains(&i.meta.id))
        .collect();

    let all_skills = library::list_skills(workspace).await?;
    let all_presets = library::list_presets(workspace).await?;
    let selected_skills: Vec<Skill> = if record.meta.skill_ids.is_empty() {
        all_skills.clone()
    } else {
        all_skills
            .iter()
            .filter(|s| record.meta.skill_ids.contains(&s.meta.id))
            .cloned()
            .collect()
    };
    let selected_presets: Vec<Preset> = if record.meta.preset_ids.is_empty() {
        all_presets.clone()
    } else {
        all_presets
            .iter()
            .filter(|p| record.meta.preset_ids.contains(&p.meta.id))
            .cloned()
            .collect()
    };

    let project_dir = &record.dir_path;
    for sub in ["skills", "presets", "prompts", ".vscode", ".mediadesk"] {
        fs::create_dir_all(project_dir.join(sub)).await?;
    }

    // These two directories are generated copies of the library, so refresh them.
    for sub in ["skills", "presets"] {
        let dir = project_dir.join(sub);
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir).await;
        }
        fs::create_dir_all(&dir).await?;
    }

    let mut files: Vec<String> = Vec::new();
    let mut skill_links = Vec::new();
    for skill in &selected_skills {
        let name = format!("{}.md", slugify(&skill.meta.id));
        // The library keeps code as structured files. In the VSCode copy, put
        // it in the Markdown body so external AI tools see it without having to
        // understand our YAML metadata.
        let mut copy_meta = skill.meta.clone();
        copy_meta.code.clear();
        let mut body = skill.content.clone();
        body.push_str(&library::render_code_files(&skill.meta.code));
        let raw = crate::frontmatter::render(&copy_meta, &body)
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let path = project_dir.join("skills").join(&name);
        atomic_write(&path, raw.as_bytes()).await?;
        skill_links.push(format!("[{}](skills/{})", skill.meta.name, name));
        files.push(rel_path(workspace, &path));
    }
    let mut preset_links = Vec::new();
    for preset in &selected_presets {
        let name = format!("{}.md", slugify(&preset.meta.id));
        let raw = crate::frontmatter::render(&preset.meta, &preset.content)
            .map_err(|e| ApiError::internal(e.to_string()))?;
        let path = project_dir.join("presets").join(&name);
        atomic_write(&path, raw.as_bytes()).await?;
        preset_links.push(format!("[{}](presets/{})", preset.meta.name, name));
        files.push(rel_path(workspace, &path));
    }

    let mut agents = String::new();
    agents.push_str(MARKER);
    agents.push_str(&format!("\n# {} · AI 协作说明\n\n", record.meta.title));
    agents.push_str(&format!(
        "> 本文件由「媒材工坊 MediaDesk」生成（{}）。项目文件发生变更后，重新从应用点击「在 VSCode 打开」即可刷新。\n\n",
        now_iso()
    ));
    agents.push_str("## 项目档案\n\n");
    agents.push_str(&format!(
        "- 项目 ID：`{}`\n- 标题：{}\n- 状态：{}\n",
        record.meta.id, record.meta.title, record.meta.status
    ));
    if !record.meta.tags.is_empty() {
        agents.push_str(&format!("- 标签：{}\n", record.meta.tags.join("、")));
    }
    if !record.brief.trim().is_empty() {
        agents.push_str(&format!(
            "\n### 项目简介\n\n{}\n",
            truncate(record.brief.trim(), 2600)
        ));
    }

    agents.push_str("\n## 素材目录\n\n");
    agents.push_str("| 类型 | 目录 | 数量 | 说明 |\n|---|---|---|---|\n");
    for kind in ASSET_KINDS {
        let desc = match kind {
            "article" => "长文、公众号文章",
            "script" => "口播稿、分镜脚本",
            "illustration" => "图片 + 同名 Markdown sidecar",
            "card" => "文卡图 PNG + 同名 sidecar",
            "video" => "视频 + 同名 sidecar",
            _ => "",
        };
        agents.push_str(&format!(
            "| {} | `{}/` | {} | {} |\n",
            kind_label(kind),
            kind,
            counts.get(kind).copied().unwrap_or(0),
            desc
        ));
    }

    if !linked_ideas.is_empty() {
        agents.push_str("\n## 灵感与立项来源\n\n");
        for idea in linked_ideas.iter().take(6) {
            agents.push_str(&format!(
                "### {}\n\n- ID：`{}`\n- 标签：{}\n\n{}\n\n",
                idea.meta.title,
                idea.meta.id,
                if idea.meta.tags.is_empty() {
                    "无".into()
                } else {
                    idea.meta.tags.join("、")
                },
                truncate(idea.content.trim(), 3500)
            ));
        }
    }

    agents.push_str("\n## 已加载技能\n\n");
    if skill_links.is_empty() {
        agents.push_str("（无）\n");
    } else {
        for link in &skill_links {
            agents.push_str(&format!("- {link}\n"));
        }
    }
    agents.push_str("\n## 启用的预设\n\n");
    if preset_links.is_empty() {
        agents.push_str("（无）\n");
    } else {
        for link in &preset_links {
            agents.push_str(&format!("- {link}\n"));
        }
    }

    agents.push_str("\n## 文件规范\n\n");
    agents.push_str("所有素材都是 Markdown + YAML frontmatter；二进制素材配有同名 `.md` sidecar。字段格式：\n\n");
    agents.push_str("```yaml\n");
    agents.push_str(frontmatter_schema());
    agents.push_str("```\n\n");
    agents.push_str(
        r#"## 协作规则

1. 先阅读本文件、`skills/` 与 `presets/`，再开始写作或改稿。
2. 新素材放入对应类型目录；不要创建第六种素材目录。
3. 修改已有素材时保留 frontmatter 的 `id`、`createdAt`，更新 `updatedAt` 与 `title`（如需要）。
4. 不要删除图片/视频的 sidecar `.md`；它保存了素材元数据。
5. 不编造数据、来源、采访或名人名言；缺少信息时在正文末尾列出待补充问题。
6. 生成的长文/脚本保存为 Markdown；不要用代码围栏包裹整篇正文。
7. `prompts/` 保存每次 AI 生成的完整提示词快照，可用于复盘与复用。
8. `skills/` 与 `presets/` 是应用生成的副本，修改请回到媒材工坊的「技能与预设」中编辑。

## 快速开始

- 项目总览：`README.md`
- 文章：`article/`
- 脚本：`script/`
- 插图：`illustration/`
- 文卡图：`card/`
- 视频：`video/`
- 提示词快照：`prompts/`
"#,
    );

    let agents_path = project_dir.join("AGENTS.md");
    atomic_write(&agents_path, agents.as_bytes()).await?;
    files.push(rel_path(workspace, &agents_path));
    // Keep CLAUDE.md self-contained for tools that do not resolve AGENTS.md.
    let claude_path = project_dir.join("CLAUDE.md");
    atomic_write(&claude_path, agents.as_bytes()).await?;
    files.push(rel_path(workspace, &claude_path));

    let mut readme = String::new();
    readme.push_str(&format!("# {}\n\n", record.meta.title));
    readme.push_str("> 由「媒材工坊 MediaDesk」生成。用 VSCode 打开本目录即可继续创作。\n\n");
    if !record.brief.trim().is_empty() {
        readme.push_str(&format!("{}\n\n", truncate(record.brief.trim(), 1200)));
    }
    readme.push_str("## 素材概览\n\n");
    for kind in ASSET_KINDS {
        readme.push_str(&format!(
            "- **{}**（`{}/`）：{} 条\n",
            kind_label(kind),
            kind,
            counts.get(kind).copied().unwrap_or(0)
        ));
    }
    readme.push_str("\n## 灵感\n\n");
    if linked_ideas.is_empty() {
        readme.push_str("暂无关联想法。\n");
    } else {
        for idea in &linked_ideas {
            readme.push_str(&format!("- {} — {}\n", idea.meta.title, idea.excerpt));
        }
    }
    readme.push_str("\n## 协作入口\n\n- AI 协作说明：`AGENTS.md`\n- Claude Code：`CLAUDE.md`\n- 提示词快照：`prompts/`\n");
    let readme_path = project_dir.join("README.md");
    atomic_write(&readme_path, readme.as_bytes()).await?;
    files.push(rel_path(workspace, &readme_path));

    let settings = json!({
        "//": "generated by MediaDesk",
        "editor.wordWrap": "on",
        "editor.formatOnSave": false,
        "markdown.preview.breaks": true,
        "markdown.preview.fontSize": 15,
        "files.trimTrailingWhitespace": true,
        "files.associations": { "*.md": "markdown" }
    });
    let settings_path = project_dir.join(".vscode/settings.json");
    atomic_write(&settings_path, json_pretty(&settings).as_bytes()).await?;
    files.push(rel_path(workspace, &settings_path));

    let tasks = json!({
        "//": "generated by MediaDesk",
        "version": "2.0.0",
        "tasks": [
            {
                "label": "媒材工坊：打开 AI 协作说明",
                "type": "shell",
                "command": "code",
                "args": ["-r", "${workspaceFolder}/AGENTS.md"],
                "problemMatcher": [],
                "presentation": { "reveal": "never" }
            },
            {
                "label": "媒材工坊：打开项目说明",
                "type": "shell",
                "command": "code",
                "args": ["-r", "${workspaceFolder}/README.md"],
                "problemMatcher": [],
                "presentation": { "reveal": "never" }
            }
        ]
    });
    let tasks_path = project_dir.join(".vscode/tasks.json");
    atomic_write(&tasks_path, json_pretty(&tasks).as_bytes()).await?;
    files.push(rel_path(workspace, &tasks_path));

    let extensions = json!({
        "//": "generated by MediaDesk",
        "recommendations": [
            "yzhang.markdown-all-in-one",
            "shd101wyy.markdown-preview-enhanced"
        ]
    });
    let ext_path = project_dir.join(".vscode/extensions.json");
    atomic_write(&ext_path, json_pretty(&extensions).as_bytes()).await?;
    files.push(rel_path(workspace, &ext_path));

    let prompts_readme = project_dir.join("prompts/README.md");
    let prompts_note = "# 提示词快照\n\n每次通过媒材工坊调用大模型成功后，完整提示词会以 Markdown 文件保存在这里。\n\n快照包含项目上下文、启用的预设与技能、本次指令，可直接复制到其他模型继续使用。\n";
    atomic_write(&prompts_readme, prompts_note.as_bytes()).await?;
    files.push(rel_path(workspace, &prompts_readme));

    let mut asset_json = serde_json::Map::new();
    for kind in ASSET_KINDS {
        let arr: Vec<serde_json::Value> = grouped
            .get(kind)
            .map(|list| {
                list.iter()
                    .map(|asset| {
                        json!({
                            "id": asset.meta.id,
                            "title": asset.meta.title,
                            "status": asset.meta.status,
                            "path": asset.rel_path,
                            "file": asset.meta.file,
                            "prompt": asset.meta.prompt,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        asset_json.insert(kind.to_string(), serde_json::Value::Array(arr));
    }

    let context = json!({
        "schemaVersion": 1,
        "generatedAt": now_iso(),
        "generator": "MediaDesk",
        "project": {
            "id": record.meta.id,
            "title": record.meta.title,
            "status": record.meta.status,
            "tags": record.meta.tags,
            "brief": record.brief,
            "dirName": record.dir_name,
        },
        "ideas": linked_ideas.iter().map(|i| json!({
            "id": i.meta.id,
            "title": i.meta.title,
            "tags": i.meta.tags,
            "excerpt": i.excerpt,
            "path": i.rel_path,
        })).collect::<Vec<_>>(),
        "skills": selected_skills.iter().map(|s| json!({
            "id": s.meta.id, "name": s.meta.name, "path": format!("skills/{}.md", slugify(&s.meta.id)),
        })).collect::<Vec<_>>(),
        "presets": selected_presets.iter().map(|p| json!({
            "id": p.meta.id, "name": p.meta.name, "kind": p.meta.kind, "path": format!("presets/{}.md", slugify(&p.meta.id)),
        })).collect::<Vec<_>>(),
        "assetCounts": counts,
        "assets": asset_json,
    });
    let context_path = project_dir.join(".mediadesk/context.json");
    atomic_write(&context_path, json_pretty(&context).as_bytes()).await?;
    files.push(rel_path(workspace, &context_path));

    Ok(PreparedContext {
        project_path: project_dir.clone(),
        files,
    })
}

pub fn find_code_bin(override_bin: Option<&str>) -> Option<PathBuf> {
    if let Some(custom) = override_bin.map(str::trim).filter(|s| !s.is_empty()) {
        let path = PathBuf::from(custom);
        if path.is_file() {
            return Some(path);
        }
        return None;
    }

    let names: &[&str] = if cfg!(windows) {
        &["code.cmd", "code.exe", "code"]
    } else {
        &["code"]
    };
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    let mut fallbacks: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "macos") {
        fallbacks.push(PathBuf::from(
            "/Applications/Visual Studio Code.app/Contents/Resources/app/bin/code",
        ));
    }
    if cfg!(windows) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            fallbacks.push(PathBuf::from(local).join("Programs/Microsoft VS Code/bin/code.cmd"));
        }
    }
    fallbacks.push(PathBuf::from("/usr/bin/code"));
    fallbacks.push(PathBuf::from("/snap/bin/code"));
    fallbacks.into_iter().find(|p| p.is_file())
}

fn quote_path(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy())
}

pub async fn launch(
    workspace: &Path,
    project_id: &str,
    code_override: Option<&str>,
) -> ApiResult<VscodeResult> {
    let prepared = prepare(workspace, project_id).await?;
    let project_path = prepared.project_path.clone();
    let manual = format!("code {}", quote_path(&project_path));
    let bin = find_code_bin(code_override).ok_or_else(|| {
        let details = VscodeErrorDetails {
            project_path: project_path.to_string_lossy().to_string(),
            manual_command: manual.clone(),
            files_written: prepared.files.clone(),
        };
        ApiError::new(
            axum::http::StatusCode::CONFLICT,
            "VSCODE_NOT_FOUND",
            "未找到 VS Code 命令行工具。上下文文件已生成，可手动在 VS Code 中打开该目录，或安装 code 命令。",
        )
        .with_details(serde_json::to_value(details).unwrap_or(serde_json::Value::Null))
    })?;

    let mut command = {
        #[cfg(windows)]
        {
            let is_script = bin
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"))
                .unwrap_or(false);
            if is_script {
                let mut cmd = Command::new("cmd");
                cmd.arg("/C").arg(&bin);
                cmd
            } else {
                Command::new(&bin)
            }
        }
        #[cfg(not(windows))]
        {
            Command::new(&bin)
        }
    };
    let child = command
        .arg(&project_path)
        .current_dir(&project_path)
        .spawn()
        .map_err(|err| {
            ApiError::new(
                axum::http::StatusCode::CONFLICT,
                "VSCODE_NOT_FOUND",
                format!("启动 VS Code 失败：{err}"),
            )
            .with_details(json!({
                "projectPath": project_path.to_string_lossy(),
                "manualCommand": manual,
                "filesWritten": prepared.files,
            }))
        })?;
    drop(child);

    Ok(VscodeResult {
        ok: true,
        command: format!("{} {}", bin.to_string_lossy(), quote_path(&project_path)),
        project_path: project_path.to_string_lossy().to_string(),
        files_written: prepared.files,
        message: "已生成 AI 上下文文件并打开 VS Code".into(),
    })
}

pub async fn prepare_only(workspace: &Path, project_id: &str) -> ApiResult<ContextSummary> {
    let prepared = prepare(workspace, project_id).await?;
    Ok(ContextSummary {
        project_path: prepared.project_path.to_string_lossy().to_string(),
        files_written: prepared.files,
    })
}

/// Open the project directory in the platform file manager. This only launches
/// the file manager; it does not refresh context files.
pub async fn open_folder(workspace: &Path, project_id: &str) -> ApiResult<FolderResult> {
    let record = require_project(workspace, project_id).await?;
    let path = record.dir_path;
    if !path.is_dir() {
        return Err(ApiError::not_found("项目目录不存在"));
    }

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = tokio::process::Command::new("open");
        command.arg(&path);
        command
    };
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = tokio::process::Command::new("explorer");
        command.arg(&path);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = tokio::process::Command::new("xdg-open");
        command.arg(&path);
        command
    };

    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|err| {
            ApiError::new(
                axum::http::StatusCode::CONFLICT,
                "FOLDER_OPEN_FAILED",
                format!("无法打开文件管理器：{err}"),
            )
            .with_details(json!({ "projectPath": path.to_string_lossy() }))
        })?;

    Ok(FolderResult {
        ok: true,
        path: path.to_string_lossy().to_string(),
        message: "已在文件管理器中打开项目目录".into(),
    })
}
