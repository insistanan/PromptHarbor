use super::{
    config::{claude_hook_command, read_settings_json, write_settings_json},
    detect_claude_user_hook, install_claude_user_hook,
};
use crate::hook_config::{has_promptbox_hook, has_stale_promptbox_hook};
use serde_json::json;
use std::{
    env, fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

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
