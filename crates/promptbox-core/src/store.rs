use crate::{current_captured_at, PromptEvent, Provider};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine};
use chrono::{Duration, SecondsFormat, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

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
        record_prompt_event(&connection, event, &self.attachment_root())
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

    pub fn delete_session(
        &self,
        provider: &str,
        session_id: &str,
    ) -> Result<DeleteSessionOutcome, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        delete_session(&connection, provider, session_id, &self.attachment_root())
    }

    pub fn get_draft(&self, provider: &str, session_id: &str) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        get_draft(&connection, provider, session_id)
    }

    pub fn list_drafts(&self, provider: &str, session_id: &str) -> Result<DraftList, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        list_drafts(&connection, provider, session_id)
    }

    pub fn get_draft_by_id(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        get_draft_by_id(&connection, provider, session_id, draft_id)
    }

    pub fn create_draft(&self, provider: &str, session_id: &str) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        create_draft(&connection, provider, session_id)
    }

    pub fn delete_draft(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
    ) -> Result<DraftList, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        delete_draft(&connection, provider, session_id, draft_id)
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

    pub fn save_draft_by_id(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        save_draft_by_id(&connection, provider, session_id, draft_id, content_md)
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

    pub fn mark_draft_copied_by_id(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        mark_draft_copied_by_id(&connection, provider, session_id, draft_id, content_md)
    }

    pub fn list_prompt_history(
        &self,
        provider: &str,
        session_id: &str,
        include_low_info: bool,
    ) -> Result<PromptHistory, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        list_prompt_history(
            &connection,
            provider,
            session_id,
            include_low_info,
            &self.attachment_root(),
        )
    }

    pub fn read_prompt_attachment_data_url(
        &self,
        attachment_id: i64,
    ) -> Result<PromptAttachmentDataUrl, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        read_prompt_attachment_data_url(&connection, attachment_id)
    }

    pub fn search_prompts(
        &self,
        query: &str,
        include_low_info: bool,
    ) -> Result<PromptSearchResults, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        search_prompts(&connection, query, include_low_info)
    }

    fn open_connection(&self) -> Result<Connection, String> {
        Connection::open(&self.database_path).map_err(|error| {
            format!(
                "打开 PromptBox 数据库失败：{}：{error}",
                self.database_path.display()
            )
        })
    }

    fn attachment_root(&self) -> PathBuf {
        self.database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("attachments")
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
pub struct DeleteSessionOutcome {
    pub deleted: bool,
    pub provider: String,
    pub session_id: String,
    pub prompt_events_deleted: usize,
    pub drafts_deleted: usize,
    pub attachments_deleted: usize,
    pub files_deleted: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftState {
    pub id: i64,
    pub provider: String,
    pub session_id: String,
    pub content_md: String,
    pub content_hash: String,
    pub status: String,
    pub copy_state: String,
    pub copied_at: Option<String>,
    pub last_copied_hash: Option<String>,
    pub sent_at: Option<String>,
    pub matched_prompt_event_id: Option<i64>,
    pub updated_at: String,
    pub is_empty: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftList {
    pub provider: String,
    pub session_id: String,
    pub items: Vec<DraftListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftListItem {
    pub id: i64,
    pub provider: String,
    pub session_id: String,
    pub content_md: String,
    pub content_hash: String,
    pub status: String,
    pub copy_state: String,
    pub copied_at: Option<String>,
    pub last_copied_hash: Option<String>,
    pub sent_at: Option<String>,
    pub matched_prompt_event_id: Option<i64>,
    pub updated_at: String,
    pub is_empty: bool,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOutcome {
    pub inserted: bool,
    pub ignored_reason: Option<String>,
    pub session_count: usize,
    pub prompt_event_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptHistory {
    pub provider: String,
    pub session_id: String,
    pub items: Vec<PromptHistoryItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptHistoryItem {
    pub id: i64,
    pub prompt_md: String,
    pub prompt_hash: String,
    pub is_low_info: bool,
    pub matched_draft_id: Option<i64>,
    pub sent_at: String,
    pub created_at: String,
    pub attachments: Vec<PromptAttachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachment {
    pub id: i64,
    pub kind: String,
    pub mime_type: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub placeholder: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachmentDataUrl {
    pub id: i64,
    pub mime_type: String,
    pub data_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSearchResults {
    pub query: String,
    pub items: Vec<PromptSearchResultItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSearchResultItem {
    pub provider: String,
    pub provider_label: String,
    pub session_id: String,
    pub short_session_id: String,
    pub title: String,
    pub project_name: String,
    pub match_kind: String,
    pub match_label: String,
    pub snippet: String,
    pub is_low_info: bool,
    pub sent_at: Option<String>,
    pub updated_at: String,
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

fn record_prompt_event(
    connection: &Connection,
    event: &PromptEvent,
    attachment_root: &Path,
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
        if let Some(matched_draft_id) = clear_matching_copied_draft(
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
        if let Err(error) = store_prompt_event_attachments(
            connection,
            event,
            prompt,
            prompt_event_id,
            attachment_root,
            &now,
        ) {
            eprintln!("提取 prompt 图片附件失败：{error}");
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

#[derive(Debug)]
struct ExtractedPromptImage {
    mime_type: String,
    bytes: Vec<u8>,
    source: String,
}

fn store_prompt_event_attachments(
    connection: &Connection,
    event: &PromptEvent,
    prompt: &str,
    prompt_event_id: i64,
    attachment_root: &Path,
    created_at: &str,
) -> Result<(), String> {
    let images = extract_prompt_images(event, prompt)?;
    if images.is_empty() {
        return Ok(());
    }

    let provider = event.provider.as_str();
    let session_segment = sanitize_path_segment(&event.session_id);
    let attachment_dir = attachment_root.join(provider).join(session_segment);
    fs::create_dir_all(&attachment_dir).map_err(|error| {
        format!(
            "创建 prompt 图片附件目录失败：{}：{error}",
            attachment_dir.display()
        )
    })?;

    for (index, image) in images.into_iter().enumerate() {
        let position = (index + 1) as i64;
        let extension = extension_from_mime_type(&image.mime_type);
        let file_name = format!("{prompt_event_id}-{position}.{extension}");
        let file_path = attachment_dir.join(&file_name);
        fs::write(&file_path, &image.bytes).map_err(|error| {
            format!("写入 prompt 图片附件失败：{}：{error}", file_path.display())
        })?;
        let file_size = i64::try_from(image.bytes.len()).unwrap_or(i64::MAX);
        let file_path_text = file_path.to_string_lossy().into_owned();
        let placeholder = image_placeholder(prompt, position as usize);

        connection
            .execute(
                r#"
                insert or ignore into prompt_event_attachments (
                  prompt_event_id, provider, session_id, kind, mime_type,
                  file_path, file_name, file_size, placeholder, source, position, created_at
                )
                values (?1, ?2, ?3, 'image', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    prompt_event_id,
                    provider,
                    event.session_id,
                    image.mime_type,
                    file_path_text,
                    file_name,
                    file_size,
                    placeholder,
                    image.source,
                    position,
                    created_at,
                ],
            )
            .map_err(|error| format!("写入 prompt 图片附件记录失败：{error}"))?;
    }

    Ok(())
}

fn extract_prompt_images(
    event: &PromptEvent,
    prompt: &str,
) -> Result<Vec<ExtractedPromptImage>, String> {
    if !prompt_may_have_image(prompt) && !json_may_have_image(&event.raw_json) {
        return Ok(Vec::new());
    }

    let mut images = extract_images_from_json_value(&event.raw_json, prompt, "hook_raw_json");
    if !images.is_empty() {
        return Ok(images);
    }

    for transcript_path in prompt_transcript_paths(event) {
        images = extract_images_from_transcript(&transcript_path, prompt)?;
        if !images.is_empty() {
            return Ok(images);
        }
    }

    Ok(Vec::new())
}

fn prompt_transcript_paths(event: &PromptEvent) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(transcript_path) = event.transcript_path.as_deref() {
        paths.push(PathBuf::from(transcript_path));
    }
    if event.provider == Provider::Codex {
        paths.extend(find_codex_transcript_paths(&event.session_id));
    }

    let mut unique_paths = Vec::new();
    for path in paths {
        let key = path.to_string_lossy().to_ascii_lowercase();
        if unique_paths
            .iter()
            .any(|existing: &PathBuf| existing.to_string_lossy().to_ascii_lowercase() == key)
        {
            continue;
        }
        unique_paths.push(path);
    }

    unique_paths
}

fn extract_images_from_transcript(
    transcript_path: &Path,
    prompt: &str,
) -> Result<Vec<ExtractedPromptImage>, String> {
    if !transcript_path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(transcript_path).map_err(|error| {
        format!(
            "读取 Agent transcript 失败：{}：{error}",
            transcript_path.display()
        )
    })?;
    let source = format!("transcript:{}", transcript_path.display());

    for line in raw.lines().rev() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let images = extract_images_from_json_value(&value, prompt, &source);
        if !images.is_empty() {
            return Ok(images);
        }
    }

    Ok(Vec::new())
}

fn extract_images_from_json_value(
    value: &Value,
    prompt: &str,
    source: &str,
) -> Vec<ExtractedPromptImage> {
    let mut images = Vec::new();
    collect_images_from_json_value(value, prompt, source, false, &mut images);
    images
}

fn collect_images_from_json_value(
    value: &Value,
    prompt: &str,
    source: &str,
    user_context: bool,
    images: &mut Vec<ExtractedPromptImage>,
) {
    match value {
        Value::Object(map) => {
            let next_user_context = user_context || object_is_user_message(value);
            if let Some(content) = map.get("content").and_then(Value::as_array) {
                if next_user_context && content_matches_prompt(content, prompt) {
                    append_images_from_content(content, source, images);
                }
            }
            for child in map.values() {
                collect_images_from_json_value(child, prompt, source, next_user_context, images);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_images_from_json_value(item, prompt, source, user_context, images);
            }
        }
        _ => {}
    }
}

fn object_is_user_message(value: &Value) -> bool {
    string_value_at(value, &["role"]).is_some_and(is_user_marker)
        || string_value_at(value, &["type"]).is_some_and(is_user_marker)
        || string_value_at(value, &["message", "role"]).is_some_and(is_user_marker)
        || string_value_at(value, &["message", "type"]).is_some_and(is_user_marker)
        || string_value_at(value, &["payload", "role"]).is_some_and(is_user_marker)
        || string_value_at(value, &["payload", "type"]).is_some_and(is_user_marker)
}

fn is_user_marker(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "user" | "user_message" | "input"
    )
}

fn string_value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn content_matches_prompt(content: &[Value], prompt: &str) -> bool {
    let prompt = normalize_prompt_for_match(prompt);
    if prompt.is_empty() {
        return false;
    }

    let texts = content
        .iter()
        .filter_map(content_text)
        .map(normalize_prompt_for_match)
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>();

    if texts.iter().any(|text| text == &prompt) {
        return true;
    }

    let joined = normalize_prompt_for_match(&texts.join("\n"));
    joined == prompt
}

fn content_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) => Some(text),
        Value::Object(map) => map
            .get("text")
            .and_then(Value::as_str)
            .or_else(|| map.get("input_text").and_then(Value::as_str)),
        _ => None,
    }
}

fn append_images_from_content(
    content: &[Value],
    source: &str,
    images: &mut Vec<ExtractedPromptImage>,
) {
    for item in content {
        if let Some((mime_type, bytes)) = image_bytes_from_content_item(item) {
            images.push(ExtractedPromptImage {
                mime_type,
                bytes,
                source: source.to_string(),
            });
        }
    }
}

fn image_bytes_from_content_item(value: &Value) -> Option<(String, Vec<u8>)> {
    let object = value.as_object()?;

    if let Some(image_url) = object.get("image_url").and_then(Value::as_str) {
        if let Some(decoded) = decode_image_data_url(image_url) {
            return Some(decoded);
        }
    }

    if let Some(source) = object.get("source").and_then(Value::as_object) {
        let data = source.get("data").and_then(Value::as_str)?;
        let mime_type = source
            .get("media_type")
            .and_then(Value::as_str)
            .or_else(|| source.get("mime_type").and_then(Value::as_str))
            .unwrap_or("image/png");
        return decode_base64_image(mime_type, data);
    }

    if let Some(data) = object.get("data").and_then(Value::as_str) {
        let mime_type = object
            .get("media_type")
            .and_then(Value::as_str)
            .or_else(|| object.get("mime_type").and_then(Value::as_str))
            .unwrap_or("image/png");
        return decode_base64_image(mime_type, data);
    }

    None
}

fn decode_image_data_url(value: &str) -> Option<(String, Vec<u8>)> {
    let (metadata, data) = value.split_once(',')?;
    let metadata = metadata.trim();
    if !metadata.starts_with("data:") || !metadata.ends_with(";base64") {
        return None;
    }

    let mime_type = metadata
        .trim_start_matches("data:")
        .trim_end_matches(";base64")
        .split(';')
        .next()
        .unwrap_or("image/png")
        .trim();
    decode_base64_image(mime_type, data)
}

fn decode_base64_image(mime_type: &str, data: &str) -> Option<(String, Vec<u8>)> {
    let mime_type = normalize_image_mime_type(mime_type);
    if !mime_type.starts_with("image/") {
        return None;
    }
    let normalized_data = data
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    let bytes = BASE64_STANDARD.decode(normalized_data).ok()?;
    if bytes.is_empty() {
        return None;
    }

    Some((mime_type, bytes))
}

fn normalize_image_mime_type(value: &str) -> String {
    let normalized = value
        .trim()
        .split(';')
        .next()
        .unwrap_or("image/png")
        .to_ascii_lowercase();
    if normalized.starts_with("image/") {
        normalized
    } else {
        "image/png".to_string()
    }
}

fn normalize_prompt_for_match(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn prompt_may_have_image(prompt: &str) -> bool {
    prompt.contains("[Image #") || prompt.contains("[图片 #")
}

fn json_may_have_image(value: &Value) -> bool {
    match value {
        Value::String(text) => text.starts_with("data:image/"),
        Value::Object(map) => map.iter().any(|(key, value)| {
            matches!(
                key.as_str(),
                "image_url" | "media_type" | "mime_type" | "source"
            ) || json_may_have_image(value)
        }),
        Value::Array(items) => items.iter().any(json_may_have_image),
        _ => false,
    }
}

fn find_codex_transcript_paths(session_id: &str) -> Vec<PathBuf> {
    let Some(home) = env::var_os("USERPROFILE").or_else(|| env::var_os("HOME")) else {
        return Vec::new();
    };
    let sessions_dir = PathBuf::from(home).join(".codex").join("sessions");
    let mut paths = Vec::new();
    collect_codex_transcript_paths(&sessions_dir, session_id, &mut paths, 0);
    paths.sort();
    paths.reverse();
    paths.truncate(8);
    paths
}

fn collect_codex_transcript_paths(
    dir: &Path,
    session_id: &str,
    paths: &mut Vec<PathBuf>,
    depth: usize,
) {
    if depth > 8 || paths.len() >= 24 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_codex_transcript_paths(&path, session_id, paths, depth + 1);
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if file_name.contains(session_id) && file_name.ends_with(".jsonl") {
            paths.push(path);
        }
    }
}

fn sanitize_path_segment(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();

    if sanitized.is_empty() {
        "unknown-session".to_string()
    } else {
        sanitized
    }
}

fn image_placeholder(prompt: &str, position: usize) -> Option<String> {
    let placeholder = format!("[Image #{position}]");
    prompt.contains(&placeholder).then_some(placeholder)
}

fn extension_from_mime_type(mime_type: &str) -> &'static str {
    match mime_type.trim().to_ascii_lowercase().as_str() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/bmp" => "bmp",
        "image/svg+xml" => "svg",
        _ => "png",
    }
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

fn get_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let draft_id = preferred_editing_draft_id(connection, session_db_id)?;
    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

fn list_drafts(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftList, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    ensure_session_has_editing_draft(connection, session_db_id)?;

    let mut statement = connection
        .prepare(
            r#"
            select
              id,
              content_md,
              content_hash,
              status,
              copy_state,
              copied_at,
              last_copied_hash,
              sent_at,
              matched_prompt_event_id,
              updated_at
            from draft_items
            where session_db_id = ?1
            order by
              case when status = 'sent' then 1 else 0 end,
              updated_at desc,
              id desc
            "#,
        )
        .map_err(|error| format!("准备读取草稿列表失败：{error}"))?;
    let rows = statement
        .query_map(params![session_db_id], |row| {
            let content_md: String = row.get(1)?;
            Ok(DraftListItem {
                id: row.get(0)?,
                provider: provider.to_string(),
                session_id: session_id.to_string(),
                content_hash: row.get(2)?,
                status: row.get(3)?,
                copy_state: row.get(4)?,
                copied_at: row.get(5)?,
                last_copied_hash: row.get(6)?,
                sent_at: row.get(7)?,
                matched_prompt_event_id: row.get(8)?,
                updated_at: row.get(9)?,
                is_empty: content_md.trim().is_empty(),
                preview: draft_preview(&content_md),
                content_md,
            })
        })
        .map_err(|error| format!("读取草稿列表失败：{error}"))?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| format!("解析草稿列表失败：{error}"))?);
    }

    Ok(DraftList {
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        items,
    })
}

fn get_draft_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

fn create_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let draft_id = insert_empty_draft(connection, session_db_id)?;
    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

fn delete_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
) -> Result<DraftList, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
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

fn save_draft(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let draft_id = preferred_editing_draft_id(connection, session_db_id)?;
    save_draft_by_id(connection, provider, session_id, draft_id, content_md)
}

fn save_draft_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let status = draft_status(connection, session_db_id, draft_id)?;
    if status == "sent" {
        return Err("已发送草稿不能继续编辑，请新建草稿".to_string());
    }

    let content_hash = prompt_hash(content_md);
    let existing_last_copied_hash = connection
        .query_row(
            "select last_copied_hash from draft_items where id = ?1 and session_db_id = ?2",
            params![draft_id, session_db_id],
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
            update draft_items
            set content_md = ?1,
                content_hash = ?2,
                copy_state = ?3,
                updated_at = ?4
            where id = ?5
              and session_db_id = ?6
            "#,
            params![
                content_md,
                content_hash,
                copy_state,
                now,
                draft_id,
                session_db_id
            ],
        )
        .map_err(|error| format!("保存当前草稿失败：{error}"))?;

    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

fn mark_draft_copied(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let draft_id = preferred_editing_draft_id(connection, session_db_id)?;
    mark_draft_copied_by_id(connection, provider, session_id, draft_id, content_md)
}

fn mark_draft_copied_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    draft_id: i64,
    content_md: &str,
) -> Result<DraftState, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let status = draft_status(connection, session_db_id, draft_id)?;
    if status == "sent" {
        return Err("已发送草稿不能再次标记为待发送".to_string());
    }

    let content_hash = prompt_hash(content_md);
    let now = current_captured_at();

    connection
        .execute(
            r#"
            update draft_items
            set content_md = ?1,
                content_hash = ?2,
                copy_state = 'copied',
                copied_at = ?3,
                last_copied_hash = ?2,
                updated_at = ?3
            where id = ?4
              and session_db_id = ?5
            "#,
            params![content_md, content_hash, now, draft_id, session_db_id],
        )
        .map_err(|error| format!("记录当前草稿复制状态失败：{error}"))?;

    draft_state_by_id(connection, provider, session_id, session_db_id, draft_id)
}

fn draft_state_by_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    session_db_id: i64,
    draft_id: i64,
) -> Result<DraftState, String> {
    connection
        .query_row(
            r#"
            select
              id,
              content_md,
              content_hash,
              status,
              copy_state,
              copied_at,
              last_copied_hash,
              sent_at,
              matched_prompt_event_id,
              updated_at
            from draft_items
            where session_db_id = ?1
              and id = ?2
            "#,
            params![session_db_id, draft_id],
            |row| {
                let content_md: String = row.get(1)?;
                Ok(DraftState {
                    id: row.get(0)?,
                    provider: provider.to_string(),
                    session_id: session_id.to_string(),
                    content_hash: row.get(2)?,
                    status: row.get(3)?,
                    copy_state: row.get(4)?,
                    copied_at: row.get(5)?,
                    last_copied_hash: row.get(6)?,
                    sent_at: row.get(7)?,
                    matched_prompt_event_id: row.get(8)?,
                    updated_at: row.get(9)?,
                    is_empty: content_md.trim().is_empty(),
                    content_md,
                })
            },
        )
        .map_err(|error| format!("读取当前草稿失败：{error}"))
}

fn preferred_editing_draft_id(connection: &Connection, session_db_id: i64) -> Result<i64, String> {
    ensure_session_has_editing_draft(connection, session_db_id)?;
    connection
        .query_row(
            r#"
            select id
            from draft_items
            where session_db_id = ?1
              and status != 'sent'
            order by updated_at desc, id desc
            limit 1
            "#,
            params![session_db_id],
            |row| row.get(0),
        )
        .map_err(|error| format!("读取当前草稿 ID 失败：{error}"))
}

fn ensure_session_has_editing_draft(
    connection: &Connection,
    session_db_id: i64,
) -> Result<(), String> {
    let has_editing = connection
        .query_row(
            "select exists(select 1 from draft_items where session_db_id = ?1 and status != 'sent')",
            params![session_db_id],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|error| format!("检查当前会话草稿失败：{error}"))?
        != 0;

    if !has_editing {
        insert_empty_draft(connection, session_db_id)?;
    }

    Ok(())
}

fn insert_empty_draft(connection: &Connection, session_db_id: i64) -> Result<i64, String> {
    let now = current_captured_at();
    connection
        .execute(
            r#"
            insert into draft_items (
              session_db_id, content_md, content_hash, status, copy_state,
              created_at, updated_at
            )
            values (?1, '', ?2, 'editing', 'idle', ?3, ?3)
            "#,
            params![session_db_id, prompt_hash(""), now],
        )
        .map_err(|error| format!("创建空草稿失败：{error}"))?;

    Ok(connection.last_insert_rowid())
}

fn draft_status(
    connection: &Connection,
    session_db_id: i64,
    draft_id: i64,
) -> Result<String, String> {
    connection
        .query_row(
            "select status from draft_items where id = ?1 and session_db_id = ?2",
            params![draft_id, session_db_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("读取草稿状态失败：{error}"))?
        .ok_or_else(|| "草稿不存在或不属于当前会话".to_string())
}

fn clear_matching_copied_draft(
    connection: &Connection,
    session_db_id: i64,
    sent_prompt_hash: &str,
    now: &str,
    prompt_event_id: i64,
) -> Result<Option<i64>, String> {
    let matched_draft_id = connection
        .query_row(
            r#"
            select id
            from draft_items
            where session_db_id = ?1
              and content_hash = ?2
              and last_copied_hash = ?2
              and trim(content_md) != ''
              and status != 'sent'
            order by copied_at desc, updated_at desc, id desc
            limit 1
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
                update draft_items
                set status = 'sent',
                    copy_state = 'cleared_after_send',
                    sent_at = ?1,
                    matched_prompt_event_id = ?2,
                    updated_at = ?1
                where id = ?3
                "#,
                params![now, prompt_event_id, draft_id],
            )
            .map_err(|error| format!("标记已发送草稿失败：{error}"))?;
        insert_empty_draft(connection, session_db_id)?;
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

fn draft_preview(content_md: &str) -> String {
    let preview = content_md.split_whitespace().collect::<Vec<_>>().join(" ");
    if preview.is_empty() {
        "空草稿".to_string()
    } else {
        preview.chars().take(80).collect()
    }
}

fn list_prompt_history(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    include_low_info: bool,
    attachment_root: &Path,
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
                    attachments: Vec::new(),
                })
            },
        )
        .map_err(|error| format!("读取 prompt 历史失败：{error}"))?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(|error| format!("解析 prompt 历史失败：{error}"))?);
    }
    backfill_missing_prompt_attachments(connection, provider, session_id, &items, attachment_root)?;
    append_prompt_history_attachments(connection, &mut items)?;

    Ok(PromptHistory {
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        items,
    })
}

fn backfill_missing_prompt_attachments(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    items: &[PromptHistoryItem],
    attachment_root: &Path,
) -> Result<(), String> {
    if !items
        .iter()
        .any(|item| prompt_may_have_image(&item.prompt_md))
    {
        return Ok(());
    }

    let transcript_path = connection
        .query_row(
            r#"
            select transcript_path
            from sessions
            where provider = ?1 and session_id = ?2
            "#,
            params![provider, session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(|error| format!("读取会话 transcript 路径失败：{error}"))?
        .flatten();
    let provider = Provider::parse(provider)?;

    let mut count_statement = connection
        .prepare("select count(*) from prompt_event_attachments where prompt_event_id = ?1")
        .map_err(|error| format!("准备检查 prompt 图片附件失败：{error}"))?;

    for item in items {
        if !prompt_may_have_image(&item.prompt_md) {
            continue;
        }
        let attachment_count = count_statement
            .query_row(params![item.id], |row| row.get::<_, i64>(0))
            .map_err(|error| format!("检查 prompt 图片附件失败：{error}"))?;
        if attachment_count > 0 {
            continue;
        }

        let event = PromptEvent {
            provider,
            event_name: "UserPromptSubmit".to_string(),
            session_id: session_id.to_string(),
            turn_id: None,
            cwd: None,
            transcript_path: transcript_path.clone(),
            model: None,
            prompt: Some(item.prompt_md.clone()),
            captured_at: item.sent_at.clone(),
            raw_json: Value::Null,
        };

        if let Err(error) = store_prompt_event_attachments(
            connection,
            &event,
            &item.prompt_md,
            item.id,
            attachment_root,
            &item.created_at,
        ) {
            eprintln!("回填 prompt 图片附件失败：{error}");
        }
    }

    Ok(())
}

fn append_prompt_history_attachments(
    connection: &Connection,
    items: &mut [PromptHistoryItem],
) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            r#"
            select id, kind, mime_type, file_path, file_name, file_size, placeholder, created_at
            from prompt_event_attachments
            where prompt_event_id = ?1
            order by position asc, id asc
            "#,
        )
        .map_err(|error| format!("准备读取 prompt 图片附件失败：{error}"))?;

    for item in items {
        let rows = statement
            .query_map(params![item.id], |row| {
                Ok(PromptAttachment {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    mime_type: row.get(2)?,
                    file_path: row.get(3)?,
                    file_name: row.get(4)?,
                    file_size: row.get(5)?,
                    placeholder: row.get(6)?,
                    created_at: row.get(7)?,
                })
            })
            .map_err(|error| format!("读取 prompt 图片附件失败：{error}"))?;

        let mut attachments = Vec::new();
        for row in rows {
            attachments.push(row.map_err(|error| format!("解析 prompt 图片附件失败：{error}"))?);
        }
        item.attachments = attachments;
    }

    Ok(())
}

fn read_prompt_attachment_data_url(
    connection: &Connection,
    attachment_id: i64,
) -> Result<PromptAttachmentDataUrl, String> {
    let (mime_type, file_path): (String, String) = connection
        .query_row(
            r#"
            select mime_type, file_path
            from prompt_event_attachments
            where id = ?1
            "#,
            params![attachment_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|error| format!("读取 prompt 图片附件记录失败：{error}"))?
        .ok_or_else(|| format!("prompt 图片附件不存在：{attachment_id}"))?;

    let bytes = fs::read(&file_path)
        .map_err(|error| format!("读取 prompt 图片附件文件失败：{file_path}：{error}"))?;
    let encoded = BASE64_STANDARD.encode(bytes);

    Ok(PromptAttachmentDataUrl {
        id: attachment_id,
        mime_type: mime_type.clone(),
        data_url: format!("data:{mime_type};base64,{encoded}"),
    })
}

fn search_prompts(
    connection: &Connection,
    query: &str,
    include_low_info: bool,
) -> Result<PromptSearchResults, String> {
    let query = query.trim();
    if query.is_empty() {
        return Ok(PromptSearchResults {
            query: String::new(),
            items: Vec::new(),
        });
    }

    let pattern = format!("%{query}%");
    let mut items = Vec::new();
    append_session_search_results(connection, query, &pattern, include_low_info, &mut items)?;
    append_prompt_search_results(connection, &pattern, include_low_info, &mut items)?;
    append_draft_search_results(connection, &pattern, &mut items)?;

    items.sort_by(|left, right| {
        right
            .updated_at
            .cmp(&left.updated_at)
            .then_with(|| right.sent_at.cmp(&left.sent_at))
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    items.truncate(80);

    Ok(PromptSearchResults {
        query: query.to_string(),
        items,
    })
}

fn append_session_search_results(
    connection: &Connection,
    query: &str,
    pattern: &str,
    include_low_info: bool,
    items: &mut Vec<PromptSearchResultItem>,
) -> Result<(), String> {
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

    Ok(())
}

fn append_prompt_search_results(
    connection: &Connection,
    pattern: &str,
    include_low_info: bool,
    items: &mut Vec<PromptSearchResultItem>,
) -> Result<(), String> {
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

    Ok(())
}

fn append_draft_search_results(
    connection: &Connection,
    pattern: &str,
    items: &mut Vec<PromptSearchResultItem>,
) -> Result<(), String> {
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

    Ok(())
}

fn search_result(
    provider: &str,
    session_id: &str,
    cwd: Option<&str>,
    title: &str,
    match_kind: &str,
    match_label: &str,
    snippet_source: &str,
    is_low_info: bool,
    sent_at: Option<String>,
    updated_at: &str,
) -> PromptSearchResultItem {
    PromptSearchResultItem {
        provider: provider.to_string(),
        provider_label: provider_label(provider).to_string(),
        session_id: session_id.to_string(),
        short_session_id: short_session_id(session_id),
        title: title.to_string(),
        project_name: cwd
            .map(project_name_from_cwd)
            .unwrap_or_else(|| "未知项目".to_string()),
        match_kind: match_kind.to_string(),
        match_label: match_label.to_string(),
        snippet: snippet(snippet_source),
        is_low_info,
        sent_at,
        updated_at: updated_at.to_string(),
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

fn delete_session(
    connection: &Connection,
    provider: &str,
    session_id: &str,
    attachment_root: &Path,
) -> Result<DeleteSessionOutcome, String> {
    let session_db_id = session_db_id(connection, provider, session_id)?;
    let attachment_files = session_attachment_files(connection, session_db_id)?;
    let prompt_events_deleted = count_session_rows(connection, "prompt_events", session_db_id)?;
    let drafts_deleted = count_session_rows(connection, "draft_items", session_db_id)?
        + count_session_rows(connection, "drafts", session_db_id)?;
    let attachments_deleted = attachment_files.len();

    let mut files_deleted = 0_usize;
    for file_path in &attachment_files {
        if remove_prompt_attachment_file(file_path, attachment_root)? {
            files_deleted += 1;
        }
    }

    connection
        .execute(
            r#"
            delete from prompt_event_attachments
            where prompt_event_id in (
              select id from prompt_events where session_db_id = ?1
            )
            "#,
            params![session_db_id],
        )
        .map_err(|error| format!("删除 prompt 图片附件记录失败：{error}"))?;
    connection
        .execute(
            "delete from prompt_events where session_db_id = ?1",
            params![session_db_id],
        )
        .map_err(|error| format!("删除已发送 prompt 记录失败：{error}"))?;
    connection
        .execute(
            "delete from draft_items where session_db_id = ?1",
            params![session_db_id],
        )
        .map_err(|error| format!("删除草稿列表失败：{error}"))?;
    connection
        .execute(
            "delete from drafts where session_db_id = ?1",
            params![session_db_id],
        )
        .map_err(|error| format!("删除兼容草稿记录失败：{error}"))?;
    connection
        .execute(
            "delete from raw_hook_events where provider = ?1 and session_id = ?2",
            params![provider, session_id],
        )
        .map_err(|error| format!("删除 raw hook 记录失败：{error}"))?;
    connection
        .execute("delete from sessions where id = ?1", params![session_db_id])
        .map_err(|error| format!("删除 Agent 会话失败：{error}"))?;

    Ok(DeleteSessionOutcome {
        deleted: true,
        provider: provider.to_string(),
        session_id: session_id.to_string(),
        prompt_events_deleted,
        drafts_deleted,
        attachments_deleted,
        files_deleted,
        message: "已删除 PromptHarbor 本地会话记录；不会删除 Claude Code 或 Codex CLI 原始会话文件"
            .to_string(),
    })
}

fn session_attachment_files(
    connection: &Connection,
    session_db_id: i64,
) -> Result<Vec<PathBuf>, String> {
    let mut statement = connection
        .prepare(
            r#"
            select distinct prompt_event_attachments.file_path
            from prompt_event_attachments
            join prompt_events on prompt_events.id = prompt_event_attachments.prompt_event_id
            where prompt_events.session_db_id = ?1
            "#,
        )
        .map_err(|error| format!("准备读取会话附件文件失败：{error}"))?;
    let rows = statement
        .query_map(params![session_db_id], |row| row.get::<_, String>(0))
        .map_err(|error| format!("读取会话附件文件失败：{error}"))?;

    let mut files = Vec::new();
    for row in rows {
        files.push(PathBuf::from(
            row.map_err(|error| format!("解析会话附件文件失败：{error}"))?,
        ));
    }
    Ok(files)
}

fn remove_prompt_attachment_file(file_path: &Path, attachment_root: &Path) -> Result<bool, String> {
    if !file_path.exists() {
        return Ok(false);
    }

    let canonical_root = attachment_root.canonicalize().map_err(|error| {
        format!(
            "解析 PromptHarbor 附件目录失败：{}：{error}",
            attachment_root.display()
        )
    })?;
    let canonical_file = file_path.canonicalize().map_err(|error| {
        format!(
            "解析 PromptHarbor 附件文件失败：{}：{error}",
            file_path.display()
        )
    })?;
    if !canonical_file.starts_with(&canonical_root) {
        return Err(format!(
            "拒绝删除附件目录外的文件：{}",
            canonical_file.display()
        ));
    }

    fs::remove_file(&canonical_file).map_err(|error| {
        format!(
            "删除 PromptHarbor 附件文件失败：{}：{error}",
            canonical_file.display()
        )
    })?;
    Ok(true)
}

fn count_session_rows(
    connection: &Connection,
    table: &str,
    session_db_id: i64,
) -> Result<usize, String> {
    let sql = format!("select count(*) from {table} where session_db_id = ?1");
    connection
        .query_row(&sql, params![session_db_id], |row| row.get::<_, i64>(0))
        .map(|count| count as usize)
        .map_err(|error| format!("读取会话数据计数失败：{table}：{error}"))
}

fn session_has_non_empty_draft(
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

fn count_table(connection: &Connection, table: &str) -> Result<usize, String> {
    let sql = format!("select count(*) from {table}");
    connection
        .query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count as usize)
        .map_err(|error| format!("读取数据表计数失败：{table}：{error}"))
}

fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
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

fn contains_query(value: &str, query: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
}

fn snippet(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut snippet = collapsed.chars().take(160).collect::<String>();
    if collapsed.chars().count() > 160 {
        snippet.push_str("...");
    }
    snippet
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

        store
            .save_draft("claude", "draft-session", "draft text")
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
    fn matching_copied_prompt_marks_draft_sent_and_creates_empty_draft() {
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
        let drafts = store.list_drafts("claude", "draft-session").unwrap();
        let connection = store.open_connection().unwrap();
        let matched_count: i64 = connection
            .query_row(
                "select count(*) from prompt_events where matched_draft_id is not null",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(draft.is_empty);
        assert_eq!(draft.copy_state, "idle");
        assert!(drafts
            .items
            .iter()
            .any(|item| item.status == "sent" && item.copy_state == "cleared_after_send"));
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
    fn prompt_history_keeps_low_info_but_can_hide_it() {
        let home = isolated_home("history-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        store
            .record_prompt_event(&test_event("claude", "history-session", "hi"))
            .unwrap();
        store
            .record_prompt_event(&test_event(
                "claude",
                "history-session",
                "implement prompt history",
            ))
            .unwrap();

        let visible = store
            .list_prompt_history("claude", "history-session", false)
            .unwrap();
        let all = store
            .list_prompt_history("claude", "history-session", true)
            .unwrap();

        assert_eq!(visible.items.len(), 1);
        assert_eq!(all.items.len(), 2);
        assert!(all.items.iter().any(|item| item.is_low_info));
    }

    #[test]
    fn search_covers_session_prompt_history_and_current_draft() {
        let home = isolated_home("search-store");
        let store = PromptStore::new(home.join("promptbox.sqlite"));
        store.initialize().unwrap();
        store
            .record_prompt_event(&test_event(
                "claude",
                "search-session",
                "implement prompt search",
            ))
            .unwrap();
        store
            .save_draft("claude", "search-session", "draft search query")
            .unwrap();

        let results = store.search_prompts("search", false).unwrap();
        let match_kinds = results
            .items
            .iter()
            .map(|item| item.match_kind.as_str())
            .collect::<Vec<_>>();

        assert!(match_kinds.contains(&"session_title"));
        assert!(match_kinds.contains(&"first_prompt"));
        assert!(match_kinds.contains(&"sent_prompt"));
        assert!(match_kinds.contains(&"current_draft"));
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
