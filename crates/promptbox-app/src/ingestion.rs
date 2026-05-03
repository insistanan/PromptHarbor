use crate::{
    collector::CollectorState,
    local_http::{HttpRequest, HttpResponse},
};
use promptbox_core::{PromptEvent, PromptStore, HOOK_EVENTS_PATH};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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

        if let Err(error) = self.store.record_prompt_event(&event) {
            let message = format!("写入正式历史失败：{error}");
            self.state.record_error(message.clone());
            return HttpResponse::new(500, "Internal Server Error", message);
        }

        self.state.record_received();
        HttpResponse::new(204, "No Content", String::new())
    }
}
