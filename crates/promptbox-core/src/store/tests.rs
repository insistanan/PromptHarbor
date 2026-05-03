use super::*;
use crate::{PromptEvent, Provider};
use serde_json::json;
use std::{
    env, fs,
    path::PathBuf,
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
    assert_eq!(copied.content_hash, text::prompt_hash("next prompt"));
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
