use serde_json::{json, Value};

pub(crate) fn ensure_user_prompt_submit_hook(
    root: &mut Value,
    expected_command: &str,
    provider: &str,
    product_label: &str,
) -> Result<(), String> {
    let object = root
        .as_object_mut()
        .ok_or_else(|| format!("{product_label} 配置根节点不是 JSON object"))?;
    let hooks_value = object.entry("hooks").or_insert_with(|| json!({}));
    let hooks_object = hooks_value
        .as_object_mut()
        .ok_or_else(|| format!("{product_label} hooks 字段不是 JSON object"))?;
    let user_prompt_submit = hooks_object
        .entry("UserPromptSubmit")
        .or_insert_with(|| json!([]));
    let user_prompt_submit_array = user_prompt_submit
        .as_array_mut()
        .ok_or_else(|| format!("{product_label} UserPromptSubmit hooks 不是 JSON array"))?;

    let has_current = user_prompt_submit_array
        .iter()
        .any(|value| has_promptbox_hook(value, expected_command));
    let has_stale = user_prompt_submit_array
        .iter()
        .any(|value| has_stale_promptbox_hook(value, expected_command, provider));

    if has_current && !has_stale {
        return Ok(());
    }

    for value in user_prompt_submit_array.iter_mut() {
        remove_promptbox_hooks(value, provider);
    }
    user_prompt_submit_array.retain(|value| !empty_hook_entry(value));

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

pub(crate) fn has_promptbox_hook(root: &Value, expected_command: &str) -> bool {
    match root {
        Value::String(value) => command_matches_expected(value, expected_command),
        Value::Array(items) => items
            .iter()
            .any(|value| has_promptbox_hook(value, expected_command)),
        Value::Object(object) => object
            .values()
            .any(|value| has_promptbox_hook(value, expected_command)),
        _ => false,
    }
}

pub(crate) fn has_stale_promptbox_hook(
    root: &Value,
    expected_command: &str,
    provider: &str,
) -> bool {
    match root {
        Value::String(value) => {
            command_matches_promptbox(value, provider)
                && !command_matches_expected(value, expected_command)
        }
        Value::Array(items) => items
            .iter()
            .any(|value| has_stale_promptbox_hook(value, expected_command, provider)),
        Value::Object(object) => object
            .values()
            .any(|value| has_stale_promptbox_hook(value, expected_command, provider)),
        _ => false,
    }
}

pub(crate) fn remove_promptbox_hooks(root: &mut Value, provider: &str) {
    match root {
        Value::Array(items) => {
            for value in items.iter_mut() {
                remove_promptbox_hooks(value, provider);
            }
            items.retain(|value| {
                !is_promptbox_command_hook(value, provider) && !empty_hook_entry(value)
            });
        }
        Value::Object(object) => {
            for value in object.values_mut() {
                remove_promptbox_hooks(value, provider);
            }
        }
        _ => {}
    }
}

pub(crate) fn prune_empty_hooks_root(root: &mut Value) {
    let Some(object) = root.as_object_mut() else {
        return;
    };
    if let Some(hooks) = object.get_mut("hooks") {
        prune_empty_hook_containers(hooks);
    }
    if object
        .get("hooks")
        .is_some_and(|value| matches!(value, Value::Object(map) if map.is_empty()))
    {
        object.remove("hooks");
    }
}

fn prune_empty_hook_containers(root: &mut Value) {
    match root {
        Value::Object(object) => {
            for value in object.values_mut() {
                prune_empty_hook_containers(value);
            }
            object.retain(|_, value| !value_is_empty_container(value));
        }
        Value::Array(items) => {
            for value in items.iter_mut() {
                prune_empty_hook_containers(value);
            }
            items.retain(|value| !value_is_empty_container(value));
        }
        _ => {}
    }
}

fn value_is_empty_container(value: &Value) -> bool {
    matches!(value, Value::Array(items) if items.is_empty())
        || matches!(value, Value::Object(object) if object.is_empty())
}

fn is_promptbox_command_hook(value: &Value, provider: &str) -> bool {
    value
        .as_object()
        .and_then(|object| object.get("command"))
        .and_then(Value::as_str)
        .is_some_and(|command| command_matches_promptbox(command, provider))
}

fn empty_hook_entry(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    object
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
}

fn command_matches_expected(command: &str, expected_command: &str) -> bool {
    command.trim().eq_ignore_ascii_case(expected_command.trim())
}

fn command_matches_promptbox(command: &str, provider: &str) -> bool {
    let lower = command.to_ascii_lowercase();
    let provider = provider.to_ascii_lowercase();
    let generated_shape = if cfg!(windows) {
        lower.trim_start().starts_with('"')
            || (lower.contains("cmd /d /s /c") && lower.contains("exit /b 0"))
    } else {
        lower.trim_start().starts_with('"')
    };

    generated_shape
        && (lower.contains("promptbox-hook.exe") || lower.contains("promptbox-hook"))
        && lower.contains("--provider")
        && lower.contains(&provider)
}
