use crate::{
    event::Provider,
    hook_adapter::{
        provider_label, HookAdapterStatus, HookBackupPath, HookConfigPathStatus,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeHookStatus {
    pub settings_path: String,
    pub expected_command: String,
    pub installed: bool,
    pub readable: bool,
    pub message: String,
    pub backup_path: Option<String>,
}

pub(in crate::claude) fn claude_status_to_adapter_status(
    status: ClaudeHookStatus,
) -> HookAdapterStatus {
    HookAdapterStatus {
        provider: Provider::Claude.as_str().to_string(),
        provider_label: provider_label(Provider::Claude).to_string(),
        expected_command: status.expected_command,
        installed: status.installed,
        ready: status.installed,
        readable: status.readable,
        message: status.message,
        config_paths: vec![HookConfigPathStatus {
            label: "settings.json".to_string(),
            path: status.settings_path,
        }],
        backup_paths: status
            .backup_path
            .into_iter()
            .map(|path| HookBackupPath {
                label: "settings 备份".to_string(),
                path,
            })
            .collect(),
        codex_hooks_enabled: None,
    }
}
