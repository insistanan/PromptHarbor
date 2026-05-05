use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillListItem {
    pub id: String,
    pub provider: String,
    pub provider_label: String,
    pub source_kind: String,
    pub source_label: String,
    pub local_status: String,
    pub name: String,
    pub description: String,
    pub translated_name: Option<String>,
    pub translated_description: Option<String>,
    pub translated_at: Option<String>,
    pub translated_provider_name: Option<String>,
    pub skill_dir: String,
    pub skill_file: String,
    pub relative_path: String,
    pub content_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetail {
    pub skill_file: String,
    pub content_md: String,
}

struct SkillSource {
    provider: &'static str,
    provider_label: &'static str,
    source_kind: &'static str,
    source_label: &'static str,
    local_status: &'static str,
    root: PathBuf,
    include_hidden_dirs: bool,
}

pub fn list_skills() -> Result<Vec<SkillListItem>, String> {
    let mut items = Vec::new();
    for source in skill_sources()? {
        items.extend(read_skill_source(&source)?);
    }

    items.sort_by(|left, right| {
        left.provider
            .cmp(&right.provider)
            .then(source_sort_key(&left.source_kind).cmp(&source_sort_key(&right.source_kind)))
            .then(left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then(left.relative_path.cmp(&right.relative_path))
    });

    Ok(items)
}

pub fn read_skill_detail(skill_file: &Path) -> Result<SkillDetail, String> {
    let file_name = skill_file
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| format!("技能说明文件路径无效：{}", skill_file.display()))?;
    if file_name != "SKILL.md" {
        return Err("只能读取技能目录中的 SKILL.md 说明文件".to_string());
    }

    let content_md = fs::read_to_string(skill_file)
        .map_err(|error| format!("读取技能说明文件失败：{}：{error}", skill_file.display()))?;

    Ok(SkillDetail {
        skill_file: path_to_string(skill_file),
        content_md,
    })
}

fn skill_sources() -> Result<Vec<SkillSource>, String> {
    let user_home = resolve_user_home()?;
    let codex_home = resolve_codex_home(&user_home);

    Ok(vec![
        SkillSource {
            provider: "claude",
            provider_label: "Claude Code",
            source_kind: "user",
            source_label: "Claude Code / 用户技能",
            local_status: "已安装",
            root: user_home.join(".claude").join("skills"),
            include_hidden_dirs: false,
        },
        SkillSource {
            provider: "codex",
            provider_label: "Codex CLI",
            source_kind: "user",
            source_label: "Codex CLI / 用户技能",
            local_status: "已安装",
            root: codex_home.join("skills"),
            include_hidden_dirs: false,
        },
        SkillSource {
            provider: "codex",
            provider_label: "Codex CLI",
            source_kind: "system",
            source_label: "Codex CLI / 内置技能",
            local_status: "系统内置",
            root: codex_home.join("skills").join(".system"),
            include_hidden_dirs: true,
        },
    ])
}

fn read_skill_source(source: &SkillSource) -> Result<Vec<SkillListItem>, String> {
    if !source.root.exists() {
        return Ok(Vec::new());
    }
    if !source.root.is_dir() {
        return Err(format!("技能目录不是文件夹：{}", source.root.display()));
    }

    let mut items = Vec::new();
    let entries = fs::read_dir(&source.root)
        .map_err(|error| format!("读取技能目录失败：{}：{error}", source.root.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|error| format!("读取技能目录条目失败：{}：{error}", source.root.display()))?;
        let skill_dir = entry.path();
        if !skill_dir.is_dir() {
            continue;
        }

        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if !source.include_hidden_dirs && file_name.starts_with('.') {
            continue;
        }

        let skill_file = skill_dir.join("SKILL.md");
        if !skill_file.is_file() {
            continue;
        }

        let content = fs::read_to_string(&skill_file)
            .map_err(|error| format!("读取技能说明文件失败：{}：{error}", skill_file.display()))?;
        let relative_path = normalize_relative_path(skill_dir.strip_prefix(&source.root).unwrap_or(&skill_dir));
        let metadata = parse_skill_metadata(&content, &file_name);

        items.push(SkillListItem {
            id: format!("{}:{}:{}", source.provider, source.source_kind, relative_path),
            provider: source.provider.to_string(),
            provider_label: source.provider_label.to_string(),
            source_kind: source.source_kind.to_string(),
            source_label: source.source_label.to_string(),
            local_status: source.local_status.to_string(),
            name: metadata.name,
            description: metadata.description,
            translated_name: None,
            translated_description: None,
            translated_at: None,
            translated_provider_name: None,
            skill_dir: path_to_string(&skill_dir),
            skill_file: path_to_string(&skill_file),
            relative_path,
            content_hash: content_sha256(&content),
        });
    }

    Ok(items)
}

fn content_sha256(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn resolve_user_home() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let user_profile = env::var("USERPROFILE")
            .map_err(|error| format!("无法读取 USERPROFILE：{error}"))?;
        Ok(PathBuf::from(user_profile))
    }

    #[cfg(not(windows))]
    {
        let home = env::var("HOME").map_err(|error| format!("无法读取 HOME：{error}"))?;
        Ok(PathBuf::from(home))
    }
}

fn resolve_codex_home(user_home: &Path) -> PathBuf {
    if let Ok(value) = env::var("CODEX_HOME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    user_home.join(".codex")
}

fn source_sort_key(source_kind: &str) -> usize {
    match source_kind {
        "user" => 0,
        "project" => 1,
        "system" => 2,
        _ => 9,
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

struct SkillMetadata {
    name: String,
    description: String,
}

fn parse_skill_metadata(content: &str, fallback_name: &str) -> SkillMetadata {
    let normalized = content.replace("\r\n", "\n");
    let (frontmatter_name, frontmatter_description, body) = parse_frontmatter(&normalized);
    let name = frontmatter_name
        .or_else(|| first_heading(body.as_str()))
        .unwrap_or_else(|| fallback_name.to_string());
    let description = frontmatter_description
        .or_else(|| first_paragraph(body.as_str()))
        .unwrap_or_else(|| "未提供原始描述".to_string());

    SkillMetadata { name, description }
}

fn parse_frontmatter(content: &str) -> (Option<String>, Option<String>, String) {
    if !content.starts_with("---\n") {
        return (None, None, content.to_string());
    }

    let mut lines = content.lines();
    let _ = lines.next();
    let mut frontmatter_lines = Vec::new();
    let mut body_lines = Vec::new();
    let mut in_frontmatter = true;

    for line in lines {
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            continue;
        }

        if in_frontmatter {
            frontmatter_lines.push(line);
        } else {
            body_lines.push(line);
        }
    }

    if in_frontmatter {
        return (None, None, content.to_string());
    }

    let mut name = None;
    let mut description = None;
    for line in frontmatter_lines {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key = key.trim();
        let value = clean_frontmatter_value(value);
        match key {
            "name" if !value.is_empty() => name = Some(value),
            "description" if !value.is_empty() => description = Some(value),
            _ => {}
        }
    }

    (name, description, body_lines.join("\n"))
}

fn clean_frontmatter_value(value: &str) -> String {
    let trimmed = value.trim();
    let trimmed = trimmed
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| trimmed.strip_prefix('\'').and_then(|value| value.strip_suffix('\'')))
        .unwrap_or(trimmed);

    trimmed.replace("\\\"", "\"")
}

fn first_heading(body: &str) -> Option<String> {
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let value = trimmed.trim_start_matches('#').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

fn first_paragraph(body: &str) -> Option<String> {
    let mut paragraph = Vec::new();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }

        paragraph.push(trimmed);
        if paragraph.len() >= 3 {
            break;
        }
    }

    if paragraph.is_empty() {
        None
    } else {
        Some(paragraph.join(" "))
    }
}
