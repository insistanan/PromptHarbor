use crate::commands::providers::{load_runtime_config, request_text_with_custom_provider};
use chrono::Utc;
use promptbox_core::{
    list_skills as list_core_skills, read_skill_detail as read_core_skill_detail,
    resolve_promptbox_paths, SkillDetail, SkillListItem,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    fs::File,
    io,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillTranslationResult {
    pub translated_name: String,
    pub translated_description: String,
    pub provider_id: String,
    pub provider_name: String,
    pub model: String,
    pub updated_at: String,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportedSkillPackageSummary {
    pub package_id: String,
    pub imported_at: String,
    pub original_file_name: String,
    pub saved_zip_path: String,
    pub staged_skill_dir: String,
    pub staged_skill_file: String,
    pub skill_dir_name: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub installed_targets: Vec<InstalledSkillTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InstalledSkillTarget {
    pub provider: String,
    pub provider_label: String,
    pub target_dir: String,
    pub installed_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillInstallResult {
    pub installed: bool,
    pub requires_confirmation: bool,
    pub message: String,
    pub targets: Vec<InstalledSkillTarget>,
    pub conflicts: Vec<InstalledSkillTarget>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillTransferResult {
    pub transferred: bool,
    pub requires_confirmation: bool,
    pub message: String,
    pub target: Option<InstalledSkillTarget>,
    pub conflicts: Vec<InstalledSkillTarget>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDeleteResult {
    pub deleted: bool,
    pub message: String,
    pub target_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportedSkillPackageDeleteResult {
    pub deleted: bool,
    pub message: String,
    pub package_dir: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillTranslationCacheFile {
    #[serde(default)]
    items: BTreeMap<String, SkillTranslationCacheRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkillTranslationCacheRecord {
    translated_name: String,
    translated_description: String,
    provider_id: String,
    provider_name: String,
    model: String,
    updated_at: String,
}

#[tauri::command]
pub(crate) fn list_skills() -> Result<Vec<SkillListItem>, String> {
    let mut skills = list_core_skills()?;
    let cache = load_translation_cache()?;

    for skill in &mut skills {
        if let Some(record) = cache.items.get(&translation_cache_key(skill)) {
            skill.translated_name = Some(record.translated_name.clone());
            skill.translated_description = Some(record.translated_description.clone());
            skill.translated_at = Some(record.updated_at.clone());
            skill.translated_provider_name = Some(record.provider_name.clone());
        }
    }

    Ok(skills)
}

#[tauri::command]
pub(crate) fn read_skill_detail(skill_file: String) -> Result<SkillDetail, String> {
    read_core_skill_detail(&PathBuf::from(skill_file))
}

#[tauri::command]
pub(crate) async fn translate_skill(
    provider_id: String,
    skill_id: String,
    skill_file: String,
    content_hash: String,
    force: Option<bool>,
) -> Result<SkillTranslationResult, String> {
    let skill = load_skill_item(&skill_id, &skill_file, &content_hash)?;
    let force_refresh = force.unwrap_or(false);
    let mut cache = load_translation_cache()?;
    if !force_refresh {
        if let Some(record) = cache.items.get(&translation_cache_key(&skill)) {
            return Ok(SkillTranslationResult {
                translated_name: record.translated_name.clone(),
                translated_description: record.translated_description.clone(),
                provider_id: record.provider_id.clone(),
                provider_name: record.provider_name.clone(),
                model: record.model.clone(),
                updated_at: record.updated_at.clone(),
                cached: true,
            });
        }
    }

    let (_, config) = load_runtime_config()?;
    let provider = config
        .custom_provider(provider_id.trim())
        .ok_or_else(|| "所选自定义供应商不存在".to_string())?;

    let translation_excerpt = build_translation_excerpt(&skill.skill_file)?;
    let response = request_text_with_custom_provider(
        provider,
        vec![
            json!({
                "role": "system",
                "content": "你是一个技能元数据本地化助手。你只负责把技能名称和简介翻译成简体中文。保留品牌名、命令名、文件名、路径、协议名、代码标识符和 Markdown 语义；不要扩写，不要解释，不要增加营销口吻，不要翻译代码块；如果原文已经是合适的简体中文，只做必要润色。只输出一个 JSON 对象，格式必须是 {\"translated_name\":\"...\",\"translated_description\":\"...\"}。"
            }),
            json!({
                "role": "user",
                "content": format!(
                    "请把下面技能信息翻译成简体中文，只返回 JSON：\nname: {}\ndescription: {}\nexcerpt:\n{}",
                    skill.name,
                    skill.description,
                    translation_excerpt
                )
            }),
        ],
    )
    .await?;

    let payload = parse_translation_payload(&response.text)?;
    let updated_at = Utc::now().to_rfc3339();
    let result = SkillTranslationResult {
        translated_name: payload.0,
        translated_description: payload.1,
        provider_id: provider.id.clone(),
        provider_name: provider.name.clone(),
        model: response.model,
        updated_at: updated_at.clone(),
        cached: false,
    };

    cache.items.insert(
        translation_cache_key(&skill),
        SkillTranslationCacheRecord {
            translated_name: result.translated_name.clone(),
            translated_description: result.translated_description.clone(),
            provider_id: result.provider_id.clone(),
            provider_name: result.provider_name.clone(),
            model: result.model.clone(),
            updated_at,
        },
    );
    save_translation_cache(&cache)?;

    Ok(result)
}

#[tauri::command]
pub(crate) fn list_imported_skill_packages() -> Result<Vec<ImportedSkillPackageSummary>, String> {
    let root = imported_packages_root()?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    let entries = fs::read_dir(&root)
        .map_err(|error| format!("读取技能导入目录失败：{}：{error}", root.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("读取技能导入条目失败：{}：{error}", root.display()))?;
        let package_dir = entry.path();
        if !package_dir.is_dir() {
            continue;
        }

        let manifest_path = package_dir.join("manifest.json");
        if !manifest_path.is_file() {
            continue;
        }

        items.push(read_import_manifest(&manifest_path)?);
    }

    items.sort_by(|left, right| right.imported_at.cmp(&left.imported_at));
    Ok(items)
}

#[tauri::command]
pub(crate) fn import_skill_zip(zip_path: String) -> Result<ImportedSkillPackageSummary, String> {
    let source_path = PathBuf::from(zip_path.trim());
    if source_path.as_os_str().is_empty() {
        return Err("请输入要导入的 zip 文件路径".to_string());
    }
    if !source_path.is_file() {
        return Err(format!("zip 文件不存在：{}", source_path.display()));
    }
    let file_name = source_path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "zip 文件名无效".to_string())?;
    validate_zip_file_name(file_name)?;

    let package_id = generate_package_id();
    let package_dir = imported_packages_root()?.join(&package_id);
    let saved_zip_path = package_dir.join("source.zip");
    fs::create_dir_all(&package_dir)
        .map_err(|error| format!("创建技能导入目录失败：{}：{error}", package_dir.display()))?;
    fs::copy(&source_path, &saved_zip_path).map_err(|error| {
        format!(
            "复制技能压缩包到 PromptHarbor 数据目录失败：{}：{error}",
            saved_zip_path.display()
        )
    })?;

    build_imported_package_manifest(file_name, &package_id, &package_dir, &saved_zip_path)
}

#[tauri::command]
pub(crate) fn import_skill_zip_bytes(
    original_file_name: String,
    zip_bytes: Vec<u8>,
) -> Result<ImportedSkillPackageSummary, String> {
    let file_name = original_file_name.trim();
    if file_name.is_empty() {
        return Err("zip 文件名不能为空".to_string());
    }
    if zip_bytes.is_empty() {
        return Err("zip 文件内容为空".to_string());
    }
    validate_zip_file_name(file_name)?;

    let package_id = generate_package_id();
    let package_dir = imported_packages_root()?.join(&package_id);
    fs::create_dir_all(&package_dir)
        .map_err(|error| format!("创建技能导入目录失败：{}：{error}", package_dir.display()))?;

    let saved_zip_path = package_dir.join("source.zip");
    fs::write(&saved_zip_path, &zip_bytes).map_err(|error| {
        format!(
            "写入技能压缩包到 PromptHarbor 数据目录失败：{}：{error}",
            saved_zip_path.display()
        )
    })?;

    build_imported_package_manifest(file_name, &package_id, &package_dir, &saved_zip_path)
}

fn build_imported_package_manifest(
    original_file_name: &str,
    package_id: &str,
    package_dir: &Path,
    saved_zip_path: &Path,
) -> Result<ImportedSkillPackageSummary, String> {
    let staged_root = package_dir.join("staged");
    fs::create_dir_all(&staged_root)
        .map_err(|error| format!("创建技能导入暂存目录失败：{}：{error}", staged_root.display()))?;

    let archive_layout = inspect_skill_archive(saved_zip_path, original_file_name)?;
    let staged_skill_dir = staged_root.join(&archive_layout.skill_dir_name);
    extract_skill_archive(saved_zip_path, &archive_layout.root_prefix, &staged_skill_dir)?;

    let staged_skill_file = staged_skill_dir.join("SKILL.md");
    if !staged_skill_file.is_file() {
        return Err("压缩包缺少有效的 SKILL.md 说明文件".to_string());
    }

    let content = fs::read_to_string(&staged_skill_file).map_err(|error| {
        format!(
            "读取导入后的技能说明文件失败：{}：{error}",
            staged_skill_file.display()
        )
    })?;
    let summary = summarize_skill_markdown(&content, &archive_layout.skill_dir_name);
    let manifest = ImportedSkillPackageSummary {
        package_id: package_id.to_string(),
        imported_at: Utc::now().to_rfc3339(),
        original_file_name: original_file_name.to_string(),
        saved_zip_path: path_to_string(&saved_zip_path),
        staged_skill_dir: path_to_string(&staged_skill_dir),
        staged_skill_file: path_to_string(&staged_skill_file),
        skill_dir_name: archive_layout.skill_dir_name,
        name: summary.name,
        description: summary.description,
        installed_targets: Vec::new(),
    };

    write_import_manifest(&package_dir.join("manifest.json"), &manifest)?;
    Ok(manifest)
}

fn validate_zip_file_name(file_name: &str) -> Result<(), String> {
    if !file_name.to_ascii_lowercase().ends_with(".zip") {
        return Err("只支持导入 .zip 压缩包".to_string());
    }

    Ok(())
}

fn build_translation_excerpt(skill_file: &str) -> Result<String, String> {
    let content = fs::read_to_string(skill_file)
        .map_err(|error| format!("读取技能说明文件失败：{}：{error}", skill_file))?;
    let mut excerpt = String::new();
    let mut in_code_block = false;

    for line in content.replace("\r\n", "\n").lines() {
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        let normalized = trimmed.trim();
        if normalized.is_empty() {
            if !excerpt.ends_with("\n\n") && !excerpt.is_empty() {
                excerpt.push_str("\n\n");
            }
            continue;
        }

        excerpt.push_str(normalized);
        excerpt.push('\n');
        if excerpt.len() >= 1200 {
            break;
        }
    }

    let excerpt = excerpt.trim();
    if excerpt.is_empty() {
        Ok("（无可用摘要）".to_string())
    } else {
        Ok(excerpt.chars().take(1200).collect())
    }
}

fn write_skill_zip_from_directory(
    source_dir: &Path,
    zip_path: &Path,
    root_dir_name: &str,
) -> Result<(), String> {
    let file = File::create(zip_path)
        .map_err(|error| format!("创建技能压缩包失败：{}：{error}", zip_path.display()))?;
    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    add_directory_to_zip(&mut writer, source_dir, source_dir, root_dir_name, options)?;
    writer
        .finish()
        .map_err(|error| format!("写入技能压缩包失败：{}：{error}", zip_path.display()))?;
    Ok(())
}

fn add_directory_to_zip(
    writer: &mut ZipWriter<File>,
    root_dir: &Path,
    current_dir: &Path,
    root_dir_name: &str,
    options: SimpleFileOptions,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir)
        .map_err(|error| format!("读取技能目录失败：{}：{error}", current_dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|error| format!("读取技能条目失败：{}：{error}", current_dir.display()))?;
        let entry_path = entry.path();
        let relative = entry_path.strip_prefix(root_dir).map_err(|error| {
            format!(
                "构建技能压缩包相对路径失败：{}：{error}",
                entry_path.display()
            )
        })?;
        let zip_entry_path = normalize_relative_path(Path::new(root_dir_name).join(relative).as_path());

        if entry_path.is_dir() {
            add_directory_to_zip(writer, root_dir, &entry_path, root_dir_name, options)?;
            continue;
        }

        writer
            .start_file(zip_entry_path.replace('\\', "/"), options)
            .map_err(|error| format!("写入技能压缩包条目失败：{}：{error}", zip_entry_path))?;
        let mut source = File::open(&entry_path)
            .map_err(|error| format!("打开技能文件失败：{}：{error}", entry_path.display()))?;
        io::copy(&mut source, writer).map_err(|error| {
            format!("写入技能压缩包内容失败：{}：{error}", entry_path.display())
        })?;
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn install_imported_skill(
    package_id: String,
    targets: Vec<String>,
    target_skill_name: Option<String>,
    overwrite: bool,
) -> Result<SkillInstallResult, String> {
    let manifest_path = imported_packages_root()?.join(package_id.trim()).join("manifest.json");
    let mut manifest = read_import_manifest(&manifest_path)?;
    if !Path::new(&manifest.staged_skill_dir).is_dir() {
        return Err("导入缓存已损坏，暂存技能目录不存在，请重新导入 zip".to_string());
    }

    let providers = install_targets_from_inputs(targets)?;
    let skill_dir_name = match target_skill_name {
        Some(value) if !value.trim().is_empty() => validate_skill_dir_name(value.trim())?,
        _ => validate_skill_dir_name(manifest.skill_dir_name.trim())?,
    };

    let mut conflicts = Vec::new();
    for target in &providers {
        let destination = target.root.join(&skill_dir_name);
        if destination.exists() && !overwrite {
            conflicts.push(InstalledSkillTarget {
                provider: target.provider.to_string(),
                provider_label: target.provider_label.to_string(),
                target_dir: path_to_string(&destination),
                installed_at: String::new(),
            });
        }
    }

    if !conflicts.is_empty() {
        return Ok(SkillInstallResult {
            installed: false,
            requires_confirmation: true,
            message: "目标技能目录已存在，请确认覆盖或改名后重试".to_string(),
            targets: Vec::new(),
            conflicts,
        });
    }

    let mut installed_targets = Vec::new();
    for target in &providers {
        fs::create_dir_all(&target.root).map_err(|error| {
            format!("创建目标技能目录失败：{}：{error}", target.root.display())
        })?;

        let destination = target.root.join(&skill_dir_name);
        if destination.exists() {
            ensure_within(&target.root, &destination)?;
            fs::remove_dir_all(&destination).map_err(|error| {
                format!("覆盖目标技能目录失败：{}：{error}", destination.display())
            })?;
        }

        copy_dir_all(Path::new(&manifest.staged_skill_dir), &destination)?;
        let installed_at = Utc::now().to_rfc3339();
        installed_targets.push(InstalledSkillTarget {
            provider: target.provider.to_string(),
            provider_label: target.provider_label.to_string(),
            target_dir: path_to_string(&destination),
            installed_at: installed_at.clone(),
        });

        if let Some(existing) = manifest
            .installed_targets
            .iter_mut()
            .find(|item| item.provider == target.provider)
        {
            *existing = InstalledSkillTarget {
                provider: target.provider.to_string(),
                provider_label: target.provider_label.to_string(),
                target_dir: path_to_string(&destination),
                installed_at,
            };
        } else {
            manifest.installed_targets.push(InstalledSkillTarget {
                provider: target.provider.to_string(),
                provider_label: target.provider_label.to_string(),
                target_dir: path_to_string(&destination),
                installed_at,
            });
        }
    }

    write_import_manifest(&manifest_path, &manifest)?;

    Ok(SkillInstallResult {
        installed: true,
        requires_confirmation: false,
        message: format!("技能已安装到 {} 个目标", installed_targets.len()),
        targets: installed_targets,
        conflicts: Vec::new(),
    })
}

#[tauri::command]
pub(crate) fn export_skill_to_library(
    skill_id: String,
    skill_file: String,
    content_hash: String,
) -> Result<ImportedSkillPackageSummary, String> {
    let skill = load_skill_item(&skill_id, &skill_file, &content_hash)?;
    let source_dir = PathBuf::from(skill.skill_dir.trim());
    if !source_dir.is_dir() {
        return Err("技能目录不存在，请刷新列表后重试".to_string());
    }

    let package_id = generate_package_id();
    let package_dir = imported_packages_root()?.join(&package_id);
    fs::create_dir_all(&package_dir)
        .map_err(|error| format!("创建技能导出目录失败：{}：{error}", package_dir.display()))?;

    let skill_dir_name = skill_dir_name_from_path(&source_dir)?;
    let export_file_name = format!("{}-{}.zip", skill.provider, skill_dir_name);
    let saved_zip_path = package_dir.join("source.zip");
    write_skill_zip_from_directory(&source_dir, &saved_zip_path, &skill_dir_name)?;

    build_imported_package_manifest(&export_file_name, &package_id, &package_dir, &saved_zip_path)
}

#[tauri::command]
pub(crate) fn transfer_skill(
    skill_id: String,
    skill_file: String,
    content_hash: String,
    target_provider: String,
    target_skill_name: Option<String>,
    overwrite: bool,
) -> Result<SkillTransferResult, String> {
    let skill = load_skill_item(&skill_id, &skill_file, &content_hash)?;
    if skill.source_kind != "user" {
        return Err("只有用户技能支持转移到另一个提供者".to_string());
    }

    let normalized_target = target_provider.trim().to_lowercase();
    if normalized_target.is_empty() {
        return Err("请选择转移目标".to_string());
    }
    if normalized_target == skill.provider {
        return Err("转移目标不能和当前提供者相同".to_string());
    }

    let source_root = user_skill_root_for_provider(skill.provider.as_str())?;
    let source_dir = PathBuf::from(skill.skill_dir.trim());
    let source_name = skill_dir_name_from_path(&source_dir)?;
    let target_root = user_skill_root_for_provider(normalized_target.as_str())?;
    let target_label = provider_label(normalized_target.as_str())?;
    let target_name = match target_skill_name {
        Some(value) if !value.trim().is_empty() => validate_skill_dir_name(value.trim())?,
        _ => source_name,
    };
    let destination = target_root.join(&target_name);

    if destination.exists() && !overwrite {
        return Ok(SkillTransferResult {
            transferred: false,
            requires_confirmation: true,
            message: "目标技能目录已存在，请确认覆盖或改名后重试".to_string(),
            target: None,
            conflicts: vec![InstalledSkillTarget {
                provider: normalized_target.clone(),
                provider_label: target_label.to_string(),
                target_dir: path_to_string(&destination),
                installed_at: String::new(),
            }],
        });
    }

    fs::create_dir_all(&target_root).map_err(|error| {
        format!("创建目标技能目录失败：{}：{error}", target_root.display())
    })?;
    if destination.exists() {
        ensure_within(&target_root, &destination)?;
        fs::remove_dir_all(&destination).map_err(|error| {
            format!("覆盖目标技能目录失败：{}：{error}", destination.display())
        })?;
    }

    copy_dir_all(&source_dir, &destination)?;
    ensure_within(&source_root, &source_dir)?;
    if let Err(error) = fs::remove_dir_all(&source_dir) {
        return Err(format!(
            "技能已复制到 {}，但删除源目录失败，请手动检查：{}：{error}",
            target_label,
            source_dir.display()
        ));
    }

    let installed_at = Utc::now().to_rfc3339();
    Ok(SkillTransferResult {
        transferred: true,
        requires_confirmation: false,
        message: format!(
            "技能已从 {} 转移到 {}",
            skill.provider_label,
            target_label
        ),
        target: Some(InstalledSkillTarget {
            provider: normalized_target,
            provider_label: target_label.to_string(),
            target_dir: path_to_string(&destination),
            installed_at,
        }),
        conflicts: Vec::new(),
    })
}

#[tauri::command]
pub(crate) fn delete_skill(
    skill_id: String,
    skill_file: String,
    content_hash: String,
) -> Result<SkillDeleteResult, String> {
    let skill = load_skill_item(&skill_id, &skill_file, &content_hash)?;
    if skill.source_kind != "user" {
        return Err("只有用户技能支持删除".to_string());
    }

    let root = user_skill_root_for_provider(skill.provider.as_str())?;
    let skill_dir = PathBuf::from(skill.skill_dir.trim());
    ensure_within(&root, &skill_dir)?;
    fs::remove_dir_all(&skill_dir)
        .map_err(|error| format!("删除技能目录失败：{}：{error}", skill_dir.display()))?;

    Ok(SkillDeleteResult {
        deleted: true,
        message: format!("技能已从 {} 删除", skill.provider_label),
        target_dir: path_to_string(&skill_dir),
    })
}

#[tauri::command]
pub(crate) fn delete_imported_skill_package(
    package_id: String,
) -> Result<ImportedSkillPackageDeleteResult, String> {
    let root = imported_packages_root()?;
    let package_dir = root.join(package_id.trim());
    if !package_dir.exists() {
        return Err("资产库包不存在，请刷新列表后重试".to_string());
    }

    ensure_within(&root, &package_dir)?;
    fs::remove_dir_all(&package_dir).map_err(|error| {
        format!("删除资产库包失败：{}：{error}", package_dir.display())
    })?;

    Ok(ImportedSkillPackageDeleteResult {
        deleted: true,
        message: "资产库包已删除，已安装到 Claude/Codex 的副本不会受影响".to_string(),
        package_dir: path_to_string(&package_dir),
    })
}

fn load_skill_item(skill_id: &str, skill_file: &str, content_hash: &str) -> Result<SkillListItem, String> {
    list_core_skills()?
        .into_iter()
        .find(|item| {
            item.id == skill_id.trim()
                && item.skill_file == skill_file.trim()
                && item.content_hash == content_hash.trim()
        })
        .ok_or_else(|| "技能项已变化，请刷新列表后重试".to_string())
}

fn translation_cache_key(skill: &SkillListItem) -> String {
    format!("{}:{}", skill.id, skill.content_hash)
}

fn load_translation_cache() -> Result<SkillTranslationCacheFile, String> {
    let path = translation_cache_path()?;
    if !path.is_file() {
        return Ok(SkillTranslationCacheFile::default());
    }

    let raw = fs::read_to_string(&path)
        .map_err(|error| format!("读取技能翻译缓存失败：{}：{error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("解析技能翻译缓存失败：{}：{error}", path.display()))
}

fn save_translation_cache(cache: &SkillTranslationCacheFile) -> Result<(), String> {
    let path = translation_cache_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建技能缓存目录失败：{}：{error}", parent.display()))?;
    }

    let raw = serde_json::to_string_pretty(cache)
        .map_err(|error| format!("序列化技能翻译缓存失败：{error}"))?;
    fs::write(&path, raw)
        .map_err(|error| format!("写入技能翻译缓存失败：{}：{error}", path.display()))
}

fn translation_cache_path() -> Result<PathBuf, String> {
    Ok(skills_data_root()?.join("translations.json"))
}

fn imported_packages_root() -> Result<PathBuf, String> {
    Ok(skills_data_root()?.join("imports"))
}

fn skills_data_root() -> Result<PathBuf, String> {
    Ok(resolve_promptbox_paths()?.home.join("skills"))
}

fn parse_translation_payload(raw: &str) -> Result<(String, String), String> {
    let candidates = [
        raw.trim().to_string(),
        strip_markdown_code_fence(raw.trim()).unwrap_or_default(),
    ];

    for candidate in candidates {
        if candidate.trim().is_empty() {
            continue;
        }

        let value: Value = match serde_json::from_str(candidate.trim()) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let translated_name = value
            .get("translated_name")
            .and_then(Value::as_str)
            .or_else(|| value.get("translatedName").and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let translated_description = value
            .get("translated_description")
            .and_then(Value::as_str)
            .or_else(|| value.get("translatedDescription").and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);

        if let (Some(name), Some(description)) = (translated_name, translated_description) {
            return Ok((name, description));
        }
    }

    Err("翻译结果格式不正确，未能解析出 translated_name 和 translated_description".to_string())
}

fn strip_markdown_code_fence(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if !trimmed.starts_with("```") {
        return None;
    }

    let lines: Vec<&str> = trimmed.lines().collect();
    if lines.len() < 3 {
        return None;
    }
    if !lines.last()?.trim().starts_with("```") {
        return None;
    }

    Some(lines[1..lines.len() - 1].join("\n"))
}

fn write_import_manifest(path: &Path, manifest: &ImportedSkillPackageSummary) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建技能导入目录失败：{}：{error}", parent.display()))?;
    }

    let raw = serde_json::to_string_pretty(manifest)
        .map_err(|error| format!("序列化技能导入清单失败：{error}"))?;
    fs::write(path, raw)
        .map_err(|error| format!("写入技能导入清单失败：{}：{error}", path.display()))
}

fn read_import_manifest(path: &Path) -> Result<ImportedSkillPackageSummary, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取技能导入清单失败：{}：{error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("解析技能导入清单失败：{}：{error}", path.display()))
}

struct ArchiveSkillLayout {
    root_prefix: PathBuf,
    skill_dir_name: String,
}

fn inspect_skill_archive(zip_path: &Path, file_name: &str) -> Result<ArchiveSkillLayout, String> {
    let file = File::open(zip_path)
        .map_err(|error| format!("打开技能压缩包失败：{}：{error}", zip_path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("读取技能压缩包失败：{error}"))?;

    let mut roots = Vec::<PathBuf>::new();
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("读取技能压缩包条目失败：{error}"))?;
        if entry.is_dir() {
            continue;
        }

        let normalized = normalize_archive_path(entry.name())?;
        if normalized
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value == "SKILL.md")
        {
            let parent = normalized.parent().unwrap_or(Path::new("")).to_path_buf();
            if !roots.iter().any(|item| item == &parent) {
                roots.push(parent);
            }
        }
    }

    if roots.is_empty() {
        return Err("压缩包里没有找到有效的 SKILL.md 技能说明文件".to_string());
    }
    if roots.len() > 1 {
        return Err("压缩包里包含多个技能目录，当前一次只支持导入一个技能".to_string());
    }

    let root_prefix = roots.remove(0);
    let skill_dir_name = if root_prefix.as_os_str().is_empty() {
        sanitize_skill_dir_name(
            Path::new(file_name)
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("imported-skill"),
        )
    } else {
        sanitize_skill_dir_name(
            root_prefix
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("imported-skill"),
        )
    };

    Ok(ArchiveSkillLayout {
        root_prefix,
        skill_dir_name,
    })
}

fn extract_skill_archive(
    zip_path: &Path,
    root_prefix: &Path,
    staged_skill_dir: &Path,
) -> Result<(), String> {
    let file = File::open(zip_path)
        .map_err(|error| format!("打开技能压缩包失败：{}：{error}", zip_path.display()))?;
    let mut archive =
        ZipArchive::new(file).map_err(|error| format!("读取技能压缩包失败：{error}"))?;

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("读取技能压缩包条目失败：{error}"))?;
        if entry.is_dir() {
            continue;
        }

        let normalized = normalize_archive_path(entry.name())?;
        let relative = if root_prefix.as_os_str().is_empty() {
            normalized
        } else if normalized.starts_with(root_prefix) {
            normalized
                .strip_prefix(root_prefix)
                .map_err(|error| format!("解析技能压缩包路径失败：{error}"))?
                .to_path_buf()
        } else {
            continue;
        };

        if relative.as_os_str().is_empty() {
            continue;
        }

        let destination = staged_skill_dir.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                format!("创建导入技能文件夹失败：{}：{error}", parent.display())
            })?;
        }

        let mut output = File::create(&destination).map_err(|error| {
            format!("写入导入技能文件失败：{}：{error}", destination.display())
        })?;
        io::copy(&mut entry, &mut output).map_err(|error| {
            format!("解压技能文件失败：{}：{error}", destination.display())
        })?;
    }

    Ok(())
}

fn normalize_archive_path(raw: &str) -> Result<PathBuf, String> {
    let path = Path::new(raw);
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => normalized.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::Prefix(_) | Component::RootDir => {
                return Err(format!("压缩包包含不安全路径：{raw}"));
            }
        }
    }

    Ok(normalized)
}

struct MarkdownSummary {
    name: String,
    description: String,
}

fn summarize_skill_markdown(content: &str, fallback_name: &str) -> MarkdownSummary {
    let normalized = content.replace("\r\n", "\n");
    let (frontmatter_name, frontmatter_description, body) = parse_frontmatter(&normalized);
    let name = frontmatter_name
        .or_else(|| first_heading(body.as_str()))
        .unwrap_or_else(|| fallback_name.to_string());
    let description = frontmatter_description
        .or_else(|| first_paragraph(body.as_str()))
        .unwrap_or_else(|| "未提供原始描述".to_string());

    MarkdownSummary { name, description }
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

struct InstallTargetRoot {
    provider: &'static str,
    provider_label: &'static str,
    root: PathBuf,
}

fn install_targets_from_inputs(targets: Vec<String>) -> Result<Vec<InstallTargetRoot>, String> {
    if targets.is_empty() {
        return Err("请至少选择一个安装目标".to_string());
    }

    let user_home = resolve_user_home()?;
    let codex_home = resolve_codex_home(&user_home);
    let mut result = Vec::new();
    for target in targets {
        let normalized = target.trim().to_lowercase();
        match normalized.as_str() {
            "claude" if !result.iter().any(|item: &InstallTargetRoot| item.provider == "claude") => {
                result.push(InstallTargetRoot {
                    provider: "claude",
                    provider_label: "Claude Code",
                    root: user_home.join(".claude").join("skills"),
                });
            }
            "codex" if !result.iter().any(|item: &InstallTargetRoot| item.provider == "codex") => {
                result.push(InstallTargetRoot {
                    provider: "codex",
                    provider_label: "Codex CLI",
                    root: codex_home.join("skills"),
                });
            }
            "claude" | "codex" => {}
            _ => return Err(format!("未知的安装目标：{target}")),
        }
    }

    if result.is_empty() {
        return Err("请至少选择一个有效的安装目标".to_string());
    }

    Ok(result)
}

fn user_skill_root_for_provider(provider: &str) -> Result<PathBuf, String> {
    let user_home = resolve_user_home()?;
    let codex_home = resolve_codex_home(&user_home);
    match provider {
        "claude" => Ok(user_home.join(".claude").join("skills")),
        "codex" => Ok(codex_home.join("skills")),
        _ => Err(format!("未知的提供者：{provider}")),
    }
}

fn provider_label(provider: &str) -> Result<&'static str, String> {
    match provider {
        "claude" => Ok("Claude Code"),
        "codex" => Ok("Codex CLI"),
        _ => Err(format!("未知的提供者：{provider}")),
    }
}

fn skill_dir_name_from_path(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(str::to_string)
        .ok_or_else(|| format!("技能目录名无效：{}", path.display()))
}

fn resolve_user_home() -> Result<PathBuf, String> {
    #[cfg(windows)]
    {
        let user_profile =
            std::env::var("USERPROFILE").map_err(|error| format!("无法读取 USERPROFILE：{error}"))?;
        Ok(PathBuf::from(user_profile))
    }

    #[cfg(not(windows))]
    {
        let home = std::env::var("HOME").map_err(|error| format!("无法读取 HOME：{error}"))?;
        Ok(PathBuf::from(home))
    }
}

fn resolve_codex_home(user_home: &Path) -> PathBuf {
    if let Ok(value) = std::env::var("CODEX_HOME") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }

    user_home.join(".codex")
}

fn sanitize_skill_dir_name(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        let invalid = matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|');
        if ch.is_control() || invalid {
            result.push('-');
        } else {
            result.push(ch);
        }
    }

    let trimmed = result.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "imported-skill".to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_skill_dir_name(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("技能目录名不能为空".to_string());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("技能目录名不能是 . 或 ..".to_string());
    }
    if trimmed.chars().any(|ch| {
        ch.is_control() || matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
    }) {
        return Err("技能目录名包含非法字符，请改名后重试".to_string());
    }

    Ok(trimmed.to_string())
}

fn ensure_within(root: &Path, target: &Path) -> Result<(), String> {
    let normalized_root = root.components().collect::<Vec<_>>();
    let normalized_target = target.components().collect::<Vec<_>>();
    if normalized_target.len() < normalized_root.len()
        || !normalized_root
            .iter()
            .zip(normalized_target.iter())
            .all(|(left, right)| left == right)
    {
        return Err(format!(
            "目标路径越界，拒绝操作：{}",
            target.display()
        ));
    }
    Ok(())
}

fn copy_dir_all(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("创建目录失败：{}：{error}", destination.display()))?;
    let entries = fs::read_dir(source)
        .map_err(|error| format!("读取目录失败：{}：{error}", source.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("读取目录条目失败：{}：{error}", source.display()))?;
        let entry_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry_path.is_dir() {
            copy_dir_all(&entry_path, &destination_path)?;
        } else {
            fs::copy(&entry_path, &destination_path).map_err(|error| {
                format!(
                    "复制文件失败：{} -> {}：{error}",
                    entry_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn generate_package_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_millis())
        .unwrap_or(0);
    format!("skill-import-{millis}")
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn normalize_relative_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
