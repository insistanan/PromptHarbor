use promptbox_core::{
    parse_local_endpoint, PromptEvent, PromptStore, HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
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

    fn record_received(&self) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.received_prompt_events += 1;
            snapshot.recent_error = None;
        }
    }

    fn record_paused(&self) {
        if let Ok(mut snapshot) = self.inner.lock() {
            snapshot.paused_prompt_events += 1;
        }
    }

    fn record_error(&self, error: String) {
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
    let response = match read_http_request(&mut stream) {
        Ok(request) => process_hook_request(request, token, store, recording_paused, state),
        Err(error) => {
            state.record_error(error.clone());
            HttpResponse::new(400, "Bad Request", error)
        }
    };

    let _ = write_http_response(&mut stream, response);
}

fn process_hook_request(
    request: HttpRequest,
    token: &str,
    store: &PromptStore,
    recording_paused: Arc<AtomicBool>,
    state: CollectorState,
) -> HttpResponse {
    if request.method != "POST" || request.path != HOOK_EVENTS_PATH {
        return HttpResponse::new(404, "Not Found", "未知采集路径".to_string());
    }

    let expected = format!("Bearer {token}");
    if header_value(&request.headers, "authorization") != Some(expected.as_str()) {
        state.record_error("token 校验失败".to_string());
        return HttpResponse::new(401, "Unauthorized", "token 校验失败".to_string());
    }

    if recording_paused.load(Ordering::SeqCst) {
        state.record_paused();
        return HttpResponse::new(204, "No Content", String::new());
    }

    let event = match serde_json::from_slice::<PromptEvent>(&request.body) {
        Ok(event) => event,
        Err(error) => {
            let message = format!("解析 hook 事件失败：{error}");
            state.record_error(message.clone());
            return HttpResponse::new(400, "Bad Request", message);
        }
    };

    if let Err(error) = store.record_prompt_event(&event) {
        let message = format!("写入正式历史失败：{error}");
        state.record_error(message.clone());
        return HttpResponse::new(500, "Internal Server Error", message);
    }

    state.record_received();
    HttpResponse::new(204, "No Content", String::new())
}

fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("设置采集连接读取超时失败：{error}"))?;

    let mut buffer = Vec::new();
    let header_end = loop {
        if let Some(position) = find_header_end(&buffer) {
            break position;
        }

        if buffer.len() > 16 * 1024 {
            return Err("HTTP 请求头超过限制".to_string());
        }

        let mut chunk = [0_u8; 1024];
        let size = stream
            .read(&mut chunk)
            .map_err(|error| format!("读取采集请求失败：{error}"))?;
        if size == 0 {
            return Err("采集请求提前结束".to_string());
        }
        buffer.extend_from_slice(&chunk[..size]);
    };

    let body_start = header_end + 4;
    let header_text = String::from_utf8(buffer[..header_end].to_vec())
        .map_err(|error| format!("HTTP 请求头不是 UTF-8：{error}"))?;
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP 请求缺少请求行".to_string())?;
    let request_parts = request_line.split_whitespace().collect::<Vec<_>>();
    if request_parts.len() < 2 {
        return Err("HTTP 请求行格式不正确".to_string());
    }

    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }

    let content_length = header_value(&headers, "content-length")
        .ok_or_else(|| "采集请求缺少 Content-Length".to_string())?
        .parse::<usize>()
        .map_err(|error| format!("Content-Length 不是有效数字：{error}"))?;
    if content_length > MAX_HOOK_BODY_BYTES {
        return Err(format!(
            "采集请求体超过限制：{content_length} bytes，大于 {MAX_HOOK_BODY_BYTES} bytes"
        ));
    }

    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let mut chunk = [0_u8; 4096];
        let size = stream
            .read(&mut chunk)
            .map_err(|error| format!("读取采集请求体失败：{error}"))?;
        if size == 0 {
            return Err("采集请求体提前结束".to_string());
        }
        body.extend_from_slice(&chunk[..size]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method: request_parts[0].to_string(),
        path: request_parts[1].to_string(),
        headers,
        body,
    })
}

fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> Result<(), String> {
    let body = response.body.as_bytes();
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n",
        response.status,
        response.reason,
        body.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("写入采集响应失败：{error}"))
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}

struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    body: String,
}

impl HttpResponse {
    fn new(status: u16, reason: &'static str, body: String) -> Self {
        Self {
            status,
            reason,
            body,
        }
    }
}
