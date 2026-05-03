mod event;
mod runtime;

pub use event::{
    append_spool_event, current_captured_at, endpoint_host_port, import_spool_events,
    normalize_hook_input, parse_local_endpoint, PromptEvent, Provider, HOOK_EVENTS_PATH,
    MAX_HOOK_BODY_BYTES,
};
pub use runtime::{
    app_status_from_error, load_config_for_hook, resolve_promptbox_paths, AppStatus,
    HookBinaryStatus, PromptBoxConfig, PromptBoxPaths, RuntimeState, APP_DISPLAY_NAME, APP_NAME,
    DEFAULT_LOCAL_ENDPOINT, HOOK_PROTOCOL_VERSION,
};

pub fn initialize_runtime() -> Result<RuntimeState, String> {
    RuntimeState::initialize()
}
