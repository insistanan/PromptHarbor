use crate::state::StartupState;
use promptbox_core::{ArchiveSessionOutcome, DeleteSessionOutcome, SessionList};

#[tauri::command]
pub(crate) fn list_sessions(state: tauri::State<'_, StartupState>) -> Result<SessionList, String> {
    let maybe_closed_after_hours = state.maybe_closed_after_hours();
    state.store()?.list_sessions(maybe_closed_after_hours)
}

#[tauri::command]
pub(crate) fn archive_session(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    force: bool,
) -> Result<ArchiveSessionOutcome, String> {
    state.store()?.archive_session(&provider, &session_id, force)
}

#[tauri::command]
pub(crate) fn delete_session(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DeleteSessionOutcome, String> {
    state.store()?.delete_session(&provider, &session_id)
}
