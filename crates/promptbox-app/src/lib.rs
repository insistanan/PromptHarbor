use promptbox_core::{
    clear_spool_events, parse_local_endpoint, read_spool_events, AppStatus, ArchiveSessionOutcome,
    ClaudeHookStatus, CodexHookStatus, DeleteSessionOutcome, DraftList, DraftState,
    PromptAttachmentDataUrl, PromptBoxConfig, PromptEvent, PromptHistory, PromptSearchResults,
    PromptStore, RuntimeState, SessionList, HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    process::Command as ProcessCommand,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition, Runtime, WebviewWindow, WindowEvent,
};

struct StartupState {
    status: Mutex<AppStatus>,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
    recording_paused: Arc<AtomicBool>,
    store: Option<PromptStore>,
}

#[tauri::command]
fn app_status(state: tauri::State<'_, StartupState>) -> AppStatus {
    current_app_status(state.inner())
}

fn current_app_status(state: &StartupState) -> AppStatus {
    let mut status = state
        .status
        .lock()
        .map(|status| status.clone())
        .unwrap_or_else(|_| promptbox_core::app_status_from_error("应用状态锁已损坏".to_string()));

    status.recording_paused = state.recording_paused.load(Ordering::SeqCst);
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
fn delete_session(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DeleteSessionOutcome, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.delete_session(&provider, &session_id)
}

#[tauri::command]
fn get_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.get_draft(&provider, &session_id)
}

#[tauri::command]
fn list_drafts(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DraftList, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.list_drafts(&provider, &session_id)
}

#[tauri::command]
fn get_draft_by_id(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.get_draft_by_id(&provider, &session_id, draft_id)
}

#[tauri::command]
fn create_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.create_draft(&provider, &session_id)
}

#[tauri::command]
fn delete_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
) -> Result<DraftList, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.delete_draft(&provider, &session_id, draft_id)
}

#[tauri::command]
fn save_draft(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    content_md: String,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.save_draft(&provider, &session_id, &content_md)
}

#[tauri::command]
fn save_draft_by_id(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
    content_md: String,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.save_draft_by_id(&provider, &session_id, draft_id, &content_md)
}

#[tauri::command]
fn mark_draft_copied(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    content_md: String,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.mark_draft_copied(&provider, &session_id, &content_md)
}

#[tauri::command]
fn mark_draft_copied_by_id(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    draft_id: i64,
    content_md: String,
) -> Result<DraftState, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.mark_draft_copied_by_id(&provider, &session_id, draft_id, &content_md)
}

#[tauri::command]
fn list_prompt_history(
    state: tauri::State<'_, StartupState>,
    provider: String,
    session_id: String,
    include_low_info: bool,
) -> Result<PromptHistory, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.list_prompt_history(&provider, &session_id, include_low_info)
}

#[tauri::command]
fn read_prompt_attachment_data_url(
    state: tauri::State<'_, StartupState>,
    attachment_id: i64,
) -> Result<PromptAttachmentDataUrl, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.read_prompt_attachment_data_url(attachment_id)
}

#[tauri::command]
fn search_prompts(
    state: tauri::State<'_, StartupState>,
    query: String,
    include_low_info: bool,
) -> Result<PromptSearchResults, String> {
    let store = state
        .store
        .as_ref()
        .ok_or_else(|| "数据库尚未初始化".to_string())?;
    store.search_prompts(&query, include_low_info)
}

#[tauri::command]
fn set_recording_paused(
    state: tauri::State<'_, StartupState>,
    paused: bool,
) -> Result<AppStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    let (mut config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
    config.recording_paused = paused;
    config.write(&paths.config_path)?;

    state.recording_paused.store(paused, Ordering::SeqCst);
    if let Ok(mut status) = state.status.lock() {
        status.recording_paused = paused;
    }

    Ok(current_app_status(state.inner()))
}

#[tauri::command]
fn update_runtime_config(
    state: tauri::State<'_, StartupState>,
    local_endpoint: String,
    recording_paused: bool,
    maybe_closed_after_hours: u64,
    retain_raw_hook_events: bool,
    raw_hook_events_retention_days: u64,
    autostart: bool,
) -> Result<AppStatus, String> {
    parse_local_endpoint(&local_endpoint)?;
    if maybe_closed_after_hours == 0 {
        return Err("可能关闭判定时间必须大于 0 小时".to_string());
    }

    let paths = promptbox_core::resolve_promptbox_paths()?;
    let (mut config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
    config.local_endpoint = local_endpoint.trim().to_string();
    config.recording_paused = recording_paused;
    config.maybe_closed_after_hours = maybe_closed_after_hours;
    config.retain_raw_hook_events = retain_raw_hook_events;
    config.raw_hook_events_retention_days = raw_hook_events_retention_days;
    config.autostart = autostart;

    apply_autostart_setting(autostart)?;
    config.write(&paths.config_path)?;

    state
        .recording_paused
        .store(config.recording_paused, Ordering::SeqCst);
    if let Ok(mut status) = state.status.lock() {
        status.local_endpoint = config.local_endpoint;
        status.recording_paused = config.recording_paused;
        status.maybe_closed_after_hours = config.maybe_closed_after_hours;
        status.retain_raw_hook_events = config.retain_raw_hook_events;
        status.raw_hook_events_retention_days = config.raw_hook_events_retention_days;
        status.autostart = config.autostart;
    }

    Ok(current_app_status(state.inner()))
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
fn uninstall_claude_hook() -> Result<ClaudeHookStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::uninstall_claude_user_hook(&paths.hook_binary_path)
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

#[tauri::command]
fn uninstall_codex_hook() -> Result<CodexHookStatus, String> {
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::uninstall_codex_user_hook(&paths.hook_binary_path)
}

#[tauri::command]
fn open_project_path(path: String) -> Result<(), String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err(format!("项目路径不存在：{}", path.display()));
    }

    let target = if path.is_dir() {
        path
    } else {
        path.parent()
            .map(|parent| parent.to_path_buf())
            .ok_or_else(|| "项目路径没有父目录".to_string())?
    };

    #[cfg(target_os = "windows")]
    let mut command = {
        let mut command = ProcessCommand::new("explorer");
        command.arg(&target);
        command
    };

    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = ProcessCommand::new("open");
        command.arg(&target);
        command
    };

    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = ProcessCommand::new("xdg-open");
        command.arg(&target);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("打开项目路径失败：{}：{error}", target.display()))
}

fn apply_autostart_setting(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
        const VALUE_NAME: &str = "PromptHarbor";

        if enabled {
            let current_exe = std::env::current_exe()
                .map_err(|error| format!("读取当前程序路径失败：{error}"))?;
            let command_value = format!("\"{}\"", current_exe.display());
            let output = ProcessCommand::new("reg")
                .args([
                    "add",
                    RUN_KEY,
                    "/v",
                    VALUE_NAME,
                    "/t",
                    "REG_SZ",
                    "/d",
                    &command_value,
                    "/f",
                ])
                .output()
                .map_err(|error| format!("写入 Windows 开机启动项失败：{error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "写入 Windows 开机启动项失败：{}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            return Ok(());
        }

        let output = ProcessCommand::new("reg")
            .args(["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])
            .output()
            .map_err(|error| format!("删除 Windows 开机启动项失败：{error}"))?;
        if !output.status.success() {
            return Ok(());
        }
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let startup_state = initialize_startup_state();
            app.manage(startup_state);

            #[cfg(desktop)]
            {
                let _ = app.handle().plugin(tauri_plugin_positioner::init());

                if let Some(window) = app.get_webview_window("main") {
                    let _ = position_window_at_work_area_bottom_right(&window);
                }
            }

            setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            list_sessions,
            archive_session,
            delete_session,
            get_draft,
            list_drafts,
            get_draft_by_id,
            create_draft,
            delete_draft,
            save_draft,
            save_draft_by_id,
            mark_draft_copied,
            mark_draft_copied_by_id,
            list_prompt_history,
            read_prompt_attachment_data_url,
            search_prompts,
            set_recording_paused,
            update_runtime_config,
            claude_hook_status,
            install_claude_hook,
            uninstall_claude_hook,
            codex_hook_status,
            install_codex_hook,
            uninstall_codex_hook,
            open_project_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running PromptHarbor");
}

fn setup_tray(app: &mut tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "打开主窗口", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "退出 PromptHarbor", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let mut tray = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("提示港 PromptHarbor")
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.build(app)?;
    Ok(())
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = position_window_at_work_area_bottom_right(&window);
        let _ = window.set_focus();
    }
}

fn position_window_at_work_area_bottom_right<R: Runtime>(
    window: &WebviewWindow<R>,
) -> tauri::Result<()> {
    let Some(monitor) = window.current_monitor()?.or_else(|| {
        window
            .available_monitors()
            .ok()
            .and_then(|monitors| monitors.into_iter().next())
    }) else {
        return Ok(());
    };

    let work_area = monitor.work_area();
    let window_size = window.outer_size()?;
    let margin = 16_i32;
    let window_width = i32::try_from(window_size.width).unwrap_or(i32::MAX);
    let window_height = i32::try_from(window_size.height).unwrap_or(i32::MAX);
    let work_width = i32::try_from(work_area.size.width).unwrap_or(i32::MAX);
    let work_height = i32::try_from(work_area.size.height).unwrap_or(i32::MAX);
    let min_x = work_area.position.x + margin;
    let min_y = work_area.position.y + margin;
    let x = (work_area.position.x + work_width - window_width - margin).max(min_x);
    let y = (work_area.position.y + work_height - window_height - margin).max(min_y);

    window.set_position(PhysicalPosition::new(x, y))
}

fn initialize_startup_state() -> StartupState {
    let prompt_events = Arc::new(Mutex::new(Vec::new()));
    let recording_paused = Arc::new(AtomicBool::new(false));
    let (status, store) = match promptbox_core::initialize_runtime() {
        Ok(runtime) => initialize_runtime_dependent_state(
            &runtime,
            Arc::clone(&prompt_events),
            Arc::clone(&recording_paused),
        ),
        Err(error) => (promptbox_core::app_status_from_error(error), None),
    };
    recording_paused.store(status.recording_paused, Ordering::SeqCst);

    StartupState {
        status: Mutex::new(status),
        prompt_events,
        recording_paused,
        store,
    }
}

fn initialize_runtime_dependent_state(
    runtime: &RuntimeState,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
    recording_paused: Arc<AtomicBool>,
) -> (AppStatus, Option<PromptStore>) {
    let mut status = runtime.app_status();
    recording_paused.store(runtime.config.recording_paused, Ordering::SeqCst);
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
        recording_paused,
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
    recording_paused: Arc<AtomicBool>,
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
            let recording_paused = Arc::clone(&recording_paused);

            thread::spawn(move || {
                handle_collector_connection(
                    stream,
                    &token,
                    &store,
                    prompt_events,
                    recording_paused,
                );
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
    recording_paused: Arc<AtomicBool>,
) {
    let response = match read_http_request(&mut stream) {
        Ok(request) => process_hook_request(request, token, store, prompt_events, recording_paused),
        Err(error) => HttpResponse::new(400, "Bad Request", error),
    };

    let _ = write_http_response(&mut stream, response);
}

fn process_hook_request(
    request: HttpRequest,
    token: &str,
    store: &PromptStore,
    prompt_events: Arc<Mutex<Vec<PromptEvent>>>,
    recording_paused: Arc<AtomicBool>,
) -> HttpResponse {
    if request.method != "POST" || request.path != HOOK_EVENTS_PATH {
        return HttpResponse::new(404, "Not Found", "未知采集路径".to_string());
    }

    let expected = format!("Bearer {token}");
    if header_value(&request.headers, "authorization") != Some(expected.as_str()) {
        return HttpResponse::new(401, "Unauthorized", "token 校验失败".to_string());
    }

    if recording_paused.load(Ordering::SeqCst) {
        return HttpResponse::new(204, "No Content", String::new());
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
