use super::PromptHistoryItem;
use crate::PromptEvent;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachment {
    pub id: i64,
    pub kind: String,
    pub mime_type: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub placeholder: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachmentDataUrl {
    pub id: i64,
    pub mime_type: String,
    pub data_url: String,
}

#[derive(Debug)]
struct ExtractedPromptImage {
    mime_type: String,
    bytes: Vec<u8>,
    source: String,
}

pub(super) fn store_prompt_event_attachments(
    connection: &Connection,
    event: &PromptEvent,
    prompt: &str,
    prompt_event_id: i64,
    attachment_root: &Path,
    created_at: &str,
) -> Result<(), String> {
    let images = extract_prompt_images(event, prompt);
    if images.is_empty() {
        return Ok(());
    }

    let provider = event.provider.as_str();
    let session_segment = sanitize_path_segment(&event.session_id);
    let attachment_dir = attachment_root.join(provider).join(session_segment);
    fs::create_dir_all(&attachment_dir).map_err(|error| {
        format!(
            "创建 prompt 图片附件目录失败：{}：{error}",
            attachment_dir.display()
        )
    })?;

    for (index, image) in images.into_iter().enumerate() {
        let position = (index + 1) as i64;
        let extension = extension_from_mime_type(&image.mime_type);
        let file_name = format!("{prompt_event_id}-{position}.{extension}");
        let file_path = attachment_dir.join(&file_name);
        fs::write(&file_path, &image.bytes).map_err(|error| {
            format!("写入 prompt 图片附件失败：{}：{error}", file_path.display())
        })?;
        let file_size = i64::try_from(image.bytes.len()).unwrap_or(i64::MAX);
        let file_path_text = file_path.to_string_lossy().into_owned();
        let placeholder = image_placeholder(prompt, position as usize);

        connection
            .execute(
                r#"
                insert or ignore into prompt_event_attachments (
                  prompt_event_id, provider, session_id, kind, mime_type,
                  file_path, file_name, file_size, placeholder, source, position, created_at
                )
                values (?1, ?2, ?3, 'image', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    prompt_event_id,
                    provider,
                    &event.session_id,
                    image.mime_type.as_str(),
                    file_path_text.as_str(),
                    file_name.as_str(),
                    file_size,
                    placeholder.as_deref(),
                    image.source.as_str(),
                    position,
                    created_at,
                ],
            )
            .map_err(|error| format!("写入 prompt 图片附件记录失败：{error}"))?;
    }

    Ok(())
}

fn extract_prompt_images(event: &PromptEvent, prompt: &str) -> Vec<ExtractedPromptImage> {
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

fn decode_image_data_url(value: &str) -> Option<(String, Vec<u8>)> {
    let (metadata, data) = value.split_once(',')?;
    let metadata = metadata.trim();
    if !metadata.starts_with("data:") || !metadata.ends_with(";base64") {
        return None;
    }

    let mime_type = metadata
        .trim_start_matches("data:")
        .trim_end_matches(";base64")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .trim();
    decode_base64_image(mime_type, data)
}

fn decode_base64_image(mime_type: &str, data: &str) -> Option<(String, Vec<u8>)> {
    let mime_type = normalize_image_mime_type(mime_type);
    if !mime_type.starts_with("image/") {
        return None;
    }
    let normalized_data = data
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let bytes = BASE64_STANDARD.decode(normalized_data).ok()?;
    if bytes.is_empty() {
        return None;
    }

    Some((mime_type, bytes))
}

fn normalize_image_mime_type(value: &str) -> String {
    let normalized = value
        .trim()
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_ascii_lowercase();
    if normalized.starts_with("image/") {
        normalized
    } else {
        "image/png".to_string()
    }
}

fn normalize_prompt_for_match(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn prompt_may_have_image(prompt: &str) -> bool {
    expected_image_count(prompt) > 0
}

pub(super) fn expected_image_count(prompt: &str) -> usize {
    image_placeholder_count(prompt, "[Image #") + image_placeholder_count(prompt, "[图片 #")
}

fn image_placeholder_count(prompt: &str, marker: &str) -> usize {
    let mut count = 0;
    let mut rest = prompt;
    while let Some(index) = rest.find(marker) {
        count += 1;
        rest = &rest[index + marker.len()..];
    }
    count
}

fn json_may_have_image(value: &Value) -> bool {
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

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "unknown-session".to_string()
    } else {
        sanitized
    }
}

fn image_placeholder(prompt: &str, position: usize) -> Option<String> {
    let english = format!("[Image #{position}]");
    if prompt.contains(&english) {
        return Some(english);
    }

    let chinese = format!("[图片 #{position}]");
    prompt.contains(&chinese).then_some(chinese)
}

fn extension_from_mime_type(mime_type: &str) -> &'static str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "png",
    }
}

pub(super) fn append_prompt_history_attachments(
    connection: &Connection,
    items: &mut [PromptHistoryItem],
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            r#"
            select id, kind, mime_type, file_path, file_name, file_size, placeholder, created_at
            from prompt_event_attachments
            where prompt_event_id = ?1
            order by position asc, id asc
            "#,
        )
        .map_err(|error| format!("准备读取 prompt 图片附件失败：{error}"))?;

    for item in items {
        let rows = statement
            .query_map(params![item.id], |row| {
                Ok(PromptAttachment {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    mime_type: row.get(2)?,
                    file_path: row.get(3)?,
                    file_name: row.get(4)?,
                    file_size: row.get(5)?,
                    placeholder: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|error| format!("读取 prompt 图片附件失败：{error}"))?;

        let mut attachments = Vec::new();
        for row in rows {
            attachments.push(row.map_err(|error| format!("解析 prompt 图片附件失败：{error}"))?);
        }

        item.expected_image_count = expected_image_count(&item.prompt_md);
        item.captured_image_count = attachments
            .iter()
            .filter(|attachment| attachment.kind == "image")
            .count();
        item.has_missing_images = item.expected_image_count > item.captured_image_count;
        item.attachments = attachments;
    }

    Ok(())
}

pub(super) fn read_prompt_attachment_data_url(
    connection: &Connection,
    attachment_id: i64,
) -> Result<PromptAttachmentDataUrl, String> {
    let (mime_type, file_path): (String, String) = connection
        .query_row(
            r#"
            select mime_type, file_path
            from prompt_event_attachments
            where id = ?1
            "#,
            params![attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| format!("读取 prompt 图片附件记录失败：{error}"))?
        .ok_or_else(|| format!("prompt 图片附件不存在：{attachment_id}"))?;

    let bytes = fs::read(&file_path)
        .map_err(|error| format!("读取 prompt 图片附件文件失败：{file_path}：{error}"))?;
    let encoded = BASE64_STANDARD.encode(bytes);

    Ok(PromptAttachmentDataUrl {
        id: attachment_id,
        mime_type: mime_type.clone(),
        data_url: format!("data:{mime_type};base64,{encoded}"),
    })
}

pub(super) fn session_attachment_files(
    connection: &Connection,
    session_db_id: i64,
) -> Result<Vec<PathBuf>, String> {
    let mut statement = connection
        .prepare(
            r#"
            select distinct prompt_event_attachments.file_path
            from prompt_event_attachments
            join prompt_events on prompt_events.id = prompt_event_attachments.prompt_event_id
            where prompt_events.session_db_id = ?1
            "#,
        )
        .map_err(|error| format!("准备读取会话附件文件失败：{error}"))?;
    let rows = statement
        .query_map(params![session_db_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("读取会话附件文件失败：{error}"))?;

    let mut files = Vec::new();
    for row in rows {
        files.push(PathBuf::from(
            row.map_err(|error| format!("解析会话附件文件失败：{error}"))?,
        ));
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provider;
    use serde_json::json;

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
