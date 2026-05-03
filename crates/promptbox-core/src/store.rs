use crate::{current_captured_at, PromptEvent};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct PromptStore {
    database_path: PathBuf,
}

impl PromptStore {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub fn initialize(&self) -> Result<StoreSummary, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        store_summary(&connection)
    }

    pub fn record_prompt_event(&self, event: &PromptEvent) -> Result<RecordOutcome, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        record_prompt_event(&connection, event)
    }

    pub fn summary(&self) -> Result<StoreSummary, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        store_summary(&connection)
    }

    fn open_connection(&self) -> Result<Connection, String> {
        Connection::open(&self.database_path).map_err(|error| {
            format!(
                "打开 PromptBox 数据库失败：{}：{error}",
                self.database_path.display()
            )
        })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreSummary {
    pub session_count: usize,
    pub prompt_event_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOutcome {
    pub inserted: bool,
    pub ignored_reason: Option<String>,
    pub session_count: usize,
    pub prompt_event_count: usize,
}

fn migrate(connection: &Connection) -> Result<(), String> {
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

            create table if not exists raw_hook_events (
              id integer primary key autoincrement,
              provider text not null,
              session_id text,
              event_name text not null,
              raw_json text not null,
              received_at text not null,
              expires_at text not null
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
            "#,
        )
        .map_err(|error| format!("初始化 PromptBox 数据库失败：{error}"))
}

fn record_prompt_event(
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

    let now = current_captured_at();
    let provider = event.provider.as_str();
    let title = title_from_prompt(prompt, &event.session_id);
    let title_source = if title == short_session_title(&event.session_id) {
        "session_id"
    } else {
        "first_non_low_info_prompt"
    };

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
                event.session_id,
                event.cwd,
                event.transcript_path,
                event.model,
                prompt,
                title,
                title_source,
                event.captured_at,
                now,
            ],
        )
        .map_err(|error| format!("写入 Agent 会话失败：{error}"))?;

    let session_db_id = session_db_id(connection, provider, &event.session_id)?;
    let prompt_hash = prompt_hash(prompt);
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
                event.session_id,
                event.turn_id,
                prompt,
                prompt_hash,
                if is_low_info_prompt(prompt) {
                    1_i64
                } else {
                    0_i64
                },
                event.captured_at,
                now,
            ],
        )
        .map_err(|error| format!("写入已发送 prompt 失败：{error}"))?
        > 0;

    let summary = store_summary(connection)?;
    Ok(RecordOutcome {
        inserted,
        ignored_reason: (!inserted).then_some("重复 turn_id，已忽略".to_string()),
        session_count: summary.session_count,
        prompt_event_count: summary.prompt_event_count,
    })
}

fn ignored(connection: &Connection, reason: String) -> Result<RecordOutcome, String> {
    let summary = store_summary(connection)?;
    Ok(RecordOutcome {
        inserted: false,
        ignored_reason: Some(reason),
        session_count: summary.session_count,
        prompt_event_count: summary.prompt_event_count,
    })
}

fn store_summary(connection: &Connection) -> Result<StoreSummary, String> {
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

fn session_db_id(connection: &Connection, provider: &str, session_id: &str) -> Result<i64, String> {
    connection
        .query_row(
            "select id from sessions where provider = ?1 and session_id = ?2",
            params![provider, session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("读取 Agent 会话 ID 失败：{error}"))?
        .ok_or_else(|| "写入后没有找到 Agent 会话".to_string())
}

fn prompt_hash(prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

fn is_low_info_prompt(prompt: &str) -> bool {
    let normalized = prompt.trim().to_ascii_lowercase();
    normalized.chars().count() <= 8
        || matches!(
            normalized.as_str(),
            "同意" | "继续" | "好的" | "收到" | "hi" | "hello" | "你好" | "可以" | "好"
        )
}

fn title_from_prompt(prompt: &str, session_id: &str) -> String {
    if is_low_info_prompt(prompt) {
        return short_session_title(session_id);
    }

    let collapsed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title = collapsed.chars().take(48).collect::<String>();
    if collapsed.chars().count() > 48 {
        title.push_str("...");
    }

    if title.trim().is_empty() {
        short_session_title(session_id)
    } else {
        title
    }
}

fn short_session_title(session_id: &str) -> String {
    let short = session_id.chars().take(8).collect::<String>();
    if short.is_empty() {
        "未命名会话".to_string()
    } else {
        format!("会话 {short}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Provider;
    use serde_json::json;
    use std::{
        env, fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn records_claude_user_prompt_into_session_and_prompt_history() {
        let home = isolated_home("store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        let initial = store.initialize().unwrap();
        assert_eq!(initial.session_count, 0);
        assert_eq!(initial.prompt_event_count, 0);

        let event = PromptEvent {
            provider: Provider::Claude,
            event_name: "UserPromptSubmit".to_string(),
            session_id: "claude-session-1".to_string(),
            turn_id: None,
            cwd: Some("D:\\code\\some\\prompt".to_string()),
            transcript_path: Some("D:\\claude\\transcript.jsonl".to_string()),
            model: None,
            prompt: Some("帮我实现 Claude Code hook".to_string()),
            captured_at: "2026-05-03T12:00:00.000Z".to_string(),
            raw_json: json!({
                "hook_event_name": "UserPromptSubmit",
                "session_id": "claude-session-1",
                "prompt": "帮我实现 Claude Code hook"
            }),
        };

        let outcome = store.record_prompt_event(&event).unwrap();
        assert!(outcome.inserted);
        assert_eq!(outcome.session_count, 1);
        assert_eq!(outcome.prompt_event_count, 1);
    }

    fn isolated_home(name: &str) -> PathBuf {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();
        let path = env::temp_dir().join(format!("promptbox-{name}-{millis}"));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }
}
