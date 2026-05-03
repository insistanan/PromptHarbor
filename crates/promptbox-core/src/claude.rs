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
pub struct ClaudeHookStatus {
    pub settings_path: String,
    pub expected_command: String,
    pub installed: bool,
    pub readable: bool,
    pub message: String,
    pub backup_path: Option<String>,
}

pub fn detect_claude_user_hook(hook_path: &Path) -> Result<ClaudeHookStatus, String> {
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

pub fn install_claude_user_hook(hook_path: &Path) -> Result<ClaudeHookStatus, String> {
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

pub fn uninstall_claude_user_hook(hook_path: &Path) -> Result<ClaudeHookStatus, String> {
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

fn claude_user_settings_path() -> Result<PathBuf, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|error| format!("无法定位用户目录以读取 Claude Code 配置：{error}"))?;
    Ok(PathBuf::from(home).join(".claude").join("settings.json"))
}

fn claude_hook_command(hook_path: &Path) -> String {
    hook_command(hook_path, "claude")
}

fn read_settings_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取 Claude Code 配置失败：{}：{error}", path.display()))?;
    serde_json::from_str(&raw)
        .map_err(|error| format!("解析 Claude Code 配置失败：{}：{error}", path.display()))
}

fn write_settings_json(path: &Path, root: &Value) -> Result<(), String> {
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

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn install_user_hook_backs_up_and_preserves_existing_hooks() {
        let home = isolated_home("claude-hook");
        let settings_dir = home.join(".claude");
        fs::create_dir_all(&settings_dir).unwrap();
        let settings_path = settings_dir.join("settings.json");
        fs::write(
            &settings_path,
            r#"{
              "theme": "dark",
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
        env::set_var("USERPROFILE", &home);

        let hook_path = home
            .join("PromptBox")
            .join("bin")
            .join("promptbox-hook.exe");
        let installed = install_claude_user_hook(&hook_path).unwrap();
        let detected = detect_claude_user_hook(&hook_path).unwrap();
        let updated = fs::read_to_string(&settings_path).unwrap();

        assert!(installed.installed);
        assert!(detected.installed);
        assert!(installed.backup_path.is_some());
        assert!(PathBuf::from(installed.backup_path.unwrap()).exists());
        assert!(updated.contains("echo existing"));
        if cfg!(windows) {
            assert!(updated.contains("cmd /d /s /c"));
            assert!(updated.contains("exit /b 0"));
        }
        assert!(updated.contains("promptbox-hook.exe"));
        assert!(updated.contains("--provider claude"));
    }

    #[test]
    fn stale_promptbox_hook_path_is_replaced() {
        let home = isolated_home("claude-stale-hook");
        let settings_dir = home.join(".claude");
        fs::create_dir_all(&settings_dir).unwrap();
        let settings_path = settings_dir.join("settings.json");
        let old_hook_path = home.join("old").join("bin").join("promptbox-hook.exe");
        let current_hook_path = home
            .join("PromptBox")
            .join("bin")
            .join("promptbox-hook.exe");
        let stale_command = claude_hook_command(&old_hook_path);
        let current_command = claude_hook_command(&current_hook_path);
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
        write_settings_json(&settings_path, &root).unwrap();
        env::set_var("USERPROFILE", &home);

        let detected = detect_claude_user_hook(&current_hook_path).unwrap();
        assert!(!detected.installed);
        assert!(detected.message.contains("不一致"));

        install_claude_user_hook(&current_hook_path).unwrap();
        let updated = read_settings_json(&settings_path).unwrap();

        assert!(has_promptbox_hook(&updated, &current_command));
        assert!(!has_stale_promptbox_hook(
            &updated,
            &current_command,
            "claude"
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
