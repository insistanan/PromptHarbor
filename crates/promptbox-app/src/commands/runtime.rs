use crate::{
    autostart::apply_autostart_setting,
    state::{current_app_status, StartupState},
};
use promptbox_core::{parse_local_endpoint, AppStatus, PromptBoxConfig};
use std::sync::atomic::Ordering;

#[tauri::command]
pub(crate) fn set_recording_paused(
    state: tauri::State<'_, StartupState>,
    paused: bool,
) -> Result<AppStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    let (mut config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
    config.recording_paused = paused;
    config.write(&paths.config_path)?;

    state.recording_paused.store(paused, Ordering::SeqCst);
    if let Ok(mut status) = state.status.lock() {
        status.recording_paused = paused;
    }

    Ok(current_app_status(state.inner()))
}

#[tauri::command]
pub(crate) fn update_runtime_config(
    state: tauri::State<'_, StartupState>,
    local_endpoint: String,
    recording_paused: bool,
    maybe_closed_after_hours: u64,
    retain_raw_hook_events: bool,
    raw_hook_events_retention_days: u64,
    autostart: bool,
) -> Result<AppStatus, String> {
    parse_local_endpoint(&local_endpoint)?;
    if maybe_closed_after_hours == 0 {
        return Err("可能关闭判定时间必须大于 0 小时".to_string());
    }

    let paths = promptbox_core::resolve_promptbox_paths()?;
    let (mut config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
    config.local_endpoint = local_endpoint.trim().to_string();
    config.recording_paused = recording_paused;
    config.maybe_closed_after_hours = maybe_closed_after_hours;
    config.retain_raw_hook_events = retain_raw_hook_events;
    config.raw_hook_events_retention_days = raw_hook_events_retention_days;
    config.autostart = autostart;

    apply_autostart_setting(autostart)?;
    config.write(&paths.config_path)?;

    state
        .recording_paused
        .store(config.recording_paused, Ordering::SeqCst);
    if let Ok(mut status) = state.status.lock() {
        status.local_endpoint = config.local_endpoint;
        status.recording_paused = config.recording_paused;
        status.maybe_closed_after_hours = config.maybe_closed_after_hours;
        status.retain_raw_hook_events = config.retain_raw_hook_events;
        status.raw_hook_events_retention_days = config.raw_hook_events_retention_days;
        status.autostart = config.autostart;
    }

    Ok(current_app_status(state.inner()))
}
