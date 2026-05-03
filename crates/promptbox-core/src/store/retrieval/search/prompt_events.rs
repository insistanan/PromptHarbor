use rusqlite::{params, Connection};

use super::result::search_result;
use super::super::super::{
    text::{bool_to_i64, short_session_title},
    types::PromptSearchResultItem,
};

pub(super) fn search_prompt_events(
    connection: &Connection,
    pattern: &str,
    include_low_info: bool,
) -> Result<Vec<PromptSearchResultItem>, String> {
    let mut statement = connection
        .prepare(
            r#"
            select
              prompt_events.provider,
              prompt_events.session_id,
              sessions.cwd,
              sessions.title,
              prompt_events.prompt_md,
              prompt_events.is_low_info,
              prompt_events.sent_at,
              sessions.updated_at
            from prompt_events
            join sessions on sessions.id = prompt_events.session_db_id
            where prompt_events.prompt_md like ?1
              and (?2 = 1 or prompt_events.is_low_info = 0)
            order by prompt_events.sent_at desc, prompt_events.id desc
            limit 80
            "#,
        )
        .map_err(|error| format!("准备搜索已发送 prompt 失败：{error}"))?;
    let rows = statement
        .query_map(params![pattern, bool_to_i64(include_low_info)], |row| {
            let is_low_info: i64 = row.get(5)?;

            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, String>(4)?,
                is_low_info != 0,
                row.get::<_, String>(6)?,
                row.get::<_, String>(7)?,
            ))
        })
        .map_err(|error| format!("搜索已发送 prompt 失败：{error}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (provider, session_id, cwd, title, prompt_md, is_low_info, sent_at, updated_at) =
            row.map_err(|error| format!("解析已发送 prompt 搜索结果失败：{error}"))?;
        let title = title.unwrap_or_else(|| short_session_title(&session_id));
        items.push(search_result(
            &provider,
            &session_id,
            cwd.as_deref(),
            &title,
            "sent_prompt",
            "已发送 prompt",
            &prompt_md,
            is_low_info,
            Some(sent_at),
            &updated_at,
        ));
    }

    Ok(items)
}
