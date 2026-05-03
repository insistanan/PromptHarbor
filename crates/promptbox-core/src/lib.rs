mod runtime;

pub use runtime::{
    app_status_from_error, load_config_for_hook, resolve_promptbox_paths, AppStatus,
    HookBinaryStatus, PromptBoxConfig, PromptBoxPaths, RuntimeState, APP_DISPLAY_NAME, APP_NAME,
    DEFAULT_LOCAL_ENDPOINT, HOOK_PROTOCOL_VERSION,
};

pub fn initialize_runtime() -> Result<RuntimeState, String> {
    RuntimeState::initialize()
}
