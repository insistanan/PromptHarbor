use crate::{
    ingestion::HookEventProcessor,
    local_http::{read_http_request, write_http_response, HttpResponse},
};
use promptbox_core::{parse_local_endpoint, PromptStore};
use std::{
    net::{TcpListener, TcpStream},
    sync::{
        atomic::AtomicBool,
        Arc, Mutex,
    },
    thread,
};

#[derive(Debug, Clone, Default)]
pub struct CollectorState {
    inner: Arc<Mutex<CollectorSnapshot>>,
}

#[derive(Debug, Clone, Default)]
pub struct CollectorSnapshot {
    pub started: bool,
    pub listen_addr: Option<String>,
    pub received_prompt_events: usize,
    pub paused_prompt_events: usize,
    pub recent_error: Option<String>,
    pub startup_error: Option<String>,
}

impl CollectorState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self) -> CollectorSnapshot {
        self.inner
            .lock()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_else(|_| CollectorSnapshot {
                recent_error: Some("采集状态锁已损坏".to_string()),
                ..CollectorSnapshot::default()
            })
    }

    fn mark_started(&self, listen_addr: String) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.started = true;
            snapshot.listen_addr = Some(listen_addr);
            snapshot.startup_error = None;
        }
    }

    pub fn mark_startup_error(&self, error: String) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.started = false;
            snapshot.startup_error = Some(error.clone());
            snapshot.recent_error = Some(error);
        }
    }

    pub(crate) fn record_received(&self) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.received_prompt_events += 1;
            snapshot.recent_error = None;
        }
    }

    pub(crate) fn record_paused(&self) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.paused_prompt_events += 1;
        }
    }

    pub(crate) fn record_error(&self, error: String) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.recent_error = Some(error);
        }
    }
}

pub fn start_local_collector(
    endpoint: &str,
    token: &str,
    store: PromptStore,
    recording_paused: Arc<AtomicBool>,
    state: CollectorState,
) -> Result<String, String> {
    let addr = parse_local_endpoint(endpoint)?;
    let listener = TcpListener::bind(addr)
        .map_err(|error| format!("启动本地采集端点失败：{endpoint}：{error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("读取本地采集端点地址失败：{error}"))?;
    let listen_addr = format!("http://{local_addr}");
    let token = token.to_string();

    state.mark_started(listen_addr.clone());
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let token = token.clone();
            let store = store.clone();
            let recording_paused = Arc::clone(&recording_paused);
            let state = state.clone();

            thread::spawn(move || {
                handle_collector_connection(stream, &token, &store, recording_paused, state);
            });
        }
    });

    Ok(format!("本地采集端点已监听 {listen_addr}"))
}

fn handle_collector_connection(
    mut stream: TcpStream,
    token: &str,
    store: &PromptStore,
    recording_paused: Arc<AtomicBool>,
    state: CollectorState,
) {
    let processor = HookEventProcessor::new(token, store, recording_paused, state.clone());
    let response = match read_http_request(&mut stream) {
        Ok(request) => processor.process(request),
        Err(error) => {
            state.record_error(error.clone());
            HttpResponse::new(400, "Bad Request", error)
        }
    };

    let _ = write_http_response(&mut stream, response);
}
