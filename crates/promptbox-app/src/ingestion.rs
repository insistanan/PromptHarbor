use crate::{
    collector::CollectorState,
    local_http::{HttpRequest, HttpResponse},
};
use promptbox_core::{PromptEvent, PromptStore, HOOK_EVENTS_PATH};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

const ATTACHMENT_CAPTURE_RETRIES: usize = 12;
const ATTACHMENT_CAPTURE_RETRY_DELAY_MS: u64 = 250;

pub(crate) struct HookEventProcessor<'a> {
    token: &'a str,
    store: &'a PromptStore,
    recording_paused: Arc<AtomicBool>,
    state: CollectorState,
}

impl<'a> HookEventProcessor<'a> {
    pub(crate) fn new(
        token: &'a str,
        store: &'a PromptStore,
        recording_paused: Arc<AtomicBool>,
        state: CollectorState,
    ) -> Self {
        Self {
            token,
            store,
            recording_paused,
            state,
        }
    }

    pub(crate) fn process(&self, request: HttpRequest) -> HttpResponse {
        if request.method != "POST" || request.path != HOOK_EVENTS_PATH {
            return HttpResponse::new(404, "Not Found", "未知采集路径".to_string());
        }

        let expected = format!("Bearer {}", self.token);
        if request.header("authorization") != Some(expected.as_str()) {
            self.state.record_error("token 校验失败".to_string());
            return HttpResponse::new(401, "Unauthorized", "token 校验失败".to_string());
        }

        if self.recording_paused.load(Ordering::SeqCst) {
            self.state.record_paused();
            return HttpResponse::new(204, "No Content", String::new());
        }

        let event = match serde_json::from_slice::<PromptEvent>(&request.body) {
            Ok(event) => event,
            Err(error) => {
                let message = format!("解析 hook 事件失败：{error}");
                self.state.record_error(message.clone());
                return HttpResponse::new(400, "Bad Request", message);
            }
        };

        let outcome = match self.store.record_prompt_event_without_attachments(&event) {
            Ok(outcome) => outcome,
            Err(error) => {
                let message = format!("写入正式历史失败：{error}");
                self.state.record_error(message.clone());
                return HttpResponse::new(500, "Internal Server Error", message);
            }
        };

        if let (Some(prompt_event_id), Some(prompt)) = (
            outcome.prompt_event_id,
            event
                .prompt
                .as_ref()
                .map(|prompt| prompt.trim().to_string())
                .filter(|prompt| !prompt.is_empty()),
        ) {
            spawn_attachment_capture(self.store.clone(), event.clone(), prompt_event_id, prompt);
        }

        self.state.record_received();
        HttpResponse::new(204, "No Content", String::new())
    }
}

fn spawn_attachment_capture(
    store: PromptStore,
    event: PromptEvent,
    prompt_event_id: i64,
    prompt: String,
) {
    thread::spawn(move || {
        for attempt in 0..=ATTACHMENT_CAPTURE_RETRIES {
            match store.capture_prompt_event_attachments(&event, prompt_event_id, &prompt) {
                Ok(captured) if captured > 0 => return,
                Ok(_) if event.transcript_path.is_none() => return,
                Ok(_) if attempt == ATTACHMENT_CAPTURE_RETRIES => return,
                Ok(_) => thread::sleep(Duration::from_millis(ATTACHMENT_CAPTURE_RETRY_DELAY_MS)),
                Err(error) => {
                    eprintln!("提取 prompt 图片附件失败：{error}");
                    return;
                }
            }
        }
    });
}
