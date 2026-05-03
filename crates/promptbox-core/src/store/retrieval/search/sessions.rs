use rusqlite::{params, Connection};

use super::result::search_result;
use super::super::super::{
    text::{contains_query, is_low_info_prompt, short_session_title},
    types::PromptSearchResultItem,
};

pub(super) fn search_sessions(
    connection: &Connection,
    query: &str,
    pattern: &str,
    include_low_info: bool,
) -> Result<Vec<PromptSearchResultItem>, String> {
    let mut statement = connection
        .prepare(
            r#"
            select provider, session_id, cwd, title, first_prompt, updated_at
            from sessions
            where title like ?1
               or first_prompt like ?1
            order by updated_at desc
            limit 40
            "#,
        )
        .map_err(|error| format!("准备搜索 Agent 会话失败：{error}"))?;
    let rows = statement
        .query_map(params![pattern], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(|error| format!("搜索 Agent 会话失败：{error}"))?;

    let mut items = Vec::new();
    for row in rows {
        let (provider, session_id, cwd, title, first_prompt, updated_at) =
            row.map_err(|error| format!("解析 Agent 会话搜索结果失败：{error}"))?;
        let title = title.unwrap_or_else(|| short_session_title(&session_id));
        let title_matches = contains_query(&title, query);
        let first_prompt_matches = first_prompt
            .as_deref()
            .is_some_and(|prompt| contains_query(prompt, query));

        if title_matches {
            items.push(search_result(
                &provider,
                &session_id,
                cwd.as_deref(),
                &title,
                "session_title",
                "会话标题",
                &title,
                false,
                None,
                &updated_at,
            ));
        }

        if let Some(first_prompt) = first_prompt
            .filter(|prompt| include_low_info || !is_low_info_prompt(prompt))
            .filter(|_| first_prompt_matches)
        {
            items.push(search_result(
                &provider,
                &session_id,
                cwd.as_deref(),
                &title,
                "first_prompt",
                "首条 prompt",
                &first_prompt,
                is_low_info_prompt(&first_prompt),
                None,
                &updated_at,
            ));
        }
    }

    Ok(items)
}
