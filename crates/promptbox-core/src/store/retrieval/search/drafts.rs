use rusqlite::{params, Connection};

use super::result::search_result;
use super::super::super::{text::short_session_title, types::PromptSearchResultItem};

pub(super) fn search_drafts(
    connection: &Connection,
    pattern: &str,
) -> Result<Vec<PromptSearchResultItem>, String> {
    let mut statement = connection
        .prepare(
            r#"
            select
              sessions.provider,
              sessions.session_id,
              sessions.cwd,
              sessions.title,
              draft_items.content_md,
              draft_items.updated_at
            from draft_items
            join sessions on sessions.id = draft_items.session_db_id
            where draft_items.content_md like ?1
              and trim(draft_items.content_md) != ''
              and draft_items.status != 'sent'
            order by draft_items.updated_at desc
            limit 40
            "#,
        )
        .map_err(|error| format!("准备搜索当前草稿失败：{error}"))?;
    let rows = statement
        .query_map(params![pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("搜索当前草稿失败：{error}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (provider, session_id, cwd, title, content_md, updated_at) =
            row.map_err(|error| format!("解析当前草稿搜索结果失败：{error}"))?;
        let title = title.unwrap_or_else(|| short_session_title(&session_id));
        items.push(search_result(
            &provider,
            &session_id,
            cwd.as_deref(),
            &title,
            "current_draft",
            "当前草稿",
            &content_md,
            false,
            None,
            &updated_at,
        ));
    }

    Ok(items)
}
