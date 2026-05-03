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
    let installed = has_promptbox_claude_hook(&root);

    Ok(ClaudeHookStatus {
        settings_path: path_to_string(&settings_path),
        expected_command,
        installed,
        readable: true,
        message: if installed {
            "Claude Code 用户级 hook 已安装".to_string()
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
        Some(backup_settings_file(&settings_path)?)
    } else {
        None
    };

    ensure_claude_hook_value(&mut root, &expected_command)?;
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

fn claude_user_settings_path() -> Result<PathBuf, String> {
    let home = env::var("USERPROFILE")
        .or_else(|_| env::var("HOME"))
        .map_err(|error| format!("无法定位用户目录以读取 Claude Code 配置：{error}"))?;
    Ok(PathBuf::from(home).join(".claude").join("settings.json"))
}

fn claude_hook_command(hook_path: &Path) -> String {
    format!("\"{}\" --provider claude", hook_path.to_string_lossy())
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

fn backup_settings_file(path: &Path) -> Result<PathBuf, String> {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let backup_path = path.with_file_name(format!("settings.json.promptbox.{timestamp}.bak"));
    fs::copy(path, &backup_path).map_err(|error| {
        format!(
            "备份 Claude Code 配置失败：{} -> {}：{error}",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(backup_path)
}

fn ensure_claude_hook_value(root: &mut Value, expected_command: &str) -> Result<(), String> {
    let object = root
        .as_object_mut()
        .ok_or_else(|| "Claude Code 配置根节点不是 JSON object".to_string())?;
    let hooks_value = object.entry("hooks").or_insert_with(|| json!({}));
    let hooks_object = hooks_value
        .as_object_mut()
        .ok_or_else(|| "Claude Code 配置中的 hooks 不是 JSON object".to_string())?;
    let user_prompt_submit = hooks_object
        .entry("UserPromptSubmit")
        .or_insert_with(|| json!([]));
    let user_prompt_submit_array = user_prompt_submit
        .as_array_mut()
        .ok_or_else(|| "Claude Code UserPromptSubmit hooks 不是 JSON array".to_string())?;

    if user_prompt_submit_array
        .iter()
        .any(has_promptbox_claude_hook)
    {
        return Ok(());
    }

    user_prompt_submit_array.push(json!({
        "hooks": [
            {
                "type": "command",
                "command": expected_command
            }
        ]
    }));

    Ok(())
}

fn has_promptbox_claude_hook(root: &Value) -> bool {
    match root {
        Value::String(value) => command_matches_promptbox_claude(value),
        Value::Array(items) => items.iter().any(has_promptbox_claude_hook),
        Value::Object(object) => object.values().any(has_promptbox_claude_hook),
        _ => false,
    }
}

fn command_matches_promptbox_claude(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("promptbox-hook") && lower.contains("--provider") && lower.contains("claude")
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
        assert!(updated.contains("promptbox-hook.exe"));
        assert!(updated.contains("--provider claude"));
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
