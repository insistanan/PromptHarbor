use crate::hook_config::{
    backup_config_file, ensure_user_prompt_submit_hook, has_promptbox_hook,
    has_stale_promptbox_hook, hook_command, prune_empty_hooks_root, remove_promptbox_hooks,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexHookStatus {
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

pub fn detect_codex_user_hook(hook_path: &Path) -> Result<CodexHookStatus, String> {
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

pub fn install_codex_user_hook(hook_path: &Path) -> Result<CodexHookStatus, String> {
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
        message: "Codex CLI 用户级 hook 已安装，codex_hooks 已开启；已运行的 Codex CLI 需要新开窗口后生效"
            .to_string(),
        hooks_backup_path: hooks_backup_path.as_ref().map(|path| path_to_string(path)),
        config_backup_path: config_backup_path.as_ref().map(|path| path_to_string(path)),
    })
}

pub fn uninstall_codex_user_hook(hook_path: &Path) -> Result<CodexHookStatus, String> {
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
            "Codex CLI PromptHarbor hook 已取消；codex_hooks 开关保持原样，其他 hook 不受影响"
                .to_string()
        } else {
            "未发现 Codex CLI PromptHarbor hook；codex_hooks 开关保持原样".to_string()
        },
        hooks_backup_path: hooks_backup_path.as_ref().map(|path| path_to_string(path)),
        config_backup_path: None,
    })
}

struct CodexUserPaths {
    hooks_path: PathBuf,
    config_path: PathBuf,
}

fn codex_user_paths() -> Result<CodexUserPaths, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|error| format!("无法定位用户目录以读取 Codex CLI 配置：{error}"))?;
    let codex_dir = PathBuf::from(home).join(".codex");
    Ok(CodexUserPaths {
        hooks_path: codex_dir.join("hooks.json"),
        config_path: codex_dir.join("config.toml"),
    })
}

fn codex_hook_command(hook_path: &Path) -> String {
    hook_command(hook_path, "codex")
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取 Codex CLI hooks 配置失败：{}：{error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("解析 Codex CLI hooks 配置失败：{}：{error}", path.display()))
}

fn write_json(path: &Path, root: &Value) -> Result<(), String> {
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

fn read_toml(path: &Path) -> Result<toml::Value, String> {
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

fn write_toml(path: &Path, root: &toml::Value) -> Result<(), String> {
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

fn ensure_codex_hooks_enabled(root: &mut toml::Value) -> Result<(), String> {
    let table = root
        .as_table_mut()
        .ok_or_else(|| "Codex CLI config.toml 根节点不是 TOML table".to_string())?;
    let features = table
        .entry("features")
        .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
    let features_table = features
        .as_table_mut()
        .ok_or_else(|| "Codex CLI config.toml 中的 [features] 不是 table".to_string())?;
    features_table.insert("codex_hooks".to_string(), toml::Value::Boolean(true));
    Ok(())
}

fn codex_hooks_enabled(root: &toml::Value) -> bool {
    root.get("features")
        .and_then(|features| features.get("codex_hooks"))
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
}

fn codex_status_message(hook_installed: bool, codex_hooks_enabled: bool) -> String {
    match (hook_installed, codex_hooks_enabled) {
        (true, true) => {
            "Codex CLI 用户级 hook 已安装，codex_hooks 已开启；已运行的 Codex CLI 需要新开窗口后生效"
                .to_string()
        }
        (true, false) => "Codex CLI hook 已安装，但 codex_hooks 尚未开启".to_string(),
        (false, true) => "codex_hooks 已开启，但 Codex CLI hook 未安装".to_string(),
        (false, false) => "Codex CLI hook 未安装，codex_hooks 尚未开启".to_string(),
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn install_user_hook_enables_feature_and_preserves_existing_config() {
        let home = isolated_home("codex-hook");
        let codex_dir = home.join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let hooks_path = codex_dir.join("hooks.json");
        let config_path = codex_dir.join("config.toml");
        fs::write(
            &hooks_path,
            r#"{
              "hooks": {
                "UserPromptSubmit": [
                  {
                    "hooks": [
                      {
                        "type": "command",
                        "command": "echo existing"
                      }
                    ]
                  }
                ]
              }
            }"#,
        )
        .unwrap();
        fs::write(
            &config_path,
            "model = \"gpt-test\"\n\n[features]\nother = true\n",
        )
        .unwrap();
        env::set_var("USERPROFILE", &home);

        let hook_path = home
            .join("PromptBox")
            .join("bin")
            .join("promptbox-hook.exe");
        let installed = install_codex_user_hook(&hook_path).unwrap();
        let detected = detect_codex_user_hook(&hook_path).unwrap();
        let hooks = fs::read_to_string(&hooks_path).unwrap();
        let config = fs::read_to_string(&config_path).unwrap();

        assert!(installed.ready);
        assert!(detected.ready);
        assert!(installed.hooks_backup_path.is_some());
        assert!(installed.config_backup_path.is_some());
        assert!(hooks.contains("echo existing"));
        if cfg!(windows) {
            assert!(hooks.contains("cmd /d /s /c"));
            assert!(hooks.contains("exit /b 0"));
        }
        assert!(hooks.contains("promptbox-hook.exe"));
        assert!(hooks.contains("--provider codex"));
        assert!(config.contains("model = \"gpt-test\""));
        assert!(config.contains("other = true"));
        assert!(config.contains("codex_hooks = true"));
    }

    #[test]
    fn stale_promptbox_hook_path_is_replaced() {
        let home = isolated_home("codex-stale-hook");
        let codex_dir = home.join(".codex");
        fs::create_dir_all(&codex_dir).unwrap();
        let hooks_path = codex_dir.join("hooks.json");
        let config_path = codex_dir.join("config.toml");
        let old_hook_path = home.join("old").join("bin").join("promptbox-hook.exe");
        let current_hook_path = home
            .join("PromptBox")
            .join("bin")
            .join("promptbox-hook.exe");
        let stale_command = codex_hook_command(&old_hook_path);
        let current_command = codex_hook_command(&current_hook_path);
        let root = json!({
            "hooks": {
                "UserPromptSubmit": [
                    {
                        "hooks": [
                            {
                                "type": "command",
                                "command": stale_command
                            },
                            {
                                "type": "command",
                                "command": "echo existing"
                            }
                        ]
                    }
                ]
            }
        });
        write_json(&hooks_path, &root).unwrap();
        fs::write(&config_path, "[features]\ncodex_hooks = true\n").unwrap();
        env::set_var("USERPROFILE", &home);

        let detected = detect_codex_user_hook(&current_hook_path).unwrap();
        assert!(!detected.hook_installed);
        assert!(!detected.ready);

        install_codex_user_hook(&current_hook_path).unwrap();
        let updated = read_json(&hooks_path).unwrap();

        assert!(has_promptbox_hook(&updated, &current_command));
        assert!(!has_stale_promptbox_hook(
            &updated,
            &current_command,
            "codex"
        ));
        assert!(serde_json::to_string(&updated)
            .unwrap()
            .contains("echo existing"));
    }

    fn isolated_home(name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = env::temp_dir().join(format!("promptbox-{name}-{millis}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
