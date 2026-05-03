use serde_json::Value;

use super::expected_images::expected_image_count;

pub(super) fn prompt_may_have_image(prompt: &str) -> bool {
    expected_image_count(prompt) > 0
}

pub(super) fn json_may_have_image(value: &Value) -> bool {
    match value {
        Value::String(text) => {
            let text = text.trim();
            text.starts_with("data:image/") || text.starts_with("file://")
        }
        Value::Object(map) => map.iter().any(|(key, value)| {
            let normalized_key = key.to_ascii_lowercase();
            matches!(
                normalized_key.as_str(),
                "image"
                    | "images"
                    | "image_url"
                    | "imageurl"
                    | "image_path"
                    | "imagepath"
                    | "input_image"
                    | "inputimage"
                    | "media_type"
                    | "mediatype"
                    | "mime_type"
                    | "mimetype"
                    | "source"
                    | "file_path"
                    | "filepath"
            ) || (normalized_key == "type"
                && value
                    .as_str()
                    .is_some_and(|value| value.to_ascii_lowercase().contains("image")))
                || json_may_have_image(value)
        }),
        Value::Array(items) => items.iter().any(json_may_have_image),
        _ => false,
    }
}
