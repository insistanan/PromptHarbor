use crate::state::StartupState;
use promptbox_core::{DraftList, DraftState};

#[tauri::command]
pub(crate) fn get_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DraftState, String> {
    state.store()?.get_draft(&provider, &session_id)
}

#[tauri::command]
pub(crate) fn list_drafts(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DraftList, String> {
    state.store()?.list_drafts(&provider, &session_id)
}

#[tauri::command]
pub(crate) fn get_draft_by_id(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
) -> Result<DraftState, String> {
    state
        .store()?
        .get_draft_by_id(&provider, &session_id, draft_id)
}

#[tauri::command]
pub(crate) fn create_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DraftState, String> {
    state.store()?.create_draft(&provider, &session_id)
}

#[tauri::command]
pub(crate) fn delete_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
) -> Result<DraftList, String> {
    state.store()?.delete_draft(&provider, &session_id, draft_id)
}

#[tauri::command]
pub(crate) fn save_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    content_md: String,
) -> Result<DraftState, String> {
    state.store()?.save_draft(&provider, &session_id, &content_md)
}

#[tauri::command]
pub(crate) fn save_draft_by_id(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
    content_md: String,
) -> Result<DraftState, String> {
    state
        .store()?
        .save_draft_by_id(&provider, &session_id, draft_id, &content_md)
}

#[tauri::command]
pub(crate) fn mark_draft_copied(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    content_md: String,
) -> Result<DraftState, String> {
    state
        .store()?
        .mark_draft_copied(&provider, &session_id, &content_md)
}

#[tauri::command]
pub(crate) fn mark_draft_copied_by_id(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
    content_md: String,
) -> Result<DraftState, String> {
    state
        .store()?
        .mark_draft_copied_by_id(&provider, &session_id, draft_id, &content_md)
}
