use super::shared::{
    draft_state_by_id, ensure_session_has_editing_draft, preferred_editing_draft_id,
    session_db_id_for_draft,
};
use super::super::types::{DraftList, DraftListItem, DraftState};
use rusqlite::{params, Connection};

pub(in crate::store) fn get_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let draft_id = preferred_editing_draft_id(connection, session_db_id)?;
    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

pub(in crate::store) fn list_drafts(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftList, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    ensure_session_has_editing_draft(connection, session_db_id)?;

    let mut statement = connection
        .prepare(
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
            order by
              case when status = 'sent' then 1 else 0 end,
              updated_at desc,
              id desc
            "#,
        )
        .map_err(|error| format!("准备读取草稿列表失败：{error}"))?;
    let rows = statement
        .query_map(params![session_db_id], |row| {
            let content_md: String = row.get(1)?;
            Ok(DraftListItem {
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
                preview: draft_preview(&content_md),
                content_md,
            })
        })
        .map_err(|error| format!("读取草稿列表失败：{error}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| format!("解析草稿列表失败：{error}"))?);
    }

    Ok(DraftList {
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        items,
    })
}

pub(in crate::store) fn get_draft_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

fn draft_preview(content_md: &str) -> String {
    let preview = content_md.split_whitespace().collect::<Vec<_>>().join(" ");
    if preview.is_empty() {
        "空草稿".to_string()
    } else {
        preview.chars().take(80).collect()
    }
}
