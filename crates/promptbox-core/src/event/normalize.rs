use super::{PromptEvent, Provider, MAX_HOOK_BODY_BYTES};
use serde_json::Value;
use std::{fs, path::PathBuf};

pub fn normalize_hook_input(
    provider: Provider,
    input: &str,
    captured_at: String,
) -> Result<PromptEvent, String> {
    if input.as_bytes().len() > MAX_HOOK_BODY_BYTES {
        return Err(format!(
            "hook 输入超过限制：{} bytes，大于 {} bytes",
            input.as_bytes().len(),
            MAX_HOOK_BODY_BYTES
        ));
    }

    let raw_json: Value =
        serde_json::from_str(input).map_err(|error| format!("解析 hook JSON 失败：{error}"))?;
    let event_name = string_field(&raw_json, "hook_event_name")
        .unwrap_or_else(|| "UserPromptSubmit".to_string());
    let session_id = string_field(&raw_json, "session_id")
        .ok_or_else(|| "hook JSON 缺少 session_id".to_string())?;

    Ok(PromptEvent {
        provider,
        event_name,
        session_id,
        turn_id: string_field(&raw_json, "turn_id"),
        cwd: path_field(&raw_json, "cwd"),
        transcript_path: path_field(&raw_json, "transcript_path"),
        model: string_field(&raw_json, "model"),
        prompt: string_field(&raw_json, "prompt"),
        captured_at,
        raw_json,
    })
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn path_field(value: &Value, key: &str) -> Option<String> {
    string_field(value, key).map(|path| normalize_path_string(&path))
}

fn normalize_path_string(value: &str) -> String {
    let path = PathBuf::from(value);
    let normalized = fs::canonicalize(&path).unwrap_or(path);
    trim_trailing_separator(&normalized.to_string_lossy())
}

fn trim_trailing_separator(value: &str) -> String {
    let mut normalized = value.to_string();
    while normalized.len() > 3 && (normalized.ends_with('\\') || normalized.ends_with('/')) {
        normalized.pop();
    }
    normalized
}
