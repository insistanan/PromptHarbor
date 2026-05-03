use crate::{
    event::Provider,
    hook_adapter::{
        provider_label, HookAdapterStatus, HookBackupPath, HookConfigPathStatus,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexHookStatus {
    pub hooks_path: String,
    pub config_path: String,
    pub expected_command: String,
    pub hook_installed: bool,
    pub codex_hooks_enabled: bool,
    pub ready: bool,
    pub message: String,
    pub hooks_backup_path: Option<String>,
    pub config_backup_path: Option<String>,
}

pub(in crate::codex) fn codex_status_to_adapter_status(
    status: CodexHookStatus,
) -> HookAdapterStatus {
    let mut backup_paths = Vec::new();
    if let Some(path) = status.hooks_backup_path {
        backup_paths.push(HookBackupPath {
            label: "hooks 备份".to_string(),
            path,
        });
    }
    if let Some(path) = status.config_backup_path {
        backup_paths.push(HookBackupPath {
            label: "config 备份".to_string(),
            path,
        });
    }

    HookAdapterStatus {
        provider: Provider::Codex.as_str().to_string(),
        provider_label: provider_label(Provider::Codex).to_string(),
        expected_command: status.expected_command,
        installed: status.hook_installed,
        ready: status.ready,
        readable: true,
        message: status.message,
        config_paths: vec![
            HookConfigPathStatus {
                label: "hooks.json".to_string(),
                path: status.hooks_path,
            },
            HookConfigPathStatus {
                label: "config.toml".to_string(),
                path: status.config_path,
            },
        ],
        backup_paths,
        codex_hooks_enabled: Some(status.codex_hooks_enabled),
    }
}
