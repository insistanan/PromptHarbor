use crate::hook_config::hook_command;
use serde_json::Value;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub(in crate::claude) fn claude_user_settings_path() -> Result<PathBuf, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|error| format!("无法定位用户目录以读取 Claude Code 配置：{error}"))?;
    Ok(PathBuf::from(home).join(".claude").join("settings.json"))
}

pub(in crate::claude) fn claude_hook_command(hook_path: &Path) -> String {
    hook_command(hook_path, "claude")
}

pub(in crate::claude) fn read_settings_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取 Claude Code 配置失败：{}：{error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("解析 Claude Code 配置失败：{}：{error}", path.display()))
}

pub(in crate::claude) fn write_settings_json(path: &Path, root: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "创建 Claude Code 配置目录失败：{}：{error}",
                parent.display()
            )
        })?;
    }

    let serialized = serde_json::to_string_pretty(root)
        .map_err(|error| format!("序列化 Claude Code 配置失败：{error}"))?;
    fs::write(path, format!("{serialized}\n"))
        .map_err(|error| format!("写入 Claude Code 配置失败：{}：{error}", path.display()))
}

pub(in crate::claude) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
