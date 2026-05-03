use promptbox_core::{
    clear_spool_events, parse_local_endpoint, read_spool_events, AppStatus, ArchiveSessionOutcome,
    ClaudeHookStatus, CodexHookStatus, DeleteSessionOutcome, DraftList, DraftState,
    PromptAttachmentDataUrl, PromptBoxConfig, PromptHistory, PromptSearchResults, PromptStore,
    RuntimeState, SessionList,
};
use std::{
    path::PathBuf,
    process::Command as ProcessCommand,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, PhysicalPosition, Runtime, WebviewWindow, WindowEvent,
};

mod collector;

struct StartupState {
    status: Mutex<AppStatus>,
    collector_state: collector::CollectorState,
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
    let collector_state = collector::CollectorState::new();
    let recording_paused = Arc::new(AtomicBool::new(false));
    let (status, store) = match promptbox_core::initialize_runtime() {
        Ok(runtime) => initialize_runtime_dependent_state(
            &runtime,
            collector_state.clone(),
            Arc::clone(&recording_paused),
        ),
        Err(error) => (promptbox_core::app_status_from_error(error), None),
    };
    recording_paused.store(status.recording_paused, Ordering::SeqCst);

    StartupState {
        status: Mutex::new(status),
        collector_state,
        recording_paused,
        store,
    }
}

fn initialize_runtime_dependent_state(
    runtime: &RuntimeState,
    collector_state: collector::CollectorState,
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
            for event in imported {
                match store.record_prompt_event(&event) {
                    Ok(outcome) => {
                        status.session_count = outcome.session_count;
                        status.prompt_event_count = outcome.prompt_event_count;
                        if outcome.inserted {
                            imported_count += 1;
                        }
                    }
                    Err(error) => {
                        status.startup_errors.push(error);
                        import_failed = true;
                        break;
                    }
                }
            }
            status.imported_spool_events = imported_count;
            status.received_prompt_events = imported_count;
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

    match collector::start_local_collector(
        &runtime.config.local_endpoint,
        &runtime.config.token,
        store.clone(),
        recording_paused,
        collector_state.clone(),
    ) {
        Ok(message) => {
            status.collector_ready = true;
            status.collector_message = message;
        }
        Err(error) => {
            status.collector_ready = false;
            status.collector_message = error.clone();
            status.startup_errors.push(error);
            collector_state.mark_startup_error(status.collector_message.clone());
        }
    }

    (status, Some(store))
}
