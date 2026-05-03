use super::attachments::PromptAttachment;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoreSummary {
    pub session_count: usize,
    pub prompt_event_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionList {
    pub active: Vec<SessionListItem>,
    pub maybe_closed: Vec<SessionListItem>,
    pub archived: Vec<SessionListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionListItem {
    pub provider: String,
    pub provider_label: String,
    pub session_id: String,
    pub short_session_id: String,
    pub status: String,
    pub cwd: Option<String>,
    pub project_name: String,
    pub title: String,
    pub last_hook_at: Option<String>,
    pub updated_at: String,
    pub prompt_count: usize,
    pub has_non_empty_draft: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveSessionOutcome {
    pub archived: bool,
    pub requires_confirmation: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteSessionOutcome {
    pub deleted: bool,
    pub provider: String,
    pub session_id: String,
    pub prompt_events_deleted: usize,
    pub drafts_deleted: usize,
    pub attachments_deleted: usize,
    pub files_deleted: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftState {
    pub id: i64,
    pub provider: String,
    pub session_id: String,
    pub content_md: String,
    pub content_hash: String,
    pub status: String,
    pub copy_state: String,
    pub copied_at: Option<String>,
    pub last_copied_hash: Option<String>,
    pub sent_at: Option<String>,
    pub matched_prompt_event_id: Option<i64>,
    pub updated_at: String,
    pub is_empty: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftList {
    pub provider: String,
    pub session_id: String,
    pub items: Vec<DraftListItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftListItem {
    pub id: i64,
    pub provider: String,
    pub session_id: String,
    pub content_md: String,
    pub content_hash: String,
    pub status: String,
    pub copy_state: String,
    pub copied_at: Option<String>,
    pub last_copied_hash: Option<String>,
    pub sent_at: Option<String>,
    pub matched_prompt_event_id: Option<i64>,
    pub updated_at: String,
    pub is_empty: bool,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordOutcome {
    pub inserted: bool,
    pub ignored_reason: Option<String>,
    pub session_count: usize,
    pub prompt_event_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_event_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptHistory {
    pub provider: String,
    pub session_id: String,
    pub items: Vec<PromptHistoryItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptHistoryItem {
    pub id: i64,
    pub prompt_md: String,
    pub prompt_hash: String,
    pub is_low_info: bool,
    pub matched_draft_id: Option<i64>,
    pub sent_at: String,
    pub created_at: String,
    pub expected_image_count: usize,
    pub captured_image_count: usize,
    pub has_missing_images: bool,
    pub attachments: Vec<PromptAttachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSearchResults {
    pub query: String,
    pub items: Vec<PromptSearchResultItem>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptSearchResultItem {
    pub provider: String,
    pub provider_label: String,
    pub session_id: String,
    pub short_session_id: String,
    pub title: String,
    pub project_name: String,
    pub match_kind: String,
    pub match_label: String,
    pub snippet: String,
    pub is_low_info: bool,
    pub sent_at: Option<String>,
    pub updated_at: String,
}
