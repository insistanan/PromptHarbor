use super::shared::{
    draft_copy_state, draft_state_by_id, draft_status, insert_empty_draft,
    preferred_editing_draft_id, session_db_id_for_draft,
};
use super::super::{
    text::prompt_hash,
    types::DraftState,
};
use crate::current_captured_at;
use rusqlite::{params, Connection, OptionalExtension};

pub(in crate::store) fn save_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let draft_id = preferred_editing_draft_id(connection, session_db_id)?;
    save_draft_by_id(connection, provider, session_id, draft_id, content_md)
}

pub(in crate::store) fn save_draft_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let status = draft_status(connection, session_db_id, draft_id)?;
    if status == "sent" {
        return Err("已发送草稿不能继续编辑，请新建草稿".to_string());
    }

    let content_hash = prompt_hash(content_md);
    let existing_last_copied_hash = connection
        .query_row(
            "select last_copied_hash from draft_items where id = ?1 and session_db_id = ?2",
            params![draft_id, session_db_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| format!("读取当前草稿复制状态失败：{error}"))?
        .flatten();
    let copy_state = draft_copy_state(
        content_md,
        &content_hash,
        existing_last_copied_hash.as_deref(),
    );
    let now = current_captured_at();

    connection
        .execute(
            r#"
            update draft_items
            set content_md = ?1,
                content_hash = ?2,
                copy_state = ?3,
                updated_at = ?4
            where id = ?5
              and session_db_id = ?6
            "#,
            params![
                content_md,
                content_hash,
                copy_state,
                now,
                draft_id,
                session_db_id
            ],
        )
        .map_err(|error| format!("保存当前草稿失败：{error}"))?;

    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

pub(in crate::store) fn mark_draft_copied(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let draft_id = preferred_editing_draft_id(connection, session_db_id)?;
    mark_draft_copied_by_id(connection, provider, session_id, draft_id, content_md)
}

pub(in crate::store) fn mark_draft_copied_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let status = draft_status(connection, session_db_id, draft_id)?;
    if status == "sent" {
        return Err("已发送草稿不能再次标记为待发送".to_string());
    }

    let content_hash = prompt_hash(content_md);
    let now = current_captured_at();

    connection
        .execute(
            r#"
            update draft_items
            set content_md = ?1,
                content_hash = ?2,
                copy_state = 'copied',
                copied_at = ?3,
                last_copied_hash = ?2,
                updated_at = ?3
            where id = ?4
              and session_db_id = ?5
            "#,
            params![content_md, content_hash, now, draft_id, session_db_id],
        )
        .map_err(|error| format!("记录当前草稿复制状态失败：{error}"))?;

    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

pub(in crate::store) fn clear_matching_copied_draft(
    connection: &Connection,
    session_db_id: i64,
    sent_prompt_hash: &str,
    now: &str,
    prompt_event_id: i64,
) -> Result<Option<i64>, String> {
    let matched_draft_id = connection
        .query_row(
            r#"
            select id
            from draft_items
            where session_db_id = ?1
              and content_hash = ?2
              and last_copied_hash = ?2
              and trim(content_md) != ''
              and status != 'sent'
            order by copied_at desc, updated_at desc, id desc
            limit 1
            "#,
            params![session_db_id, sent_prompt_hash],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| format!("匹配已复制草稿失败：{error}"))?;

    if let Some(draft_id) = matched_draft_id {
        connection
            .execute(
                r#"
                update draft_items
                set status = 'sent',
                    copy_state = 'cleared_after_send',
                    sent_at = ?1,
                    matched_prompt_event_id = ?2,
                    updated_at = ?1
                where id = ?3
                "#,
                params![now, prompt_event_id, draft_id],
            )
            .map_err(|error| format!("标记已发送草稿失败：{error}"))?;
        insert_empty_draft(connection, session_db_id)?;
    }

    Ok(matched_draft_id)
}
