use crate::hook_binary::HOOK_PROTOCOL_VERSION;
use serde::Serialize;
use std::path::Path;

use super::{RuntimeState, APP_DISPLAY_NAME, APP_NAME, DEFAULT_LOCAL_ENDPOINT};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_name: String,
    pub display_name: String,
    pub version: String,
    pub hook_protocol_version: String,
    pub local_endpoint: String,
    pub data_policy: String,
    pub promptbox_home: Option<String>,
    pub config_path: Option<String>,
    pub database_path: Option<String>,
    pub spool_path: Option<String>,
    pub logs_dir: Option<String>,
    pub hook_binary_path: Option<String>,
    pub recording_paused: bool,
    pub maybe_closed_after_hours: u64,
    pub retain_raw_hook_events: bool,
    pub raw_hook_events_retention_days: u64,
    pub autostart: bool,
    pub config_ready: bool,
    pub hook_binary_ready: bool,
    pub hook_binary_message: String,
    pub database_ready: bool,
    pub database_message: String,
    pub session_count: usize,
    pub prompt_event_count: usize,
    pub collector_ready: bool,
    pub collector_message: String,
    pub imported_spool_events: usize,
    pub received_prompt_events: usize,
    pub paused_prompt_events: usize,
    pub startup_errors: Vec<String>,
}

impl RuntimeState {
    pub fn app_status(&self) -> AppStatus {
        AppStatus {
            app_name: APP_NAME.to_string(),
            display_name: APP_DISPLAY_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            hook_protocol_version: HOOK_PROTOCOL_VERSION.to_string(),
            local_endpoint: self.config.local_endpoint.clone(),
            data_policy: "本地优先；默认不联网；只记录用户 prompt".to_string(),
            promptbox_home: Some(path_to_string(&self.paths.home)),
            config_path: Some(path_to_string(&self.paths.config_path)),
            database_path: Some(path_to_string(&self.paths.database_path)),
            spool_path: Some(path_to_string(&self.paths.spool_path)),
            logs_dir: Some(path_to_string(&self.paths.logs_dir)),
            hook_binary_path: Some(path_to_string(&self.paths.hook_binary_path)),
            recording_paused: self.config.recording_paused,
            maybe_closed_after_hours: self.config.maybe_closed_after_hours,
            retain_raw_hook_events: self.config.retain_raw_hook_events,
            raw_hook_events_retention_days: self.config.raw_hook_events_retention_days,
            autostart: self.config.autostart,
            config_ready: true,
            hook_binary_ready: self.hook_binary.ready,
            hook_binary_message: self.hook_binary.message.clone(),
            database_ready: false,
            database_message: "数据库尚未初始化".to_string(),
            session_count: 0,
            prompt_event_count: 0,
            collector_ready: false,
            collector_message: "本地采集端点尚未启动".to_string(),
            imported_spool_events: 0,
            received_prompt_events: 0,
            paused_prompt_events: 0,
            startup_errors: self.startup_errors.clone(),
        }
    }
}

pub fn app_status_from_error(error: String) -> AppStatus {
    AppStatus {
        app_name: APP_NAME.to_string(),
        display_name: APP_DISPLAY_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hook_protocol_version: HOOK_PROTOCOL_VERSION.to_string(),
        local_endpoint: DEFAULT_LOCAL_ENDPOINT.to_string(),
        data_policy: "本地优先；默认不联网；只记录用户 prompt".to_string(),
        promptbox_home: None,
        config_path: None,
        database_path: None,
        spool_path: None,
        logs_dir: None,
        hook_binary_path: None,
        recording_paused: false,
        maybe_closed_after_hours: 12,
        retain_raw_hook_events: true,
        raw_hook_events_retention_days: 7,
        autostart: false,
        config_ready: false,
        hook_binary_ready: false,
        hook_binary_message: "运行时初始化失败".to_string(),
        database_ready: false,
        database_message: "数据库未初始化".to_string(),
        session_count: 0,
        prompt_event_count: 0,
        collector_ready: false,
        collector_message: "本地采集端点未启动".to_string(),
        imported_spool_events: 0,
        received_prompt_events: 0,
        paused_prompt_events: 0,
        startup_errors: vec![error],
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
