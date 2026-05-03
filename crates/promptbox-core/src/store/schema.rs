use rusqlite::Connection;

use super::types::StoreSummary;

pub(super) fn migrate(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            r#"
            pragma foreign_keys = on;

            create table if not exists sessions (
              id integer primary key autoincrement,
              provider text not null,
              session_id text not null,
              status text not null default 'active',
              cwd text,
              transcript_path text,
              model text,
              first_prompt text,
              title text,
              title_source text not null default 'session_id',
              last_hook_at text,
              maybe_closed_at text,
              archived_at text,
              created_at text not null,
              updated_at text not null,
              unique(provider, session_id)
            );

            create table if not exists prompt_events (
              id integer primary key autoincrement,
              session_db_id integer not null references sessions(id),
              provider text not null,
              session_id text not null,
              turn_id text,
              prompt_md text not null,
              prompt_hash text not null,
              is_low_info integer not null default 0,
              matched_draft_id integer,
              source text not null default 'hook',
              sent_at text not null,
              created_at text not null
            );

            create table if not exists drafts (
              id integer primary key autoincrement,
              session_db_id integer not null references sessions(id),
              content_md text not null,
              content_hash text not null,
              copy_state text not null default 'idle',
              copied_at text,
              last_copied_hash text,
              updated_at text not null,
              unique(session_db_id)
            );

            create table if not exists draft_items (
              id integer primary key autoincrement,
              session_db_id integer not null references sessions(id),
              content_md text not null,
              content_hash text not null,
              status text not null default 'editing',
              copy_state text not null default 'idle',
              copied_at text,
              last_copied_hash text,
              sent_at text,
              matched_prompt_event_id integer,
              created_at text not null,
              updated_at text not null
            );

            create table if not exists raw_hook_events (
              id integer primary key autoincrement,
              provider text not null,
              session_id text,
              event_name text not null,
              raw_json text not null,
              received_at text not null,
              expires_at text not null
            );

            create table if not exists prompt_event_attachments (
              id integer primary key autoincrement,
              prompt_event_id integer not null references prompt_events(id) on delete cascade,
              provider text not null,
              session_id text not null,
              kind text not null,
              mime_type text not null,
              file_path text not null,
              file_name text not null,
              file_size integer not null,
              placeholder text,
              source text not null,
              position integer not null,
              created_at text not null,
              unique(prompt_event_id, position)
            );

            create index if not exists idx_sessions_status_updated_at
              on sessions(status, updated_at);
            create index if not exists idx_sessions_cwd
              on sessions(cwd);
            create index if not exists idx_prompt_events_session_sent_at
              on prompt_events(session_db_id, sent_at);
            create unique index if not exists idx_prompt_events_turn_id
              on prompt_events(provider, session_id, turn_id)
              where turn_id is not null;
            create index if not exists idx_prompt_event_attachments_event
              on prompt_event_attachments(prompt_event_id, position);
            create index if not exists idx_draft_items_session_updated_at
              on draft_items(session_db_id, status, updated_at);

            insert or ignore into draft_items (
              id, session_db_id, content_md, content_hash, status, copy_state,
              copied_at, last_copied_hash, created_at, updated_at
            )
            select
              id,
              session_db_id,
              content_md,
              content_hash,
              'editing',
              copy_state,
              copied_at,
              last_copied_hash,
              updated_at,
              updated_at
            from drafts;
            "#,
        )
        .map_err(|error| format!("初始化 PromptBox 数据库失败：{error}"))
}

pub(super) fn store_summary(connection: &Connection) -> Result<StoreSummary, String> {
    let session_count = count_table(connection, "sessions")?;
    let prompt_event_count = count_table(connection, "prompt_events")?;

    Ok(StoreSummary {
        session_count,
        prompt_event_count,
    })
}

fn count_table(connection: &Connection, table: &str) -> Result<usize, String> {
    let sql = format!("select count(*) from {table}");
    connection
        .query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count as usize)
        .map_err(|error| format!("读取数据表计数失败：{table}：{error}"))
}
