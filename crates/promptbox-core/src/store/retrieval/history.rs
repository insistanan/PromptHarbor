use super::super::{
    attachments,
    text::{bool_to_i64, session_db_id},
    types::{PromptHistory, PromptHistoryItem},
};
use rusqlite::{params, Connection};

pub(in crate::store) fn list_prompt_history(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    include_low_info: bool,
) -> Result<PromptHistory, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let mut statement = connection
        .prepare(
            r#"
            select id, prompt_md, prompt_hash, is_low_info, matched_draft_id, sent_at, created_at
            from prompt_events
            where session_db_id = ?1
              and (?2 = 1 or is_low_info = 0)
            order by sent_at desc, id desc
            "#,
        )
        .map_err(|error| format!("准备读取 prompt 历史失败：{error}"))?;
    let rows = statement
        .query_map(
            params![session_db_id, bool_to_i64(include_low_info)],
            |row| {
                let is_low_info: i64 = row.get(3)?;

                Ok(PromptHistoryItem {
                    id: row.get(0)?,
                    prompt_md: row.get(1)?,
                    prompt_hash: row.get(2)?,
                    is_low_info: is_low_info != 0,
                    matched_draft_id: row.get(4)?,
                    sent_at: row.get(5)?,
                    created_at: row.get(6)?,
                    expected_image_count: 0,
                    captured_image_count: 0,
                    has_missing_images: false,
                    attachments: Vec::new(),
                })
            },
        )
        .map_err(|error| format!("读取 prompt 历史失败：{error}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| format!("解析 prompt 历史失败：{error}"))?);
    }
    attachments::append_prompt_history_attachments(connection, &mut items)?;

    Ok(PromptHistory {
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        items,
    })
}
