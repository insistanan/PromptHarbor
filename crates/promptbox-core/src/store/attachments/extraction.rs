use crate::PromptEvent;
use serde_json::Value;
use std::{
    fs::{self, File},
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

mod data_url;
mod expected_images;
mod image_clues;

use self::data_url::{decode_base64_image, decode_image_data_url};
use self::image_clues::{json_may_have_image, prompt_may_have_image};

pub(super) use self::expected_images::{expected_image_count, has_missing_images};

const MAX_IMAGE_FILE_BYTES: u64 = 32 * 1024 * 1024;

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
    let mut images = Vec::new();

    if prompt_may_have_image(prompt) || json_may_have_image(&event.raw_json) {
        collect_images_from_json_value(&event.raw_json, prompt, "hook_raw_json", false, &mut images);
    }

    if let Some(transcript_path) = event.transcript_path.as_deref() {
        append_images_from_transcript(transcript_path, prompt, &mut images);
    }

    images
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

fn append_images_from_transcript(
    transcript_path: &str,
    prompt: &str,
    images: &mut Vec<ExtractedPromptImage>,
) {
    let Ok(file) = File::open(transcript_path) else {
        return;
    };

    let reader = BufReader::new(file);
    let mut matched_prompt = false;
    let mut fallback_images = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if !json_may_have_image(&value) {
            continue;
        }

        let line_images = extract_images_from_json_value(&value, prompt, "hook_transcript");
        if line_images.is_empty() {
            continue;
        }

        if json_value_matches_prompt(&value, prompt) {
            matched_prompt = true;
            append_extracted_images(images, line_images);
            continue;
        }

        if json_value_has_user_message(&value) {
            fallback_images = line_images;
        }
    }

    if !matched_prompt && prompt_may_have_image(prompt) {
        append_extracted_images(images, fallback_images);
    }
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

fn json_value_has_user_message(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            object_is_user_message(value) || map.values().any(json_value_has_user_message)
        }
        Value::Array(items) => items.iter().any(json_value_has_user_message),
        _ => false,
    }
}

fn json_value_matches_prompt(value: &Value, prompt: &str) -> bool {
    match value {
        Value::String(text) => prompt_strings_match(text, prompt),
        Value::Object(map) => {
            string_value_at(value, &["prompt"])
                .is_some_and(|candidate| prompt_strings_match(candidate, prompt))
                || map
                    .get("content")
                    .and_then(Value::as_array)
                    .is_some_and(|content| content_matches_prompt(content, prompt))
                || map.values().any(|child| json_value_matches_prompt(child, prompt))
        }
        Value::Array(items) => items
            .iter()
            .any(|item| json_value_matches_prompt(item, prompt)),
        _ => false,
    }
}

fn prompt_strings_match(candidate: &str, prompt: &str) -> bool {
    let prompt = normalize_prompt_for_match(prompt);
    let candidate = normalize_prompt_for_match(candidate);
    if prompt.is_empty() || candidate.is_empty() {
        return false;
    }
    if candidate == prompt {
        return true;
    }

    let prompt_without_images = normalize_prompt_without_image_markup(&prompt);
    let candidate_without_images = normalize_prompt_without_image_markup(&candidate);
    !prompt_without_images.is_empty() && candidate_without_images == prompt_without_images
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
    if joined == prompt {
        return true;
    }

    let prompt_without_images = normalize_prompt_without_image_markup(&prompt);
    let joined_without_images = normalize_prompt_without_image_markup(&joined);
    !prompt_without_images.is_empty() && joined_without_images == prompt_without_images
}

fn content_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Object(map) => map
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| map.get("input_text").and_then(Value::as_str))
            .or_else(|| map.get("content").and_then(Value::as_str))
            .or_else(|| map.get("value").and_then(Value::as_str)),
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

fn append_extracted_images(
    images: &mut Vec<ExtractedPromptImage>,
    next_images: Vec<ExtractedPromptImage>,
) {
    for image in next_images {
        append_unique_image(images, image.mime_type, image.bytes, &image.source);
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
        if let Some(decoded) = decode_image_reference(image_url, mime_type_from_object(object)) {
            return Some(decoded);
        }
    }

    if let Some(image_url) = object.get("image_url").and_then(Value::as_object) {
        if let Some(decoded) = image_bytes_from_content_item(&Value::Object(image_url.clone())) {
            return Some(decoded);
        }
    }

    if let Some(url) = object.get("url").and_then(Value::as_str) {
        if let Some(decoded) = decode_image_reference(url, mime_type_from_object(object)) {
            return Some(decoded);
        }
    }

    if let Some(image) = object.get("image") {
        match image {
            Value::String(reference) => {
                if let Some(decoded) = decode_image_reference(reference, mime_type_from_object(object)) {
                    return Some(decoded);
                }
            }
            Value::Object(_) => {
                if let Some(decoded) = image_bytes_from_content_item(image) {
                    return Some(decoded);
                }
            }
            _ => {}
        }
    }

    if let Some(source) = object.get("source").and_then(Value::as_object) {
        if let Some(data) = source.get("data").and_then(Value::as_str) {
            let mime_type = mime_type_from_object(source).unwrap_or("image/png");
            if let Some(decoded) = decode_base64_image(mime_type, data) {
                return Some(decoded);
            }
        }
        if let Some(decoded) = image_bytes_from_content_item(&Value::Object(source.clone())) {
            return Some(decoded);
        }
    }

    if let Some(data) = object.get("data").and_then(Value::as_str) {
        let mime_type = mime_type_from_object(object).unwrap_or("image/png");
        if let Some(decoded) = decode_base64_image(mime_type, data) {
            return Some(decoded);
        }
    }

    if object_is_image_like(value) {
        for key in [
            "file_path",
            "filePath",
            "path",
            "image_path",
            "imagePath",
            "local_path",
            "localPath",
        ] {
            if let Some(reference) = object.get(key).and_then(Value::as_str) {
                if let Some(decoded) = decode_image_reference(reference, mime_type_from_object(object)) {
                    return Some(decoded);
                }
            }
        }
    }

    None
}

fn mime_type_from_object(object: &serde_json::Map<String, Value>) -> Option<&str> {
    object
        .get("media_type")
        .and_then(Value::as_str)
        .or_else(|| object.get("mediaType").and_then(Value::as_str))
        .or_else(|| object.get("mime_type").and_then(Value::as_str))
        .or_else(|| object.get("mimeType").and_then(Value::as_str))
}

fn object_is_image_like(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };

    if object
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|value| value.to_ascii_lowercase().contains("image"))
    {
        return true;
    }

    object.keys().any(|key| {
        let normalized = key.to_ascii_lowercase();
        normalized.contains("image")
            || matches!(
                normalized.as_str(),
                "media_type" | "mimetype" | "mime_type" | "file_path" | "filepath"
            )
    })
}

fn decode_image_reference(
    reference: &str,
    mime_type_hint: Option<&str>,
) -> Option<(String, Vec<u8>)> {
    decode_image_data_url(reference)
        .or_else(|| read_image_file_reference(reference, mime_type_hint))
}

fn read_image_file_reference(
    reference: &str,
    mime_type_hint: Option<&str>,
) -> Option<(String, Vec<u8>)> {
    let path = path_from_file_reference(reference)?;
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_IMAGE_FILE_BYTES
    {
        return None;
    }

    let bytes = fs::read(&path).ok()?;
    let mime_type = mime_type_hint
        .and_then(normalize_image_mime_type_hint)
        .or_else(|| infer_image_mime_type_from_path(&path))
        .or_else(|| infer_image_mime_type_from_bytes(&bytes))?;

    Some((mime_type, bytes))
}

fn path_from_file_reference(reference: &str) -> Option<PathBuf> {
    let reference = reference.trim();
    if reference.is_empty()
        || reference.starts_with("data:")
        || reference.starts_with("http://")
        || reference.starts_with("https://")
    {
        return None;
    }

    if let Some(path) = reference.strip_prefix("file://") {
        return path_from_file_url_body(path);
    }

    if looks_like_local_path(reference) {
        return Some(PathBuf::from(reference));
    }

    None
}

fn path_from_file_url_body(value: &str) -> Option<PathBuf> {
    let decoded = percent_decode(value);

    #[cfg(windows)]
    {
        let normalized = decoded.replace('/', "\\");
        if normalized.starts_with('\\') && normalized.len() > 3 && normalized.as_bytes()[2] == b':' {
            return Some(PathBuf::from(&normalized[1..]));
        }
        if !normalized.starts_with('\\') {
            if let Some((host, path)) = normalized.split_once('\\') {
                return Some(PathBuf::from(format!("\\\\{host}\\{path}")));
            }
        }
        return Some(PathBuf::from(normalized));
    }

    #[cfg(not(windows))]
    {
        Some(PathBuf::from(decoded))
    }
}

fn looks_like_local_path(value: &str) -> bool {
    let path = Path::new(value);
    if path.is_absolute() || value.starts_with("\\\\") || value.starts_with("//") {
        return true;
    }

    let bytes = value.as_bytes();
    bytes.len() > 2
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let (Some(high), Some(low)) = (hex_value(bytes[index + 1]), hex_value(bytes[index + 2])) {
                decoded.push((high << 4) | low);
                index += 3;
                continue;
            }
        }

        decoded.push(bytes[index]);
        index += 1;
    }

    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn normalize_image_mime_type_hint(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .split(';')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    normalized.starts_with("image/").then_some(normalized)
}

fn infer_image_mime_type_from_path(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let mime_type = match extension.as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => return None,
    };
    Some(mime_type.to_string())
}

fn infer_image_mime_type_from_bytes(bytes: &[u8]) -> Option<String> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some("image/png".to_string());
    }
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        return Some("image/jpeg".to_string());
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif".to_string());
    }
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp".to_string());
    }
    if bytes.starts_with(b"BM") {
        return Some("image/bmp".to_string());
    }
    None
}

fn normalize_prompt_for_match(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_prompt_without_image_markup(value: &str) -> String {
    let without_tags = strip_codex_image_tags(value);
    let without_placeholders = strip_image_placeholders(&without_tags);
    normalize_prompt_for_match(&without_placeholders)
}

fn strip_codex_image_tags(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut rest = value;

    while let Some(index) = rest.find("<image") {
        output.push_str(&rest[..index]);
        let tag_and_after = &rest[index..];
        let Some(end) = tag_and_after.find('>') else {
            output.push_str(tag_and_after);
            return output;
        };
        rest = &tag_and_after[end + 1..];
    }

    output.push_str(rest);
    output.replace("</image>", " ")
}

fn strip_image_placeholders(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut index = 0;

    while index < value.len() {
        let rest = &value[index..];
        let Some(start_offset) = rest.find('[') else {
            output.push_str(rest);
            break;
        };

        let start = index + start_offset;
        output.push_str(&value[index..start]);
        let marker = &value[start..];
        let is_image_marker =
            marker.starts_with("[Image #") || marker.starts_with("[图片 #");
        if is_image_marker {
            if let Some(end_offset) = marker.find(']') {
                index = start + end_offset + 1;
                output.push(' ');
                continue;
            }
        }

        output.push('[');
        index = start + 1;
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PromptEvent, Provider};
    use serde_json::{json, Value};
    use std::{env, fs, time::{SystemTime, UNIX_EPOCH}};

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

    #[test]
    fn extracts_codex_input_image_from_transcript() {
        let transcript_path = temp_file_path("codex-image-transcript.jsonl");
        let line = serde_json::to_string(&json!({
            "item": {
                "type": "message",
                "role": "user",
                "content": [
                    { "type": "input_text", "text": "<image name=[Image #1]>" },
                    {
                        "type": "input_image",
                        "image_url": "data:image/png;base64,aGVsbG8="
                    },
                    { "type": "input_text", "text": "</image>" },
                    { "type": "input_text", "text": "pasted image" }
                ]
            }
        }))
        .unwrap();
        fs::write(&transcript_path, format!("{line}\n")).unwrap();
        let event = test_event(json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "image-session",
            "prompt": "pasted image",
            "transcript_path": transcript_path.to_string_lossy()
        }));

        let images = extract_prompt_images(&event, "pasted image");

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].bytes.as_slice(), b"hello");
        let _ = fs::remove_file(transcript_path);
    }

    #[test]
    fn extracts_local_image_path_from_transcript() {
        let image_path = temp_file_path("prompt-image.png");
        let transcript_path = temp_file_path("local-image-transcript.jsonl");
        fs::write(&image_path, b"local-image").unwrap();
        let line = serde_json::to_string(&json!({
            "message": {
                "role": "user",
                "content": [
                    { "type": "text", "text": "看这张图 [Image #1]" },
                    {
                        "type": "input_image",
                        "path": image_path.to_string_lossy()
                    }
                ]
            }
        }))
        .unwrap();
        fs::write(&transcript_path, format!("{line}\n")).unwrap();
        let event = test_event(json!({
            "hook_event_name": "UserPromptSubmit",
            "session_id": "image-session",
            "prompt": "看这张图 [Image #1]",
            "transcript_path": transcript_path.to_string_lossy()
        }));

        let images = extract_prompt_images(&event, "看这张图 [Image #1]");

        assert_eq!(images.len(), 1);
        assert_eq!(images[0].mime_type, "image/png");
        assert_eq!(images[0].bytes.as_slice(), b"local-image");
        let _ = fs::remove_file(transcript_path);
        let _ = fs::remove_file(image_path);
    }

    fn test_event(raw_json: Value) -> PromptEvent {
        let prompt = raw_json
            .get("prompt")
            .and_then(Value::as_str)
            .unwrap_or("看这张图 [Image #1]")
            .to_string();
        let transcript_path = raw_json
            .get("transcript_path")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        PromptEvent {
            provider: Provider::Claude,
            event_name: "UserPromptSubmit".to_string(),
            session_id: "image-session".to_string(),
            turn_id: None,
            cwd: None,
            transcript_path,
            model: None,
            prompt: Some(prompt),
            captured_at: "2026-05-03T12:00:00.000Z".to_string(),
            raw_json,
        }
    }

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        env::temp_dir().join(format!("promptbox-{millis}-{name}"))
    }
}
