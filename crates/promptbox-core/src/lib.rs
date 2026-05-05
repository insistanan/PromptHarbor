mod claude;
mod codex;
mod event;
mod hook_adapter;
mod hook_binary;
mod hook_config;
mod runtime;
mod skills;
mod store;

use std::path::Path;

pub use event::{
    append_spool_event, clear_spool_events, current_captured_at, endpoint_host_port,
    import_spool_events, normalize_hook_input, parse_local_endpoint, read_spool_events,
    PromptEvent, Provider, HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
pub use hook_adapter::{
    detect_user_hook, install_user_hook, uninstall_user_hook, HookAdapter, HookAdapterStatus,
    HookBackupPath, HookConfigPathStatus,
};
pub use hook_binary::{HookBinaryStatus, HOOK_PROTOCOL_VERSION};
pub use runtime::{
    app_status_from_error, load_config_for_hook, resolve_promptbox_paths, AppStatus,
    CustomProviderConfig, CustomProviderProtocol, CustomProviderSummary,
    CustomProviderUpsertInput, PromptBoxConfig, PromptBoxPaths, RuntimeState,
    APP_DISPLAY_NAME, APP_NAME, DEFAULT_LOCAL_ENDPOINT,
};
pub use skills::{list_skills, read_skill_detail, SkillDetail, SkillListItem};
pub use store::{
    ArchiveSessionOutcome, DeleteSessionOutcome, DraftList, DraftListItem, DraftState,
    PromptAttachment, PromptAttachmentDataUrl, PromptHistory, PromptHistoryItem,
    PromptSearchResultItem, PromptSearchResults, PromptStore, RecordOutcome, SessionList,
    SessionListItem, StoreSummary,
};

pub fn initialize_runtime() -> Result<RuntimeState, String> {
    RuntimeState::initialize()
}

pub fn initialize_runtime_with_hook_source(
    hook_source: Option<&Path>,
) -> Result<RuntimeState, String> {
    RuntimeState::initialize_with_hook_source(hook_source)
}
