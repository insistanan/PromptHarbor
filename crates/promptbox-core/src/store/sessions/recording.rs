use super::super::{
    drafts,
    schema::store_summary,
    text::{is_low_info_prompt, prompt_hash, session_db_id, short_session_title, title_from_prompt},
    types::RecordOutcome,
};
use crate::{current_captured_at, PromptEvent};
use rusqlite::{params, Connection};

pub(in crate::store) fn record_prompt_event(
    connection: &Connection,
    event: &PromptEvent,
) -> Result<RecordOutcome, String> {
    if event.event_name != "UserPromptSubmit" {
        return ignored(
            connection,
            format!("忽略非 UserPromptSubmit 事件：{}", event.event_name),
        );
    }

    let Some(prompt) = event
        .prompt
        .as_ref()
        .map(|prompt| prompt.trim())
        .filter(|prompt| !prompt.is_empty())
    else {
        return ignored(connection, "忽略没有用户 prompt 内容的事件".to_string());
    };

    let prompt = prompt.to_string();
    let now = current_captured_at();
    let provider = event.provider.as_str();
    let title = title_from_prompt(&prompt, &event.session_id);
    let title_source = if title == short_session_title(&event.session_id) {
        "session_id"
    } else {
        "first_non_low_info_prompt"
    };

    connection
        .execute_batch("begin immediate")
        .map_err(|error| format!("开启已发送 prompt 入库事务失败：{error}"))?;

    let transaction_result = (|| {
        connection
            .execute(
                r#"
                insert into sessions (
                  provider, session_id, status, cwd, transcript_path, model,
                  first_prompt, title, title_source, last_hook_at, created_at, updated_at
                )
                values (?1, ?2, 'active', ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
                on conflict(provider, session_id) do update set
                  status = 'active',
                  cwd = coalesce(excluded.cwd, sessions.cwd),
                  transcript_path = coalesce(excluded.transcript_path, sessions.transcript_path),
                  model = coalesce(excluded.model, sessions.model),
                  first_prompt = coalesce(sessions.first_prompt, excluded.first_prompt),
                  title = case
                    when sessions.title_source in ('session_id', 'first_non_low_info_prompt')
                      and sessions.first_prompt is null
                    then excluded.title
                    else sessions.title
                  end,
                  title_source = case
                    when sessions.title_source in ('session_id', 'first_non_low_info_prompt')
                      and sessions.first_prompt is null
                    then excluded.title_source
                    else sessions.title_source
                  end,
                  last_hook_at = excluded.last_hook_at,
                  maybe_closed_at = null,
                  archived_at = null,
                  updated_at = excluded.updated_at
                "#,
                params![
                    provider,
                    &event.session_id,
                    event.cwd.as_deref(),
                    event.transcript_path.as_deref(),
                    event.model.as_deref(),
                    prompt.as_str(),
                    title.as_str(),
                    title_source,
                    event.captured_at.as_str(),
                    now.as_str(),
                ],
            )
            .map_err(|error| format!("写入 Agent 会话失败：{error}"))?;

        let session_db_id = session_db_id(connection, provider, &event.session_id)?;
        let prompt_hash = prompt_hash(&prompt);
        let inserted = connection
            .execute(
                r#"
                insert or ignore into prompt_events (
                  session_db_id, provider, session_id, turn_id, prompt_md, prompt_hash,
                  is_low_info, source, sent_at, created_at
                )
                values (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'hook', ?8, ?9)
                "#,
                params![
                    session_db_id,
                    provider,
                    &event.session_id,
                    event.turn_id.as_deref(),
                    prompt.as_str(),
                    prompt_hash.as_str(),
                    if is_low_info_prompt(&prompt) {
                        1_i64
                    } else {
                        0_i64
                    },
                    event.captured_at.as_str(),
                    now.as_str(),
                ],
            )
            .map_err(|error| format!("写入已发送 prompt 失败：{error}"))?
            > 0;

        let prompt_event_id = if inserted {
            let prompt_event_id = connection.last_insert_rowid();
            if let Some(matched_draft_id) = drafts::clear_matching_copied_draft(
                connection,
                session_db_id,
                &prompt_hash,
                &now,
                prompt_event_id,
            )? {
                connection
                    .execute(
                        "update prompt_events set matched_draft_id = ?1 where id = ?2",
                        params![matched_draft_id, prompt_event_id],
                    )
                    .map_err(|error| format!("标记已发送 prompt 匹配草稿失败：{error}"))?;
            }
            Some(prompt_event_id)
        } else {
            None
        };

        let summary = store_summary(connection)?;
        Ok((
            RecordOutcome {
                inserted,
                ignored_reason: (!inserted).then_some("重复 turn_id，已忽略".to_string()),
                session_count: summary.session_count,
                prompt_event_count: summary.prompt_event_count,
                prompt_event_id,
            },
            prompt_event_id,
        ))
    })();

    match transaction_result {
        Ok((outcome, _prompt_event_id)) => {
            if let Err(error) = connection.execute_batch("commit") {
                let _ = connection.execute_batch("rollback");
                return Err(format!("提交已发送 prompt 入库事务失败：{error}"));
            }

            Ok(outcome)
        }
        Err(error) => {
            let _ = connection.execute_batch("rollback");
            Err(error)
        }
    }
}

fn ignored(connection: &Connection, reason: String) -> Result<RecordOutcome, String> {
    let summary = store_summary(connection)?;
    Ok(RecordOutcome {
        inserted: false,
        ignored_reason: Some(reason),
        session_count: summary.session_count,
        prompt_event_count: summary.prompt_event_count,
        prompt_event_id: None,
    })
}
