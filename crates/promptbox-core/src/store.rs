use crate::PromptEvent;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

mod attachments;
mod drafts;
mod retrieval;
mod schema;
mod sessions;
mod text;
mod types;

pub use attachments::{PromptAttachment, PromptAttachmentDataUrl};
pub use types::{
    ArchiveSessionOutcome, DeleteSessionOutcome, DraftList, DraftListItem, DraftState,
    PromptHistory, PromptHistoryItem, PromptSearchResultItem, PromptSearchResults, RecordOutcome,
    SessionList, SessionListItem, StoreSummary,
};

use schema::{migrate, store_summary};

#[derive(Debug, Clone)]
pub struct PromptStore {
    database_path: PathBuf,
}

impl PromptStore {
    pub fn new(database_path: PathBuf) -> Self {
        Self { database_path }
    }

    pub fn initialize(&self) -> Result<StoreSummary, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        store_summary(&connection)
    }

    pub fn record_prompt_event(&self, event: &PromptEvent) -> Result<RecordOutcome, String> {
        let outcome = self.record_prompt_event_without_attachments(event)?;
        if let (Some(prompt_event_id), Some(prompt)) = (
            outcome.prompt_event_id,
            event.prompt
                .as_ref()
                .map(|prompt| prompt.trim())
                .filter(|prompt| !prompt.is_empty()),
        ) {
            if let Err(error) = self.capture_prompt_event_attachments(event, prompt_event_id, prompt)
            {
                eprintln!("提取 prompt 图片附件失败：{error}");
            }
        }
        Ok(outcome)
    }

    pub fn record_prompt_event_without_attachments(
        &self,
        event: &PromptEvent,
    ) -> Result<RecordOutcome, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        sessions::record_prompt_event(&connection, event)
    }

    pub fn capture_prompt_event_attachments(
        &self,
        event: &PromptEvent,
        prompt_event_id: i64,
        prompt: &str,
    ) -> Result<usize, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        attachments::store_prompt_event_attachments(
            &connection,
            event,
            prompt,
            prompt_event_id,
            &self.attachment_root(),
            &crate::current_captured_at(),
        )
    }

    pub fn summary(&self) -> Result<StoreSummary, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        store_summary(&connection)
    }

    pub fn list_sessions(&self, maybe_closed_after_hours: u64) -> Result<SessionList, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        sessions::update_maybe_closed_sessions(&connection, maybe_closed_after_hours)?;
        sessions::list_sessions(&connection)
    }

    pub fn archive_session(
        &self,
        provider: &str,
        session_id: &str,
        force: bool,
    ) -> Result<ArchiveSessionOutcome, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        sessions::archive_session(&connection, provider, session_id, force)
    }

    pub fn delete_session(
        &self,
        provider: &str,
        session_id: &str,
    ) -> Result<DeleteSessionOutcome, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        sessions::delete_session(&connection, provider, session_id, &self.attachment_root())
    }

    pub fn get_draft(&self, provider: &str, session_id: &str) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::get_draft(&connection, provider, session_id)
    }

    pub fn list_drafts(&self, provider: &str, session_id: &str) -> Result<DraftList, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::list_drafts(&connection, provider, session_id)
    }

    pub fn get_draft_by_id(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::get_draft_by_id(&connection, provider, session_id, draft_id)
    }

    pub fn create_draft(&self, provider: &str, session_id: &str) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::create_draft(&connection, provider, session_id)
    }

    pub fn delete_draft(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
    ) -> Result<DraftList, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::delete_draft(&connection, provider, session_id, draft_id)
    }

    pub fn save_draft(
        &self,
        provider: &str,
        session_id: &str,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::save_draft(&connection, provider, session_id, content_md)
    }

    pub fn save_draft_by_id(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::save_draft_by_id(&connection, provider, session_id, draft_id, content_md)
    }

    pub fn mark_draft_copied(
        &self,
        provider: &str,
        session_id: &str,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::mark_draft_copied(&connection, provider, session_id, content_md)
    }

    pub fn mark_draft_copied_by_id(
        &self,
        provider: &str,
        session_id: &str,
        draft_id: i64,
        content_md: &str,
    ) -> Result<DraftState, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        drafts::mark_draft_copied_by_id(&connection, provider, session_id, draft_id, content_md)
    }

    pub fn list_prompt_history(
        &self,
        provider: &str,
        session_id: &str,
        include_low_info: bool,
    ) -> Result<PromptHistory, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        retrieval::list_prompt_history(
            &connection,
            provider,
            session_id,
            include_low_info,
        )
    }

    pub fn read_prompt_attachment_data_url(
        &self,
        attachment_id: i64,
    ) -> Result<PromptAttachmentDataUrl, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        attachments::read_prompt_attachment_data_url(&connection, attachment_id)
    }

    pub fn search_prompts(
        &self,
        query: &str,
        include_low_info: bool,
    ) -> Result<PromptSearchResults, String> {
        let connection = self.open_connection()?;
        migrate(&connection)?;
        retrieval::search_prompts(&connection, query, include_low_info)
    }

    fn open_connection(&self) -> Result<Connection, String> {
        Connection::open(&self.database_path).map_err(|error| {
            format!(
                "打开 PromptBox 数据库失败：{}：{error}",
                self.database_path.display()
            )
        })
    }

    fn attachment_root(&self) -> PathBuf {
        self.database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("attachments")
    }
}

#[cfg(test)]
mod tests;
