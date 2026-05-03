use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

use super::{paths::PromptBoxPaths, DEFAULT_LOCAL_ENDPOINT};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBoxConfig {
    pub local_endpoint: String,
    pub token: String,
    pub recording_paused: bool,
    pub maybe_closed_after_hours: u64,
    pub retain_raw_hook_events: bool,
    pub raw_hook_events_retention_days: u64,
    pub autostart: bool,
}

impl PromptBoxConfig {
    pub fn new() -> Self {
        Self {
            local_endpoint: DEFAULT_LOCAL_ENDPOINT.to_string(),
            token: generate_token(),
            recording_paused: false,
            maybe_closed_after_hours: 12,
            retain_raw_hook_events: true,
            raw_hook_events_retention_days: 7,
            autostart: false,
        }
    }

    pub fn load_or_create(path: &Path) -> Result<(Self, bool), String> {
        if !path.exists() {
            let config = Self::new();
            config.write(path)?;
            return Ok((config, true));
        }

        let raw = fs::read_to_string(path)
            .map_err(|error| format!("读取 PromptBox 用户配置失败：{}：{error}", path.display()))?;
        let partial: PartialPromptBoxConfig = toml::from_str(&raw)
            .map_err(|error| format!("解析 PromptBox 用户配置失败：{}：{error}", path.display()))?;

        let (config, changed) = partial.into_config();
        if changed {
            config.write(path)?;
        }

        Ok((config, changed))
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建配置目录失败：{}：{error}", parent.display()))?;
        }

        let serialized = toml::to_string_pretty(self)
            .map_err(|error| format!("序列化 PromptBox 用户配置失败：{error}"))?;
        fs::write(path, serialized)
            .map_err(|error| format!("写入 PromptBox 用户配置失败：{}：{error}", path.display()))
    }
}

#[derive(Debug, Default, Deserialize)]
struct PartialPromptBoxConfig {
    local_endpoint: Option<String>,
    token: Option<String>,
    recording_paused: Option<bool>,
    maybe_closed_after_hours: Option<u64>,
    retain_raw_hook_events: Option<bool>,
    raw_hook_events_retention_days: Option<u64>,
    autostart: Option<bool>,
}

impl PartialPromptBoxConfig {
    fn into_config(self) -> (PromptBoxConfig, bool) {
        let mut changed = false;

        let local_endpoint = self.local_endpoint.unwrap_or_else(|| {
            changed = true;
            DEFAULT_LOCAL_ENDPOINT.to_string()
        });
        let token = self
            .token
            .filter(|token| !token.trim().is_empty())
            .unwrap_or_else(|| {
                changed = true;
                generate_token()
            });
        let recording_paused = self.recording_paused.unwrap_or_else(|| {
            changed = true;
            false
        });
        let maybe_closed_after_hours = self.maybe_closed_after_hours.unwrap_or_else(|| {
            changed = true;
            12
        });
        let retain_raw_hook_events = self.retain_raw_hook_events.unwrap_or_else(|| {
            changed = true;
            true
        });
        let raw_hook_events_retention_days =
            self.raw_hook_events_retention_days.unwrap_or_else(|| {
                changed = true;
                7
            });
        let autostart = self.autostart.unwrap_or_else(|| {
            changed = true;
            false
        });

        (
            PromptBoxConfig {
                local_endpoint,
                token,
                recording_paused,
                maybe_closed_after_hours,
                retain_raw_hook_events,
                raw_hook_events_retention_days,
                autostart,
            },
            changed,
        )
    }
}

pub fn load_config_for_hook() -> Result<PromptBoxConfig, String> {
    let paths = PromptBoxPaths::resolve()?;
    let raw = fs::read_to_string(&paths.config_path).map_err(|error| {
        format!(
            "读取 PromptBox 用户配置失败：{}：{error}",
            paths.config_path.display()
        )
    })?;
    let partial: PartialPromptBoxConfig = toml::from_str(&raw).map_err(|error| {
        format!(
            "解析 PromptBox 用户配置失败：{}：{error}",
            paths.config_path.display()
        )
    })?;

    Ok(partial.into_config().0)
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}
