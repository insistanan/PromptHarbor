use crate::{current_captured_at, PromptEvent};
use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

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

    pub fn list_sessions(&self, maybe_closed_after_hours: u64) -> Result<SessionList, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        update_maybe_closed_sessions(&connection, maybe_closed_after_hours)?;
        list_sessions(&connection)
    }

    pub fn archive_session(
        &self,
        provider: &str,
        session_id: &str,
        force: bool,
    ) -> Result<ArchiveSessionOutcome, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        archive_session(&connection, provider, session_id, force)
    }

    pub fn get_draft(&self, provider: &str, session_id: &str) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        get_draft(&connection, provider, session_id)
    }

    pub fn save_draft(
        &self,
        provider: &str,
        session_id: &str,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        save_draft(&connection, provider, session_id, content_md)
    }

    pub fn mark_draft_copied(
        &self,
        provider: &str,
        session_id: &str,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        mark_draft_copied(&connection, provider, session_id, content_md)
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
pub struct SessionList {
    pub active: Vec<SessionListItem>,
    pub maybe_closed: Vec<SessionListItem>,
    pub archived: Vec<SessionListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItem {
    pub provider: String,
    pub provider_label: String,
    pub session_id: String,
    pub short_session_id: String,
    pub status: String,
    pub cwd: Option<String>,
    pub project_name: String,
    pub title: String,
    pub last_hook_at: Option<String>,
    pub updated_at: String,
    pub prompt_count: usize,
    pub has_non_empty_draft: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSessionOutcome {
    pub archived: bool,
    pub requires_confirmation: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftState {
    pub provider: String,
    pub session_id: String,
    pub content_md: String,
    pub content_hash: String,
    pub copy_state: String,
    pub copied_at: Option<String>,
    pub last_copied_hash: Option<String>,
    pub updated_at: String,
    pub is_empty: bool,
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

    if inserted {
        let prompt_event_id = connection.last_insert_rowid();
        if let Some(matched_draft_id) =
            clear_matching_copied_draft(connection, session_db_id, &prompt_hash, &now)?
        {
            connection
                .execute(
                    "update prompt_events set matched_draft_id = ?1 where id = ?2",
                    params![matched_draft_id, prompt_event_id],
                )
                .map_err(|error| format!("标记已发送 prompt 匹配草稿失败：{error}"))?;
        }
    }

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

fn update_maybe_closed_sessions(
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

fn list_sessions(connection: &Connection) -> Result<SessionList, String> {
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
              count(prompt_events.id) as prompt_count,
              coalesce(max(case
                when drafts.content_md is not null and trim(drafts.content_md) != ''
                then 1 else 0
              end), 0) as has_non_empty_draft
            from sessions
            left join prompt_events on prompt_events.session_db_id = sessions.id
            left join drafts on drafts.session_db_id = sessions.id
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

fn get_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    draft_state(connection, provider, session_id, session_db_id)
}

fn save_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let content_hash = prompt_hash(content_md);
    let existing_last_copied_hash = connection
        .query_row(
            "select last_copied_hash from drafts where session_db_id = ?1",
            params![session_db_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| format!("读取当前草稿复制状态失败：{error}"))?
        .flatten();
    let copy_state = draft_copy_state(
        content_md,
        &content_hash,
        existing_last_copied_hash.as_deref(),
    );
    let now = current_captured_at();

    connection
        .execute(
            r#"
            insert into drafts (session_db_id, content_md, content_hash, copy_state, updated_at)
            values (?1, ?2, ?3, ?4, ?5)
            on conflict(session_db_id) do update set
              content_md = excluded.content_md,
              content_hash = excluded.content_hash,
              copy_state = excluded.copy_state,
              updated_at = excluded.updated_at
            "#,
            params![session_db_id, content_md, content_hash, copy_state, now],
        )
        .map_err(|error| format!("保存当前草稿失败：{error}"))?;

    draft_state(connection, provider, session_id, session_db_id)
}

fn mark_draft_copied(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let content_hash = prompt_hash(content_md);
    let now = current_captured_at();

    connection
        .execute(
            r#"
            insert into drafts (
              session_db_id, content_md, content_hash, copy_state,
              copied_at, last_copied_hash, updated_at
            )
            values (?1, ?2, ?3, 'copied', ?4, ?3, ?4)
            on conflict(session_db_id) do update set
              content_md = excluded.content_md,
              content_hash = excluded.content_hash,
              copy_state = excluded.copy_state,
              copied_at = excluded.copied_at,
              last_copied_hash = excluded.last_copied_hash,
              updated_at = excluded.updated_at
            "#,
            params![session_db_id, content_md, content_hash, now],
        )
        .map_err(|error| format!("记录当前草稿复制状态失败：{error}"))?;

    draft_state(connection, provider, session_id, session_db_id)
}

fn draft_state(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    session_db_id: i64,
) -> Result<DraftState, String> {
    let draft = connection
        .query_row(
            r#"
            select content_md, content_hash, copy_state, copied_at, last_copied_hash, updated_at
            from drafts
            where session_db_id = ?1
            "#,
            params![session_db_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("读取当前草稿失败：{error}"))?;

    let (content_md, content_hash, copy_state, copied_at, last_copied_hash, updated_at) = draft
        .unwrap_or_else(|| {
            (
                String::new(),
                prompt_hash(""),
                "idle".to_string(),
                None,
                None,
                current_captured_at(),
            )
        });
    let is_empty = content_md.trim().is_empty();

    Ok(DraftState {
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        content_md,
        content_hash,
        copy_state,
        copied_at,
        last_copied_hash,
        updated_at,
        is_empty,
    })
}

fn clear_matching_copied_draft(
    connection: &Connection,
    session_db_id: i64,
    sent_prompt_hash: &str,
    now: &str,
) -> Result<Option<i64>, String> {
    let matched_draft_id = connection
        .query_row(
            r#"
            select id
            from drafts
            where session_db_id = ?1
              and content_hash = ?2
              and last_copied_hash = ?2
              and trim(content_md) != ''
            "#,
            params![session_db_id, sent_prompt_hash],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|error| format!("匹配已复制草稿失败：{error}"))?;

    if let Some(draft_id) = matched_draft_id {
        connection
            .execute(
                r#"
                update drafts
                set content_md = '',
                    content_hash = ?1,
                    copy_state = 'cleared_after_send',
                    updated_at = ?2
                where id = ?3
                "#,
                params![prompt_hash(""), now, draft_id],
            )
            .map_err(|error| format!("清空已发送草稿失败：{error}"))?;
    }

    Ok(matched_draft_id)
}

fn draft_copy_state(
    content_md: &str,
    content_hash: &str,
    last_copied_hash: Option<&str>,
) -> String {
    if content_md.trim().is_empty() {
        "idle".to_string()
    } else if last_copied_hash == Some(content_hash) {
        "copied".to_string()
    } else {
        "dirty".to_string()
    }
}

fn archive_session(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    force: bool,
) -> Result<ArchiveSessionOutcome, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let status = connection
        .query_row(
            "select status from sessions where id = ?1",
            params![session_db_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(|error| format!("读取 Agent 会话状态失败：{error}"))?;

    if status == "archived" {
        return Ok(ArchiveSessionOutcome {
            archived: true,
            requires_confirmation: false,
            message: "Agent 会话已经是历史状态".to_string(),
        });
    }

    if status != "active" && status != "maybe_closed" {
        return Err(format!("不支持归档当前状态的 Agent 会话：{status}"));
    }

    let has_non_empty_draft = session_has_non_empty_draft(connection, session_db_id)?;
    if has_non_empty_draft && !force {
        return Ok(ArchiveSessionOutcome {
            archived: false,
            requires_confirmation: true,
            message: "该 Agent 会话有非空当前草稿，归档前需要确认".to_string(),
        });
    }

    let now = current_captured_at();
    connection
        .execute(
            r#"
            update sessions
            set status = 'archived',
                archived_at = ?1,
                updated_at = ?1
            where id = ?2
            "#,
            params![now, session_db_id],
        )
        .map_err(|error| format!("归档 Agent 会话失败：{error}"))?;

    Ok(ArchiveSessionOutcome {
        archived: true,
        requires_confirmation: false,
        message: "Agent 会话已归档为历史".to_string(),
    })
}

fn session_has_non_empty_draft(
    connection: &Connection,
    session_db_id: i64,
) -> Result<bool, String> {
    connection
        .query_row(
            "select exists(select 1 from drafts where session_db_id = ?1 and trim(content_md) != '')",
            params![session_db_id],
            |row| row.get::<_, i64>(0),
        )
        .map(|exists| exists != 0)
        .map_err(|error| format!("检查当前草稿失败：{error}"))
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

fn short_session_id(session_id: &str) -> String {
    session_id.chars().take(8).collect::<String>()
}

fn provider_label(provider: &str) -> &'static str {
    match provider {
        "claude" => "Claude Code",
        "codex" => "Codex CLI",
        _ => "未知 Agent",
    }
}

fn project_name_from_cwd(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(cwd)
        .to_string()
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

    #[test]
    fn records_codex_turn_id_and_deduplicates_repeated_turn() {
        let home = isolated_home("codex-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();

        let event = PromptEvent {
            provider: Provider::Codex,
            event_name: "UserPromptSubmit".to_string(),
            session_id: "codex-session-1".to_string(),
            turn_id: Some("turn-1".to_string()),
            cwd: Some("D:\\code\\some\\prompt".to_string()),
            transcript_path: Some("D:\\codex\\rollout.jsonl".to_string()),
            model: Some("gpt-test".to_string()),
            prompt: Some("implement codex hook".to_string()),
            captured_at: "2026-05-03T12:00:00.000Z".to_string(),
            raw_json: json!({
                "hook_event_name": "UserPromptSubmit",
                "session_id": "codex-session-1",
                "turn_id": "turn-1",
                "prompt": "implement codex hook"
            }),
        };

        let first = store.record_prompt_event(&event).unwrap();
        let second = store.record_prompt_event(&event).unwrap();

        assert!(first.inserted);
        assert!(!second.inserted);
        assert_eq!(second.session_count, 1);
        assert_eq!(second.prompt_event_count, 1);
        assert_eq!(
            second.ignored_reason.as_deref(),
            Some("重复 turn_id，已忽略")
        );
    }

    #[test]
    fn moves_inactive_sessions_to_maybe_closed_without_archiving() {
        let home = isolated_home("maybe-closed-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        let event = test_event("claude", "old-session", "old prompt");
        store.record_prompt_event(&event).unwrap();

        let connection = store.open_connection().unwrap();
        connection
            .execute(
                "update sessions set last_hook_at = '2026-05-01T00:00:00.000Z', updated_at = '2026-05-01T00:00:00.000Z'",
                [],
            )
            .unwrap();

        let sessions = store.list_sessions(12).unwrap();
        assert_eq!(sessions.active.len(), 0);
        assert_eq!(sessions.maybe_closed.len(), 1);
        assert_eq!(sessions.archived.len(), 0);
    }

    #[test]
    fn archive_requires_confirmation_for_non_empty_draft_then_succeeds() {
        let home = isolated_home("archive-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        let event = test_event("claude", "draft-session", "draft prompt");
        store.record_prompt_event(&event).unwrap();

        let connection = store.open_connection().unwrap();
        let session_db_id = session_db_id(&connection, "claude", "draft-session").unwrap();
        connection
            .execute(
                "insert into drafts (session_db_id, content_md, content_hash, updated_at) values (?1, 'draft text', 'hash', ?2)",
                params![session_db_id, current_captured_at()],
            )
            .unwrap();

        let blocked = store
            .archive_session("claude", "draft-session", false)
            .unwrap();
        let archived = store
            .archive_session("claude", "draft-session", true)
            .unwrap();
        let sessions = store.list_sessions(12).unwrap();

        assert!(!blocked.archived);
        assert!(blocked.requires_confirmation);
        assert!(archived.archived);
        assert_eq!(sessions.archived.len(), 1);
    }

    #[test]
    fn saves_and_marks_current_draft_as_copied() {
        let home = isolated_home("draft-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        let event = test_event("claude", "draft-session", "first prompt");
        store.record_prompt_event(&event).unwrap();

        let saved = store
            .save_draft("claude", "draft-session", "  next prompt\n")
            .unwrap();
        let copied = store
            .mark_draft_copied("claude", "draft-session", "  next prompt\n")
            .unwrap();

        assert_eq!(saved.copy_state, "dirty");
        assert_eq!(copied.copy_state, "copied");
        assert_eq!(copied.content_hash, prompt_hash("next prompt"));
        assert_eq!(
            copied.last_copied_hash.as_deref(),
            Some(copied.content_hash.as_str())
        );
        assert!(!copied.is_empty);
    }

    #[test]
    fn matching_copied_prompt_clears_current_draft() {
        let home = isolated_home("draft-clear-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        store
            .record_prompt_event(&test_event("claude", "draft-session", "first prompt"))
            .unwrap();
        store
            .mark_draft_copied("claude", "draft-session", "send this prompt")
            .unwrap();

        store
            .record_prompt_event(&test_event("claude", "draft-session", " send this prompt "))
            .unwrap();
        let draft = store.get_draft("claude", "draft-session").unwrap();
        let connection = store.open_connection().unwrap();
        let matched_count: i64 = connection
            .query_row(
                "select count(*) from prompt_events where matched_draft_id is not null",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(draft.is_empty);
        assert_eq!(draft.copy_state, "cleared_after_send");
        assert_eq!(matched_count, 1);
    }

    #[test]
    fn mismatched_prompt_keeps_current_draft() {
        let home = isolated_home("draft-mismatch-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        store
            .record_prompt_event(&test_event("claude", "draft-session", "first prompt"))
            .unwrap();
        store
            .mark_draft_copied("claude", "draft-session", "send this prompt")
            .unwrap();

        store
            .record_prompt_event(&test_event("claude", "draft-session", "changed prompt"))
            .unwrap();
        let draft = store.get_draft("claude", "draft-session").unwrap();

        assert_eq!(draft.content_md, "send this prompt");
        assert_eq!(draft.copy_state, "copied");
    }

    #[test]
    fn archived_session_returns_to_active_after_new_prompt() {
        let home = isolated_home("reactivate-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        let first = test_event("claude", "archive-session", "first prompt");
        store.record_prompt_event(&first).unwrap();
        store
            .archive_session("claude", "archive-session", true)
            .unwrap();

        let second = PromptEvent {
            prompt: Some("second prompt".to_string()),
            captured_at: "2026-05-03T13:00:00.000Z".to_string(),
            ..test_event("claude", "archive-session", "second prompt")
        };
        store.record_prompt_event(&second).unwrap();
        let sessions = store.list_sessions(12).unwrap();

        assert_eq!(sessions.active.len(), 1);
        assert_eq!(sessions.archived.len(), 0);
    }

    fn test_event(provider: &str, session_id: &str, prompt: &str) -> PromptEvent {
        PromptEvent {
            provider: Provider::parse(provider).unwrap(),
            event_name: "UserPromptSubmit".to_string(),
            session_id: session_id.to_string(),
            turn_id: None,
            cwd: Some("D:\\code\\some\\prompt".to_string()),
            transcript_path: None,
            model: None,
            prompt: Some(prompt.to_string()),
            captured_at: "2026-05-03T12:00:00.000Z".to_string(),
            raw_json: json!({
                "hook_event_name": "UserPromptSubmit",
                "session_id": session_id,
                "prompt": prompt
            }),
        }
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
