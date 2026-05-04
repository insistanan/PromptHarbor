use crate::hook_binary::{HookBinaryManager, HookBinaryStatus};
use std::path::Path;

mod config;
mod paths;
mod status;

pub use config::{load_config_for_hook, PromptBoxConfig};
pub use paths::{resolve_promptbox_paths, PromptBoxPaths};
pub use status::{app_status_from_error, AppStatus};

pub const APP_NAME: &str = "PromptHarbor";
pub const APP_DISPLAY_NAME: &str = "提示港 PromptHarbor";
pub const DEFAULT_LOCAL_ENDPOINT: &str = "127.0.0.1:9996";
const PROMPTBOX_HOME_ENV: &str = "PROMPTBOX_HOME";

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub paths: PromptBoxPaths,
    pub config: PromptBoxConfig,
    pub hook_binary: HookBinaryStatus,
    pub startup_errors: Vec<String>,
}

impl RuntimeState {
    pub fn initialize() -> Result<Self, String> {
        Self::initialize_with_hook_source(None)
    }

    pub fn initialize_with_hook_source(hook_source: Option<&Path>) -> Result<Self, String> {
        let mut startup_errors = Vec::new();
        let paths = PromptBoxPaths::resolve()?;
        paths.ensure_directories()?;

        let (config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
        let hook_binary = HookBinaryManager::ensure(&paths.hook_binary_path, hook_source)
            .unwrap_or_else(|error| {
                let message = error;
                startup_errors.push(message.clone());
                HookBinaryStatus::not_ready(
                    paths.hook_binary_path.clone(),
                    paths.hook_binary_path.exists(),
                    message,
                    None,
                )
            });

        Ok(Self {
            paths,
            config,
            hook_binary,
            startup_errors,
        })
    }
}
