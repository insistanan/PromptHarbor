use crate::{
    hook_adapter::{HookAdapter, HookAdapterStatus},
    hook_config::{
        backup_config_file, ensure_user_prompt_submit_hook, has_promptbox_hook,
        has_stale_promptbox_hook, prune_empty_hooks_root, remove_promptbox_hooks,
    },
};
use serde_json::json;
use std::path::Path;

mod config;
mod status;

pub(crate) use status::ClaudeHookStatus;

use config::{
    claude_hook_command, claude_user_settings_path, path_to_string, read_settings_json,
    write_settings_json,
};
use status::claude_status_to_adapter_status;

#[cfg(test)]
mod tests;

pub(crate) struct ClaudeHookAdapter;

impl HookAdapter for ClaudeHookAdapter {
    fn detect(&self, hook_path: &Path) -> Result<HookAdapterStatus, String> {
        detect_claude_user_hook(hook_path).map(claude_status_to_adapter_status)
    }

    fn install(&self, hook_path: &Path) -> Result<HookAdapterStatus, String> {
        install_claude_user_hook(hook_path).map(claude_status_to_adapter_status)
    }

    fn uninstall(&self, hook_path: &Path) -> Result<HookAdapterStatus, String> {
        uninstall_claude_user_hook(hook_path).map(claude_status_to_adapter_status)
    }
}

pub(crate) fn detect_claude_user_hook(hook_path: &Path) -> Result<ClaudeHookStatus, String> {
    let settings_path = claude_user_settings_path()?;
    let expected_command = claude_hook_command(hook_path);

    if !settings_path.exists() {
        return Ok(ClaudeHookStatus {
            settings_path: path_to_string(&settings_path),
            expected_command,
            installed: false,
            readable: true,
            message: "Claude Code 用户级 settings.json 尚未创建".to_string(),
            backup_path: None,
        });
    }

    let root = read_settings_json(&settings_path)?;
    let has_current = has_promptbox_hook(&root, &expected_command);
    let has_stale = has_stale_promptbox_hook(&root, &expected_command, "claude");
    let installed = has_current && !has_stale;

    Ok(ClaudeHookStatus {
        settings_path: path_to_string(&settings_path),
        expected_command,
        installed,
        readable: true,
        message: if installed {
            "Claude Code 用户级 hook 已安装".to_string()
        } else if has_current || has_stale {
            "Claude Code 用户级 hook 路径与当前 PromptBox home 不一致，请重新安装".to_string()
        } else {
            "Claude Code 用户级 hook 未安装".to_string()
        },
        backup_path: None,
    })
}

pub(crate) fn install_claude_user_hook(hook_path: &Path) -> Result<ClaudeHookStatus, String> {
    let settings_path = claude_user_settings_path()?;
    let expected_command = claude_hook_command(hook_path);
    let mut root = if settings_path.exists() {
        read_settings_json(&settings_path)?
    } else {
        json!({})
    };

    let backup_path = if settings_path.exists() {
        Some(backup_config_file(&settings_path, "Claude Code")?)
    } else {
        None
    };

    ensure_user_prompt_submit_hook(&mut root, &expected_command, "claude", "Claude Code")?;
    write_settings_json(&settings_path, &root)?;

    Ok(ClaudeHookStatus {
        settings_path: path_to_string(&settings_path),
        expected_command,
        installed: true,
        readable: true,
        message: "Claude Code 用户级 hook 已安装".to_string(),
        backup_path: backup_path.as_ref().map(|path| path_to_string(path)),
    })
}

pub(crate) fn uninstall_claude_user_hook(hook_path: &Path) -> Result<ClaudeHookStatus, String> {
    let settings_path = claude_user_settings_path()?;
    let expected_command = claude_hook_command(hook_path);

    if !settings_path.exists() {
        return Ok(ClaudeHookStatus {
            settings_path: path_to_string(&settings_path),
            expected_command,
            installed: false,
            readable: true,
            message: "Claude Code 用户级 settings.json 尚未创建，无需取消 hook".to_string(),
            backup_path: None,
        });
    }

    let mut root = read_settings_json(&settings_path)?;
    let had_promptbox_hook = has_promptbox_hook(&root, &expected_command)
        || has_stale_promptbox_hook(&root, &expected_command, "claude");
    let backup_path = Some(backup_config_file(&settings_path, "Claude Code")?);

    remove_promptbox_hooks(&mut root, "claude");
    prune_empty_hooks_root(&mut root);
    write_settings_json(&settings_path, &root)?;

    Ok(ClaudeHookStatus {
        settings_path: path_to_string(&settings_path),
        expected_command,
        installed: false,
        readable: true,
        message: if had_promptbox_hook {
            "Claude Code 用户级 PromptHarbor hook 已取消，其他 hook 已保留".to_string()
        } else {
            "未发现 Claude Code 用户级 PromptHarbor hook，配置未破坏".to_string()
        },
        backup_path: backup_path.as_ref().map(|path| path_to_string(path)),
    })
}
