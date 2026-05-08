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

pub(crate) use status::CodexHookStatus;

use config::{
    codex_hook_command, codex_hooks_enabled, codex_status_message, codex_user_paths,
    ensure_codex_hooks_enabled, path_to_string, read_json, read_toml, write_json, write_toml,
};
use status::codex_status_to_adapter_status;

#[cfg(test)]
mod tests;

pub(crate) struct CodexHookAdapter;

impl HookAdapter for CodexHookAdapter {
    fn detect(&self, hook_path: &Path) -> Result<HookAdapterStatus, String> {
        detect_codex_user_hook(hook_path).map(codex_status_to_adapter_status)
    }

    fn install(&self, hook_path: &Path) -> Result<HookAdapterStatus, String> {
        install_codex_user_hook(hook_path).map(codex_status_to_adapter_status)
    }

    fn uninstall(&self, hook_path: &Path) -> Result<HookAdapterStatus, String> {
        uninstall_codex_user_hook(hook_path).map(codex_status_to_adapter_status)
    }
}

pub(crate) fn detect_codex_user_hook(hook_path: &Path) -> Result<CodexHookStatus, String> {
    let paths = codex_user_paths()?;
    let expected_command = codex_hook_command(hook_path);
    let hook_installed = if paths.hooks_path.exists() {
        let root = read_json(&paths.hooks_path)?;
        has_promptbox_hook(&root, &expected_command)
            && !has_stale_promptbox_hook(&root, &expected_command, "codex")
    } else {
        false
    };
    let codex_hooks_enabled = if paths.config_path.exists() {
        let root = read_toml(&paths.config_path)?;
        codex_hooks_enabled(&root)
    } else {
        false
    };

    Ok(CodexHookStatus {
        hooks_path: path_to_string(&paths.hooks_path),
        config_path: path_to_string(&paths.config_path),
        expected_command,
        hook_installed,
        codex_hooks_enabled,
        ready: hook_installed && codex_hooks_enabled,
        message: codex_status_message(hook_installed, codex_hooks_enabled),
        hooks_backup_path: None,
        config_backup_path: None,
    })
}

pub(crate) fn install_codex_user_hook(hook_path: &Path) -> Result<CodexHookStatus, String> {
    let paths = codex_user_paths()?;
    let expected_command = codex_hook_command(hook_path);

    let mut hooks_root = if paths.hooks_path.exists() {
        read_json(&paths.hooks_path)?
    } else {
        json!({})
    };
    let mut config_root = if paths.config_path.exists() {
        read_toml(&paths.config_path)?
    } else {
        toml::Value::Table(toml::map::Map::new())
    };

    let hooks_backup_path = if paths.hooks_path.exists() {
        Some(backup_config_file(&paths.hooks_path, "Codex CLI")?)
    } else {
        None
    };
    let config_backup_path = if paths.config_path.exists() {
        Some(backup_config_file(&paths.config_path, "Codex CLI")?)
    } else {
        None
    };

    ensure_user_prompt_submit_hook(&mut hooks_root, &expected_command, "codex", "Codex CLI")?;
    ensure_codex_hooks_enabled(&mut config_root)?;
    write_json(&paths.hooks_path, &hooks_root)?;
    write_toml(&paths.config_path, &config_root)?;

    Ok(CodexHookStatus {
        hooks_path: path_to_string(&paths.hooks_path),
        config_path: path_to_string(&paths.config_path),
        expected_command,
        hook_installed: true,
        codex_hooks_enabled: true,
        ready: true,
        message: "Codex CLI 用户级 hook 已安装，hooks 已开启；已运行的 Codex CLI 需要新开窗口后生效"
            .to_string(),
        hooks_backup_path: hooks_backup_path.as_ref().map(|path| path_to_string(path)),
        config_backup_path: config_backup_path.as_ref().map(|path| path_to_string(path)),
    })
}

pub(crate) fn uninstall_codex_user_hook(hook_path: &Path) -> Result<CodexHookStatus, String> {
    let paths = codex_user_paths()?;
    let expected_command = codex_hook_command(hook_path);

    let mut hooks_backup_path = None;
    let mut hook_removed = false;
    if paths.hooks_path.exists() {
        let mut hooks_root = read_json(&paths.hooks_path)?;
        hook_removed = has_promptbox_hook(&hooks_root, &expected_command)
            || has_stale_promptbox_hook(&hooks_root, &expected_command, "codex");
        hooks_backup_path = Some(backup_config_file(&paths.hooks_path, "Codex CLI")?);
        remove_promptbox_hooks(&mut hooks_root, "codex");
        prune_empty_hooks_root(&mut hooks_root);
        write_json(&paths.hooks_path, &hooks_root)?;
    }

    let codex_hooks_enabled = if paths.config_path.exists() {
        let config_root = read_toml(&paths.config_path)?;
        codex_hooks_enabled(&config_root)
    } else {
        false
    };

    Ok(CodexHookStatus {
        hooks_path: path_to_string(&paths.hooks_path),
        config_path: path_to_string(&paths.config_path),
        expected_command,
        hook_installed: false,
        codex_hooks_enabled,
        ready: false,
        message: if hook_removed {
            "Codex CLI PromptHarbor hook 已取消；hooks 开关保持原样，其他 hook 不受影响"
                .to_string()
        } else {
            "未发现 Codex CLI PromptHarbor hook；hooks 开关保持原样".to_string()
        },
        hooks_backup_path: hooks_backup_path.as_ref().map(|path| path_to_string(path)),
        config_backup_path: None,
    })
}
