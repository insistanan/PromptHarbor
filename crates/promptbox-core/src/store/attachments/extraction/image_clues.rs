use serde_json::Value;

use super::expected_images::expected_image_count;

pub(super) fn prompt_may_have_image(prompt: &str) -> bool {
    expected_image_count(prompt) > 0
}

pub(super) fn json_may_have_image(value: &Value) -> bool {
    match value {
        Value::String(text) => text.starts_with("data:image/"),
        Value::Object(map) => map.iter().any(|(key, value)| {
            matches!(
                key.as_str(),
                "image_url" | "media_type" | "mime_type" | "source"
            ) || json_may_have_image(value)
        }),
        Value::Array(items) => items.iter().any(json_may_have_image),
        _ => false,
    }
}
