use crate::state::{current_app_status, StartupState};
use promptbox_core::AppStatus;

#[tauri::command]
pub(crate) fn app_status(state: tauri::State<'_, StartupState>) -> AppStatus {
    current_app_status(state.inner())
}
