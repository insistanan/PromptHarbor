use serde::Serialize;

mod extraction;
mod history;
mod storage;

pub(super) use self::history::{
    append_prompt_history_attachments, read_prompt_attachment_data_url, session_attachment_files,
};
pub(super) use self::storage::store_prompt_event_attachments;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachment {
    pub id: i64,
    pub kind: String,
    pub mime_type: String,
    pub file_path: String,
    pub file_name: String,
    pub file_size: i64,
    pub placeholder: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptAttachmentDataUrl {
    pub id: i64,
    pub mime_type: String,
    pub data_url: String,
}
