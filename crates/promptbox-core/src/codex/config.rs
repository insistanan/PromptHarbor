use crate::hook_config::hook_command;
use serde_json::Value;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub(in crate::codex) struct CodexUserPaths {
    pub hooks_path: PathBuf,
    pub config_path: PathBuf,
}

pub(in crate::codex) fn codex_user_paths() -> Result<CodexUserPaths, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|error| format!("无法定位用户目录以读取 Codex CLI 配置：{error}"))?;
    let codex_dir = PathBuf::from(home).join(".codex");
    Ok(CodexUserPaths {
        hooks_path: codex_dir.join("hooks.json"),
        config_path: codex_dir.join("config.toml"),
    })
}

pub(in crate::codex) fn codex_hook_command(hook_path: &Path) -> String {
    hook_command(hook_path, "codex")
}

pub(in crate::codex) fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取 Codex CLI hooks 配置失败：{}：{error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("解析 Codex CLI hooks 配置失败：{}：{error}", path.display()))
}

pub(in crate::codex) fn write_json(path: &Path, root: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("创建 Codex CLI 配置目录失败：{}：{error}", parent.display())
        })?;
    }

    let serialized = serde_json::to_string_pretty(root)
        .map_err(|error| format!("序列化 Codex CLI hooks 配置失败：{error}"))?;
    fs::write(path, format!("{serialized}\n"))
        .map_err(|error| format!("写入 Codex CLI hooks 配置失败：{}：{error}", path.display()))
}

pub(in crate::codex) fn read_toml(path: &Path) -> Result<toml::Value, String> {
    let raw = fs::read_to_string(path).map_err(|error| {
        format!(
            "读取 Codex CLI config.toml 失败：{}：{error}",
            path.display()
        )
    })?;
    toml::from_str(&raw).map_err(|error| {
        format!(
            "解析 Codex CLI config.toml 失败：{}：{error}",
            path.display()
        )
    })
}

pub(in crate::codex) fn write_toml(path: &Path, root: &toml::Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!("创建 Codex CLI 配置目录失败：{}：{error}", parent.display())
        })?;
    }

    let serialized = toml::to_string_pretty(root)
        .map_err(|error| format!("序列化 Codex CLI config.toml 失败：{error}"))?;
    fs::write(path, serialized).map_err(|error| {
        format!(
            "写入 Codex CLI config.toml 失败：{}：{error}",
            path.display()
        )
    })
}

pub(in crate::codex) fn ensure_codex_hooks_enabled(
    root: &mut toml::Value,
) -> Result<(), String> {
    let table = root
        .as_table_mut()
        .ok_or_else(|| "Codex CLI config.toml 根节点不是 TOML table".to_string())?;
    let features = table
        .entry("features")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let features_table = features
        .as_table_mut()
        .ok_or_else(|| "Codex CLI config.toml 中的 [features] 不是 table".to_string())?;
    features_table.insert("hooks".to_string(), toml::Value::Boolean(true));
    Ok(())
}

pub(in crate::codex) fn codex_hooks_enabled(root: &toml::Value) -> bool {
    root.get("features")
        .and_then(|features| features.get("hooks"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
}

pub(in crate::codex) fn codex_status_message(
    hook_installed: bool,
    codex_hooks_enabled: bool,
) -> String {
    match (hook_installed, codex_hooks_enabled) {
        (true, true) => {
            "Codex CLI 用户级 hook 已安装，hooks 已开启；已运行的 Codex CLI 需要新开窗口后生效"
                .to_string()
        }
        (true, false) => "Codex CLI hook 已安装，但 hooks 尚未开启".to_string(),
        (false, true) => "hooks 已开启，但 Codex CLI hook 未安装".to_string(),
        (false, false) => "Codex CLI hook 未安装，hooks 尚未开启".to_string(),
    }
}

pub(in crate::codex) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
