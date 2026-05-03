use super::{
    listing::list_drafts,
    shared::{
        draft_state_by_id, ensure_session_has_editing_draft, insert_empty_draft,
        session_db_id_for_draft,
    },
};
use super::super::types::{DraftList, DraftState};
use rusqlite::{params, Connection};

pub(in crate::store) fn create_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let draft_id = insert_empty_draft(connection, session_db_id)?;
    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

pub(in crate::store) fn delete_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
) -> Result<DraftList, String> {
    let session_db_id = session_db_id_for_draft(connection, provider, session_id)?;
    let deleted = connection
        .execute(
            "delete from draft_items where id = ?1 and session_db_id = ?2",
            params![draft_id, session_db_id],
        )
        .map_err(|error| format!("删除草稿失败：{error}"))?;

    if deleted == 0 {
        return Err("草稿不存在或不属于当前会话".to_string());
    }

    ensure_session_has_editing_draft(connection, session_db_id)?;
    list_drafts(connection, provider, session_id)
}

pub(in crate::store) fn session_has_non_empty_draft(
    connection: &Connection,
    session_db_id: i64,
) -> Result<bool, String> {
    connection
        .query_row(
            r#"
            select exists(
              select 1
              from draft_items
              where session_db_id = ?1
                and trim(content_md) != ''
                and status != 'sent'
            )
            "#,
            params![session_db_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists != 0)
        .map_err(|error| format!("检查当前草稿失败：{error}"))
}
