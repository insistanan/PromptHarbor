use super::super::{
    text::{
        project_name_from_cwd, provider_label, short_session_id, short_session_title,
    },
    types::{SessionList, SessionListItem},
};
use crate::current_captured_at;
use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection};

pub(in crate::store) fn update_maybe_closed_sessions(
    connection: &Connection,
    maybe_closed_after_hours: u64,
) -> Result<(), String> {
    let threshold = (Utc::now() - Duration::hours(maybe_closed_after_hours as i64))
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let now = current_captured_at();

    connection
        .execute(
            r#"
            update sessions
            set status = 'maybe_closed',
                maybe_closed_at = ?1,
                updated_at = ?1
            where status = 'active'
              and last_hook_at is not null
              and last_hook_at < ?2
            "#,
            params![now, threshold],
        )
        .map_err(|error| format!("更新可能已关闭 Agent 会话失败：{error}"))?;

    Ok(())
}

pub(in crate::store) fn list_sessions(connection: &Connection) -> Result<SessionList, String> {
    let mut statement = connection
        .prepare(
            r#"
            select
              sessions.provider,
              sessions.session_id,
              sessions.status,
              sessions.cwd,
              sessions.title,
              sessions.last_hook_at,
              sessions.updated_at,
              count(distinct prompt_events.id) as prompt_count,
              coalesce(max(case
                when draft_items.content_md is not null
                  and trim(draft_items.content_md) != ''
                  and draft_items.status != 'sent'
                then 1 else 0
              end), 0) as has_non_empty_draft
            from sessions
            left join prompt_events on prompt_events.session_db_id = sessions.id
            left join draft_items on draft_items.session_db_id = sessions.id
            group by sessions.id
            order by sessions.updated_at desc
            "#,
        )
        .map_err(|error| format!("准备读取 Agent 会话列表失败：{error}"))?;

    let rows = statement
        .query_map([], |row| {
            let provider: String = row.get(0)?;
            let session_id: String = row.get(1)?;
            let status: String = row.get(2)?;
            let cwd: Option<String> = row.get(3)?;
            let title: Option<String> = row.get(4)?;
            let last_hook_at: Option<String> = row.get(5)?;
            let updated_at: String = row.get(6)?;
            let prompt_count: i64 = row.get(7)?;
            let has_non_empty_draft: i64 = row.get(8)?;

            Ok(SessionListItem {
                provider_label: provider_label(&provider).to_string(),
                short_session_id: short_session_id(&session_id),
                project_name: cwd
                    .as_deref()
                    .map(project_name_from_cwd)
                    .unwrap_or_else(|| "未知项目".to_string()),
                title: title.unwrap_or_else(|| short_session_title(&session_id)),
                provider,
                session_id,
                status,
                cwd,
                last_hook_at,
                updated_at,
                prompt_count: prompt_count as usize,
                has_non_empty_draft: has_non_empty_draft > 0,
            })
        })
        .map_err(|error| format!("读取 Agent 会话列表失败：{error}"))?;

    let mut sessions = SessionList {
        active: Vec::new(),
        maybe_closed: Vec::new(),
        archived: Vec::new(),
    };

    for row in rows {
        let session = row.map_err(|error| format!("解析 Agent 会话列表失败：{error}"))?;
        match session.status.as_str() {
            "active" => sessions.active.push(session),
            "maybe_closed" => sessions.maybe_closed.push(session),
            "archived" => sessions.archived.push(session),
            _ => sessions.maybe_closed.push(session),
        }
    }

    Ok(sessions)
}
