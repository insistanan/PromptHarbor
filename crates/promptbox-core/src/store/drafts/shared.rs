use super::super::{
    text::{prompt_hash, session_db_id},
    types::DraftState,
};
use crate::current_captured_at;
use rusqlite::{params, Connection, OptionalExtension};

pub(in crate::store::drafts) fn draft_state_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    session_db_id: i64,
    draft_id: i64,
) -> Result<DraftState, String> {
    connection
        .query_row(
            r#"
            select
              id,
              content_md,
              content_hash,
              status,
              copy_state,
              copied_at,
              last_copied_hash,
              sent_at,
              matched_prompt_event_id,
              updated_at
            from draft_items
            where session_db_id = ?1
              and id = ?2
            "#,
            params![session_db_id, draft_id],
            |row| {
                let content_md: String = row.get(1)?;
                Ok(DraftState {
                    id: row.get(0)?,
                    provider: provider.to_string(),
                    session_id: session_id.to_string(),
                    content_hash: row.get(2)?,
                    status: row.get(3)?,
                    copy_state: row.get(4)?,
                    copied_at: row.get(5)?,
                    last_copied_hash: row.get(6)?,
                    sent_at: row.get(7)?,
                    matched_prompt_event_id: row.get(8)?,
                    updated_at: row.get(9)?,
                    is_empty: content_md.trim().is_empty(),
                    content_md,
                })
            },
        )
        .map_err(|error| format!("读取当前草稿失败：{error}"))
}

pub(in crate::store::drafts) fn preferred_editing_draft_id(
    connection: &Connection,
    session_db_id: i64,
) -> Result<i64, String> {
    ensure_session_has_editing_draft(connection, session_db_id)?;
    connection
        .query_row(
            r#"
            select id
            from draft_items
            where session_db_id = ?1
              and status != 'sent'
            order by updated_at desc, id desc
            limit 1
            "#,
            params![session_db_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("读取当前草稿 ID 失败：{error}"))
}

pub(in crate::store::drafts) fn ensure_session_has_editing_draft(
    connection: &Connection,
    session_db_id: i64,
) -> Result<(), String> {
    let has_editing = connection
        .query_row(
            "select exists(select 1 from draft_items where session_db_id = ?1 and status != 'sent')",
            params![session_db_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("检查当前会话草稿失败：{error}"))?
        != 0;

    if !has_editing {
        insert_empty_draft(connection, session_db_id)?;
    }

    Ok(())
}

pub(in crate::store::drafts) fn insert_empty_draft(
    connection: &Connection,
    session_db_id: i64,
) -> Result<i64, String> {
    let now = current_captured_at();
    connection
        .execute(
            r#"
            insert into draft_items (
              session_db_id, content_md, content_hash, status, copy_state,
              created_at, updated_at
            )
            values (?1, '', ?2, 'editing', 'idle', ?3, ?3)
            "#,
            params![session_db_id, prompt_hash(""), now],
        )
        .map_err(|error| format!("创建空草稿失败：{error}"))?;

    Ok(connection.last_insert_rowid())
}

pub(in crate::store::drafts) fn draft_status(
    connection: &Connection,
    session_db_id: i64,
    draft_id: i64,
) -> Result<String, String> {
    connection
        .query_row(
            "select status from draft_items where id = ?1 and session_db_id = ?2",
            params![draft_id, session_db_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("读取草稿状态失败：{error}"))?
        .ok_or_else(|| "草稿不存在或不属于当前会话".to_string())
}

pub(in crate::store::drafts) fn session_db_id_for_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<i64, String> {
    session_db_id(connection, provider, session_id)
}

pub(in crate::store::drafts) fn draft_copy_state(
    content_md: &str,
    content_hash: &str,
    last_copied_hash: Option<&str>,
) -> String {
    if content_md.trim().is_empty() {
        "idle".to_string()
    } else if last_copied_hash == Some(content_hash) {
        "copied".to_string()
    } else {
        "dirty".to_string()
    }
}
