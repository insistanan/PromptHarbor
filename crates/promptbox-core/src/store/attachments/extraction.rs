use crate::PromptEvent;
use serde_json::Value;

mod data_url;
mod expected_images;
mod image_clues;

use self::data_url::{decode_base64_image, decode_image_data_url};
use self::image_clues::{json_may_have_image, prompt_may_have_image};

pub(super) use self::expected_images::{expected_image_count, has_missing_images};

#[derive(Debug)]
pub(super) struct ExtractedPromptImage {
    pub(super) mime_type: String,
    pub(super) bytes: Vec<u8>,
    pub(super) source: String,
}

pub(super) fn extract_prompt_images(
    event: &PromptEvent,
    prompt: &str,
) -> Vec<ExtractedPromptImage> {
    if !prompt_may_have_image(prompt) && !json_may_have_image(&event.raw_json) {
        return Vec::new();
    }

    extract_images_from_json_value(&event.raw_json, prompt, "hook_raw_json")
}

fn extract_images_from_json_value(
    value: &Value,
    prompt: &str,
    source: &str,
) -> Vec<ExtractedPromptImage> {
    let mut images = Vec::new();
    collect_images_from_json_value(value, prompt, source, false, &mut images);
    images
}

fn collect_images_from_json_value(
    value: &Value,
    prompt: &str,
    source: &str,
    user_context: bool,
    images: &mut Vec<ExtractedPromptImage>,
) {
    match value {
        Value::Object(map) => {
            append_image_from_value(value, source, images);

            let next_user_context = user_context || object_is_user_message(value);
            if let Some(content) = map.get("content").and_then(Value::as_array) {
                if next_user_context && content_matches_prompt(content, prompt) {
                    append_images_from_content(content, source, images);
                }
            }
            for child in map.values() {
                collect_images_from_json_value(child, prompt, source, next_user_context, images);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_images_from_json_value(item, prompt, source, user_context, images);
            }
        }
        _ => {}
    }
}

fn object_is_user_message(value: &Value) -> bool {
    string_value_at(value, &["role"]).is_some_and(is_user_marker)
        || string_value_at(value, &["type"]).is_some_and(is_user_marker)
        || string_value_at(value, &["message", "role"]).is_some_and(is_user_marker)
        || string_value_at(value, &["message", "type"]).is_some_and(is_user_marker)
        || string_value_at(value, &["payload", "role"]).is_some_and(is_user_marker)
        || string_value_at(value, &["payload", "type"]).is_some_and(is_user_marker)
}

fn is_user_marker(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "user" | "user_message" | "input"
    )
}

fn string_value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn content_matches_prompt(content: &[Value], prompt: &str) -> bool {
    let prompt = normalize_prompt_for_match(prompt);
    if prompt.is_empty() {
        return false;
    }

    let texts = content
        .iter()
        .filter_map(content_text)
        .map(normalize_prompt_for_match)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();

    if texts.iter().any(|text| text == &prompt) {
        return true;
    }

    let joined = normalize_prompt_for_match(&texts.join("\n"));
    joined == prompt
}

fn content_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Object(map) => map
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| map.get("input_text").and_then(Value::as_str)),
        _ => None,
    }
}

fn append_images_from_content(
    content: &[Value],
    source: &str,
    images: &mut Vec<ExtractedPromptImage>,
) {
    for item in content {
        append_image_from_value(item, source, images);
    }
}

fn append_image_from_value(value: &Value, source: &str, images: &mut Vec<ExtractedPromptImage>) {
    if let Some((mime_type, bytes)) = image_bytes_from_content_item(value) {
        append_unique_image(images, mime_type, bytes, source);
    }
}

fn append_unique_image(
    images: &mut Vec<ExtractedPromptImage>,
    mime_type: String,
    bytes: Vec<u8>,
    source: &str,
) {
    if images
        .iter()
        .any(|image| {
            image.mime_type.as_str() == mime_type.as_str()
                && image.bytes.as_slice() == bytes.as_slice()
        })
    {
        return;
    }

    images.push(ExtractedPromptImage {
        mime_type,
        bytes,
        source: source.to_string(),
    });
}

fn image_bytes_from_content_item(value: &Value) -> Option<(String, Vec<u8>)> {
    let object = value.as_object()?;

    if let Some(image_url) = object.get("image_url").and_then(Value::as_str) {
        if let Some(decoded) = decode_image_data_url(image_url) {
            return Some(decoded);
        }
    }

    if let Some(image_url) = object.get("image_url").and_then(Value::as_object) {
        if let Some(url) = image_url.get("url").and_then(Value::as_str) {
            if let Some(decoded) = decode_image_data_url(url) {
                return Some(decoded);
            }
        }
    }

    if let Some(url) = object.get("url").and_then(Value::as_str) {
        if let Some(decoded) = decode_image_data_url(url) {
            return Some(decoded);
        }
    }

    if let Some(source) = object.get("source").and_then(Value::as_object) {
        let data = source.get("data").and_then(Value::as_str)?;
        let mime_type = source
            .get("media_type")
            .and_then(Value::as_str)
            .or_else(|| source.get("mime_type").and_then(Value::as_str))
            .unwrap_or("image/png");
        return decode_base64_image(mime_type, data);
    }

    if let Some(data) = object.get("data").and_then(Value::as_str) {
        let mime_type = object
            .get("media_type")
            .and_then(Value::as_str)
            .or_else(|| object.get("mime_type").and_then(Value::as_str))
            .unwrap_or("image/png");
        return decode_base64_image(mime_type, data);
    }

    None
}

fn normalize_prompt_for_match(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PromptEvent, Provider};
    use serde_json::{json, Value};

    #[test]
    fn extracts_image_url_object_from_hook_event() {
        let event = test_event(json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "image-session",
            "prompt": "看这张图 [Image #1]",
            "image_url": {
                "url": "data:image/png;base64,aGVsbG8="
            }
        }));

        let images = extract_prompt_images(&event, "看这张图 [Image #1]");

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].mime_type, "image/png");
        assert_eq!(images[0].bytes.as_slice(), b"hello");
    }

    #[test]
    fn deduplicates_image_seen_in_user_content() {
        let event = test_event(json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "image-session",
            "prompt": "看这张图 [Image #1]",
            "role": "user",
            "content": [
                { "type": "text", "text": "看这张图 [Image #1]" },
                {
                    "type": "image",
                    "source": {
                        "media_type": "image/png",
                        "data": "aGVsbG8="
                    }
                }
            ]
        }));

        let images = extract_prompt_images(&event, "看这张图 [Image #1]");

        assert_eq!(images.len(), 1);
    }

    fn test_event(raw_json: Value) -> PromptEvent {
        PromptEvent {
            provider: Provider::Claude,
            event_name: "UserPromptSubmit".to_string(),
            session_id: "image-session".to_string(),
            turn_id: None,
            cwd: None,
            transcript_path: None,
            model: None,
            prompt: Some("看这张图 [Image #1]".to_string()),
            captured_at: "2026-05-03T12:00:00.000Z".to_string(),
            raw_json,
        }
    }
}
