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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_providers: Vec<CustomProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CustomProviderProtocol {
    OpenaiChat,
    OpenaiResponses,
    Anthropic,
    Gemini,
    ZhipuV4,
}

impl CustomProviderProtocol {
    pub fn label(&self) -> &'static str {
        match self {
            Self::OpenaiChat => "OpenAI Chat",
            Self::OpenaiResponses => "OpenAI Responses",
            Self::Anthropic => "Anthropic",
            Self::Gemini => "Gemini",
            Self::ZhipuV4 => "智谱 v4",
        }
    }

    pub fn supported(&self) -> bool {
        matches!(self, Self::OpenaiChat)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    pub id: String,
    pub name: String,
    pub protocol: CustomProviderProtocol,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    pub enabled: bool,
}

impl CustomProviderConfig {
    pub fn protocol_label(&self) -> &'static str {
        self.protocol.label()
    }

    pub fn supported(&self) -> bool {
        self.protocol.supported()
    }

    pub fn secret_configured(&self) -> bool {
        !self.api_key.trim().is_empty()
    }

    pub fn summary(&self) -> CustomProviderSummary {
        CustomProviderSummary {
            id: self.id.clone(),
            name: self.name.clone(),
            protocol: self.protocol.clone(),
            protocol_label: self.protocol_label().to_string(),
            base_url: self.base_url.clone(),
            default_model: self.default_model.clone(),
            enabled: self.enabled,
            supported: self.supported(),
            secret_configured: self.secret_configured(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderSummary {
    pub id: String,
    pub name: String,
    pub protocol: CustomProviderProtocol,
    pub protocol_label: String,
    pub base_url: String,
    pub default_model: String,
    pub enabled: bool,
    pub supported: bool,
    pub secret_configured: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomProviderUpsertInput {
    pub provider_id: Option<String>,
    pub name: String,
    pub protocol: CustomProviderProtocol,
    pub base_url: String,
    pub api_key: Option<String>,
    pub default_model: String,
    pub enabled: bool,
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
            custom_providers: Vec::new(),
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

    pub fn custom_provider_summaries(&self) -> Vec<CustomProviderSummary> {
        self.custom_providers
            .iter()
            .map(CustomProviderConfig::summary)
            .collect()
    }

    pub fn custom_provider(&self, provider_id: &str) -> Option<&CustomProviderConfig> {
        self.custom_providers
            .iter()
            .find(|provider| provider.id == provider_id)
    }

    pub fn custom_provider_from_input(
        &self,
        input: CustomProviderUpsertInput,
    ) -> Result<CustomProviderConfig, String> {
        let provider_id = input.provider_id.as_deref().map(str::trim).unwrap_or_default();
        let existing = if provider_id.is_empty() {
            None
        } else {
            self.custom_provider(provider_id)
        };

        if !provider_id.is_empty() && existing.is_none() {
            return Err("要更新的自定义供应商不存在".to_string());
        }

        let name = input.name.trim();
        if name.is_empty() {
            return Err("供应商名称不能为空".to_string());
        }

        if input.enabled && !input.protocol.supported() {
            return Err(format!(
                "{} 协议暂未支持，请先关闭启用开关后保存",
                input.protocol.label()
            ));
        }

        let base_url = input.base_url.trim().to_string();
        let default_model = input.default_model.trim().to_string();
        let next_api_key = input.api_key.unwrap_or_default();
        let next_api_key = next_api_key.trim();
        let api_key = if next_api_key.is_empty() {
            existing
                .map(|provider| provider.api_key.clone())
                .unwrap_or_default()
        } else {
            next_api_key.to_string()
        };

        if input.protocol.supported() {
            if base_url.is_empty() {
                return Err("OpenAI Chat 兼容接口地址不能为空".to_string());
            }
            if default_model.is_empty() {
                return Err("默认模型不能为空".to_string());
            }
            if api_key.trim().is_empty() {
                return Err("API 密钥不能为空".to_string());
            }
        }

        Ok(CustomProviderConfig {
            id: existing
                .map(|provider| provider.id.clone())
                .unwrap_or_else(generate_provider_id),
            name: name.to_string(),
            protocol: input.protocol,
            base_url,
            api_key,
            default_model,
            enabled: input.enabled,
        })
    }

    pub fn upsert_custom_provider(
        &mut self,
        input: CustomProviderUpsertInput,
    ) -> Result<CustomProviderSummary, String> {
        let provider = self.custom_provider_from_input(input)?;
        if let Some(index) = self
            .custom_providers
            .iter()
            .position(|item| item.id == provider.id)
        {
            self.custom_providers[index] = provider.clone();
        } else {
            self.custom_providers.push(provider.clone());
        }
        Ok(provider.summary())
    }

    pub fn delete_custom_provider(&mut self, provider_id: &str) -> Result<(), String> {
        let before = self.custom_providers.len();
        self.custom_providers
            .retain(|provider| provider.id != provider_id);
        if before == self.custom_providers.len() {
            return Err("要删除的自定义供应商不存在".to_string());
        }
        Ok(())
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
    custom_providers: Option<Vec<CustomProviderConfig>>,
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
        let custom_providers = self.custom_providers.unwrap_or_default();

        (
            PromptBoxConfig {
                local_endpoint,
                token,
                recording_paused,
                maybe_closed_after_hours,
                retain_raw_hook_events,
                raw_hook_events_retention_days,
                autostart,
                custom_providers,
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

fn generate_provider_id() -> String {
    let suffix: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect();
    format!("provider-{suffix}")
}
