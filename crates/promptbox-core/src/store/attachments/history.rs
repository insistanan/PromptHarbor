use super::{
    extraction::{expected_image_count, has_missing_images},
    PromptAttachment, PromptAttachmentDataUrl,
};
use super::super::types::PromptHistoryItem;
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use rusqlite::{params, Connection, OptionalExtension};
use std::{fs, path::PathBuf};

pub(in crate::store) fn append_prompt_history_attachments(
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
        item.has_missing_images =
            has_missing_images(item.expected_image_count, item.captured_image_count);
        item.attachments = attachments;
    }

    Ok(())
}

pub(in crate::store) fn read_prompt_attachment_data_url(
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

pub(in crate::store) fn session_attachment_files(
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
