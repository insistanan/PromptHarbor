use crate::state::StartupState;
use promptbox_core::{PromptAttachmentDataUrl, PromptHistory, PromptSearchResults};

#[tauri::command]
pub(crate) fn list_prompt_history(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    include_low_info: bool,
) -> Result<PromptHistory, String> {
    state
        .store()?
        .list_prompt_history(&provider, &session_id, include_low_info)
}

#[tauri::command]
pub(crate) fn read_prompt_attachment_data_url(
    state: tauri::State<'_, StartupState>,
    attachment_id: i64,
) -> Result<PromptAttachmentDataUrl, String> {
    state.store()?.read_prompt_attachment_data_url(attachment_id)
}

#[tauri::command]
pub(crate) fn search_prompts(
    state: tauri::State<'_, StartupState>,
    query: String,
    include_low_info: bool,
) -> Result<PromptSearchResults, String> {
    state.store()?.search_prompts(&query, include_low_info)
}
