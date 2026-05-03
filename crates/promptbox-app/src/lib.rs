use promptbox_core::{
    clear_spool_events, parse_local_endpoint, read_spool_events, AppStatus, ArchiveSessionOutcome,
    ClaudeHookStatus, CodexHookStatus, PromptEvent, PromptStore, RuntimeState, SessionList,
    HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use tauri::Manager;
use tauri_plugin_positioner::{Position, WindowExt};

struct StartupState {
    status: Mutex<AppStatus>,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
    store: Option<PromptStore>,
}

#[tauri::command]
fn app_status(state: tauri::State<'_, StartupState>) -> AppStatus {
    let mut status = state
        .status
        .lock()
        .map(|status| status.clone())
        .unwrap_or_else(|_| promptbox_core::app_status_from_error("应用状态锁已损坏".to_string()));

    if let Ok(events) = state.prompt_events.lock() {
        status.received_prompt_events = events.len();
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

#[tauri::command]
fn list_sessions(state: tauri::State<'_, StartupState>) -> Result<SessionList, String> {
    let maybe_closed_after_hours = state
        .status
        .lock()
        .map(|status| status.maybe_closed_after_hours)
        .unwrap_or(12);
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.list_sessions(maybe_closed_after_hours)
}

#[tauri::command]
fn archive_session(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    force: bool,
) -> Result<ArchiveSessionOutcome, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.archive_session(&provider, &session_id, force)
}

#[tauri::command]
fn claude_hook_status() -> Result<ClaudeHookStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::detect_claude_user_hook(&paths.hook_binary_path)
}

#[tauri::command]
fn install_claude_hook() -> Result<ClaudeHookStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::install_claude_user_hook(&paths.hook_binary_path)
}

#[tauri::command]
fn codex_hook_status() -> Result<CodexHookStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::detect_codex_user_hook(&paths.hook_binary_path)
}

#[tauri::command]
fn install_codex_hook() -> Result<CodexHookStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::install_codex_user_hook(&paths.hook_binary_path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let startup_state = initialize_startup_state();
            app.manage(startup_state);

            #[cfg(desktop)]
            {
                let _ = app.handle().plugin(tauri_plugin_positioner::init());

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.move_window(Position::BottomRight);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            list_sessions,
            archive_session,
            claude_hook_status,
            install_claude_hook,
            codex_hook_status,
            install_codex_hook
        ])
        .run(tauri::generate_context!())
        .expect("error while running PromptHarbor");
}

fn initialize_startup_state() -> StartupState {
    let prompt_events = Arc::new(Mutex::new(Vec::new()));
    let (status, store) = match promptbox_core::initialize_runtime() {
        Ok(runtime) => initialize_runtime_dependent_state(&runtime, Arc::clone(&prompt_events)),
        Err(error) => (promptbox_core::app_status_from_error(error), None),
    };

    StartupState {
        status: Mutex::new(status),
        prompt_events,
        store,
    }
}

fn initialize_runtime_dependent_state(
    runtime: &RuntimeState,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
) -> (AppStatus, Option<PromptStore>) {
    let mut status = runtime.app_status();
    let store = PromptStore::new(runtime.paths.database_path.clone());

    match store.initialize() {
        Ok(summary) => {
            status.database_ready = true;
            status.database_message = "数据库就绪".to_string();
            status.session_count = summary.session_count;
            status.prompt_event_count = summary.prompt_event_count;
        }
        Err(error) => {
            status.database_ready = false;
            status.database_message = error.clone();
            status.startup_errors.push(error);
        }
    }

    match read_spool_events(&runtime.paths.spool_path) {
        Ok(imported) => {
            let mut imported_count = 0;
            let mut import_failed = false;
            match prompt_events.lock() {
                Ok(mut events) => {
                    for event in imported {
                        match store.record_prompt_event(&event) {
                            Ok(outcome) => {
                                status.session_count = outcome.session_count;
                                status.prompt_event_count = outcome.prompt_event_count;
                                if outcome.inserted {
                                    imported_count += 1;
                                }
                                events.push(event);
                            }
                            Err(error) => {
                                status.startup_errors.push(error);
                                import_failed = true;
                                break;
                            }
                        }
                    }
                    status.received_prompt_events = events.len();
                }
                Err(_) => {
                    status
                        .startup_errors
                        .push("采集缓冲区不可用，spool 暂未清理".to_string());
                    import_failed = true;
                }
            }
            status.imported_spool_events = imported_count;
            if !import_failed {
                if let Err(error) = clear_spool_events(&runtime.paths.spool_path) {
                    status.startup_errors.push(error);
                }
            }
        }
        Err(error) => {
            status.startup_errors.push(error);
        }
    }

    match start_local_collector(
        &runtime.config.local_endpoint,
        &runtime.config.token,
        store.clone(),
        Arc::clone(&prompt_events),
    ) {
        Ok(message) => {
            status.collector_ready = true;
            status.collector_message = message;
        }
        Err(error) => {
            status.collector_ready = false;
            status.collector_message = error.clone();
            status.startup_errors.push(error);
        }
    }

    (status, Some(store))
}

fn start_local_collector(
    endpoint: &str,
    token: &str,
    store: PromptStore,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
) -> Result<String, String> {
    let addr = parse_local_endpoint(endpoint)?;
    let listener = TcpListener::bind(addr)
        .map_err(|error| format!("启动本地采集端点失败：{endpoint}：{error}"))?;
    let local_addr = listener
        .local_addr()
        .map_err(|error| format!("读取本地采集端点地址失败：{error}"))?;
    let token = token.to_string();

    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                continue;
            };
            let token = token.clone();
            let store = store.clone();
            let prompt_events = Arc::clone(&prompt_events);

            thread::spawn(move || {
                handle_collector_connection(stream, &token, &store, prompt_events);
            });
        }
    });

    Ok(format!("本地采集端点已监听 http://{local_addr}"))
}

fn handle_collector_connection(
    mut stream: TcpStream,
    token: &str,
    store: &PromptStore,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
) {
    let response = match read_http_request(&mut stream) {
        Ok(request) => process_hook_request(request, token, store, prompt_events),
        Err(error) => HttpResponse::new(400, "Bad Request", error),
    };

    let _ = write_http_response(&mut stream, response);
}

fn process_hook_request(
    request: HttpRequest,
    token: &str,
    store: &PromptStore,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
) -> HttpResponse {
    if request.method != "POST" || request.path != HOOK_EVENTS_PATH {
        return HttpResponse::new(404, "Not Found", "未知采集路径".to_string());
    }

    let expected = format!("Bearer {token}");
    if header_value(&request.headers, "authorization") != Some(expected.as_str()) {
        return HttpResponse::new(401, "Unauthorized", "token 校验失败".to_string());
    }

    let event = match serde_json::from_slice::<PromptEvent>(&request.body) {
        Ok(event) => event,
        Err(error) => {
            return HttpResponse::new(400, "Bad Request", format!("解析 hook 事件失败：{error}"));
        }
    };

    if let Err(error) = store.record_prompt_event(&event) {
        return HttpResponse::new(
            500,
            "Internal Server Error",
            format!("写入正式历史失败：{error}"),
        );
    }

    match prompt_events.lock() {
        Ok(mut events) => events.push(event),
        Err(_) => {
            return HttpResponse::new(500, "Internal Server Error", "采集缓冲区不可用".to_string());
        }
    }

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
