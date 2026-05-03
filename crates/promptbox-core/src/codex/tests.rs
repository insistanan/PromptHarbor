use super::{
    config::{codex_hook_command, read_json, write_json},
    detect_codex_user_hook, install_codex_user_hook,
};
use crate::hook_config::{has_promptbox_hook, has_stale_promptbox_hook};
use serde_json::json;
use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

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
