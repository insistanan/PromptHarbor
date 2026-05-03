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
        has_promptbox_codex_hook(&root)
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
        Some(backup_file(&paths.hooks_path)?)
    } else {
        None
    };
    let config_backup_path = if paths.config_path.exists() {
        Some(backup_file(&paths.config_path)?)
    } else {
        None
    };

    ensure_codex_hook_value(&mut hooks_root, &expected_command)?;
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
        message: "Codex CLI 用户级 hook 已安装，codex_hooks 已开启".to_string(),
        hooks_backup_path: hooks_backup_path.as_ref().map(|path| path_to_string(path)),
        config_backup_path: config_backup_path.as_ref().map(|path| path_to_string(path)),
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
    format!("\"{}\" --provider codex", hook_path.to_string_lossy())
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

fn backup_file(path: &Path) -> Result<PathBuf, String> {
    let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config");
    let backup_path = path.with_file_name(format!("{file_name}.promptbox.{timestamp}.bak"));
    fs::copy(path, &backup_path).map_err(|error| {
        format!(
            "备份 Codex CLI 配置失败：{} -> {}：{error}",
            path.display(),
            backup_path.display()
        )
    })?;
    Ok(backup_path)
}

fn ensure_codex_hook_value(root: &mut Value, expected_command: &str) -> Result<(), String> {
    let object = root
        .as_object_mut()
        .ok_or_else(|| "Codex CLI hooks.json 根节点不是 JSON object".to_string())?;
    let hooks_value = object.entry("hooks").or_insert_with(|| json!({}));
    let hooks_object = hooks_value
        .as_object_mut()
        .ok_or_else(|| "Codex CLI hooks 字段不是 JSON object".to_string())?;
    let user_prompt_submit = hooks_object
        .entry("UserPromptSubmit")
        .or_insert_with(|| json!([]));
    let user_prompt_submit_array = user_prompt_submit
        .as_array_mut()
        .ok_or_else(|| "Codex CLI UserPromptSubmit hooks 不是 JSON array".to_string())?;

    if user_prompt_submit_array
        .iter()
        .any(has_promptbox_codex_hook)
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

fn has_promptbox_codex_hook(root: &Value) -> bool {
    match root {
        Value::String(value) => command_matches_promptbox_codex(value),
        Value::Array(items) => items.iter().any(has_promptbox_codex_hook),
        Value::Object(object) => object.values().any(has_promptbox_codex_hook),
        _ => false,
    }
}

fn command_matches_promptbox_codex(command: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    lower.contains("promptbox-hook") && lower.contains("--provider") && lower.contains("codex")
}

fn codex_status_message(hook_installed: bool, codex_hooks_enabled: bool) -> String {
    match (hook_installed, codex_hooks_enabled) {
        (true, true) => "Codex CLI 用户级 hook 已安装，codex_hooks 已开启".to_string(),
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
        assert!(hooks.contains("promptbox-hook.exe"));
        assert!(hooks.contains("--provider codex"));
        assert!(config.contains("model = \"gpt-test\""));
        assert!(config.contains("other = true"));
        assert!(config.contains("codex_hooks = true"));
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
