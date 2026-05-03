use crate::hook_binary::hook_exe_name;
use serde::Serialize;
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use super::PROMPTBOX_HOME_ENV;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptBoxPaths {
    pub home: PathBuf,
    pub config_path: PathBuf,
    pub database_path: PathBuf,
    pub spool_path: PathBuf,
    pub logs_dir: PathBuf,
    pub hook_binary_path: PathBuf,
}

impl PromptBoxPaths {
    pub fn resolve() -> Result<Self, String> {
        let home = resolve_promptbox_home()?;

        Ok(Self {
            config_path: home.join("config.toml"),
            database_path: home.join("promptbox.sqlite"),
            spool_path: home.join("spool").join("events.jsonl"),
            logs_dir: home.join("logs"),
            hook_binary_path: home.join("bin").join(hook_exe_name()),
            home,
        })
    }

    pub fn ensure_directories(&self) -> Result<(), String> {
        fs::create_dir_all(&self.home)
            .map_err(|error| format!("创建 PromptBox home 失败：{error}"))?;
        fs::create_dir_all(parent_dir(&self.spool_path)?)
            .map_err(|error| format!("创建 spool 目录失败：{error}"))?;
        fs::create_dir_all(&self.logs_dir).map_err(|error| format!("创建日志目录失败：{error}"))?;
        fs::create_dir_all(parent_dir(&self.hook_binary_path)?)
            .map_err(|error| format!("创建 hook bin 目录失败：{error}"))?;

        Ok(())
    }
}

pub fn resolve_promptbox_paths() -> Result<PromptBoxPaths, String> {
    PromptBoxPaths::resolve()
}

fn resolve_promptbox_home() -> Result<PathBuf, String> {
    if let Ok(home) = env::var(PROMPTBOX_HOME_ENV) {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    #[cfg(windows)]
    {
        let appdata = env::var("APPDATA").map_err(|error| {
            format!("无法读取 APPDATA，也没有设置 {PROMPTBOX_HOME_ENV}：{error}")
        })?;
        return Ok(PathBuf::from(appdata).join("PromptBox"));
    }

    #[cfg(not(windows))]
    {
        let home = env::var("HOME")
            .map_err(|error| format!("无法读取 HOME，也没有设置 {PROMPTBOX_HOME_ENV}：{error}"))?;
        Ok(PathBuf::from(home).join(".promptbox"))
    }
}

fn parent_dir(path: &Path) -> Result<&Path, String> {
    path.parent()
        .ok_or_else(|| format!("路径没有父目录：{}", path.display()))
}
