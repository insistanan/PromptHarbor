use super::extraction::extract_prompt_images;
use crate::PromptEvent;
use rusqlite::{params, Connection};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(in crate::store) fn store_prompt_event_attachments(
    connection: &Connection,
    event: &PromptEvent,
    prompt: &str,
    prompt_event_id: i64,
    attachment_root: &Path,
    created_at: &str,
) -> Result<usize, String> {
    let images = extract_prompt_images(event, prompt);
    if images.is_empty() {
        return Ok(0);
    }
    let image_count = images.len();

    let provider = event.provider.as_str();
    let attachment_dir = attachment_dir(attachment_root, provider, &event.session_id);
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

    Ok(image_count)
}

fn attachment_dir(attachment_root: &Path, provider: &str, session_id: &str) -> PathBuf {
    attachment_root
        .join(provider)
        .join(sanitize_path_segment(session_id))
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
