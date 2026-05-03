mod claude;
mod codex;
mod event;
mod runtime;
mod store;

pub use claude::{detect_claude_user_hook, install_claude_user_hook, ClaudeHookStatus};
pub use codex::{detect_codex_user_hook, install_codex_user_hook, CodexHookStatus};
pub use event::{
    append_spool_event, clear_spool_events, current_captured_at, endpoint_host_port,
    import_spool_events, normalize_hook_input, parse_local_endpoint, read_spool_events,
    PromptEvent, Provider, HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
pub use runtime::{
    app_status_from_error, load_config_for_hook, resolve_promptbox_paths, AppStatus,
    HookBinaryStatus, PromptBoxConfig, PromptBoxPaths, RuntimeState, APP_DISPLAY_NAME, APP_NAME,
    DEFAULT_LOCAL_ENDPOINT, HOOK_PROTOCOL_VERSION,
};
pub use store::{
    ArchiveSessionOutcome, DraftState, PromptHistory, PromptHistoryItem, PromptSearchResultItem,
    PromptSearchResults, PromptStore, RecordOutcome, SessionList, SessionListItem, StoreSummary,
};

pub fn initialize_runtime() -> Result<RuntimeState, String> {
    RuntimeState::initialize()
}
