use crate::collector;
use promptbox_core::{AppStatus, PromptStore};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

pub(crate) struct StartupState {
    pub(crate) status: Mutex<AppStatus>,
    pub(crate) collector_state: collector::CollectorState,
    pub(crate) recording_paused: Arc<AtomicBool>,
    pub(crate) store: Option<PromptStore>,
}

impl StartupState {
    pub(crate) fn store(&self) -> Result<&PromptStore, String> {
        self.store
            .as_ref()
            .ok_or_else(|| "数据库尚未初始化".to_string())
    }

    pub(crate) fn maybe_closed_after_hours(&self) -> u64 {
        self.status
            .lock()
            .map(|status| status.maybe_closed_after_hours)
            .unwrap_or(12)
    }
}

pub(crate) fn current_app_status(state: &StartupState) -> AppStatus {
    let mut status = state
        .status
        .lock()
        .map(|status| status.clone())
        .unwrap_or_else(|_| promptbox_core::app_status_from_error("应用状态锁已损坏".to_string()));

    status.recording_paused = state.recording_paused.load(Ordering::SeqCst);
    let collector_snapshot = state.collector_state.snapshot();
    status.received_prompt_events =
        status.imported_spool_events + collector_snapshot.received_prompt_events;
    status.paused_prompt_events = collector_snapshot.paused_prompt_events;
    if let Some(error) = collector_snapshot.recent_error {
        status.collector_message = if status.collector_ready {
            format!("{}；最近错误：{error}", status.collector_message)
        } else {
            error
        };
    }
    if let Some(store) = &state.store {
        let maybe_closed_after_hours = status.maybe_closed_after_hours;
        let _ = store.list_sessions(maybe_closed_after_hours);
        if let Ok(summary) = store.summary() {
            status.database_ready = true;
            status.database_message = "数据库就绪".to_string();
            status.session_count = summary.session_count;
            status.prompt_event_count = summary.prompt_event_count;
        }
    }

    status
}
