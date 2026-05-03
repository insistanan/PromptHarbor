use crate::{
    claude::ClaudeHookAdapter,
    codex::CodexHookAdapter,
    event::Provider,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookAdapterStatus {
    pub provider: String,
    pub provider_label: String,
    pub expected_command: String,
    pub installed: bool,
    pub ready: bool,
    pub readable: bool,
    pub message: String,
    pub config_paths: Vec<HookConfigPathStatus>,
    pub backup_paths: Vec<HookBackupPath>,
    pub codex_hooks_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookConfigPathStatus {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookBackupPath {
    pub label: String,
    pub path: String,
}

pub trait HookAdapter {
    fn detect(&self, hook_path: &Path) -> Result<HookAdapterStatus, String>;
    fn install(&self, hook_path: &Path) -> Result<HookAdapterStatus, String>;
    fn uninstall(&self, hook_path: &Path) -> Result<HookAdapterStatus, String>;
}

pub fn detect_user_hook(
    provider: Provider,
    hook_path: &Path,
) -> Result<HookAdapterStatus, String> {
    adapter_for(provider).detect(hook_path)
}

pub fn install_user_hook(
    provider: Provider,
    hook_path: &Path,
) -> Result<HookAdapterStatus, String> {
    adapter_for(provider).install(hook_path)
}

pub fn uninstall_user_hook(
    provider: Provider,
    hook_path: &Path,
) -> Result<HookAdapterStatus, String> {
    adapter_for(provider).uninstall(hook_path)
}

fn adapter_for(provider: Provider) -> Box<dyn HookAdapter> {
    match provider {
        Provider::Claude => Box::new(ClaudeHookAdapter),
        Provider::Codex => Box::new(CodexHookAdapter),
    }
}

pub(crate) fn provider_label(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => "Claude Code",
        Provider::Codex => "Codex CLI",
    }
}
