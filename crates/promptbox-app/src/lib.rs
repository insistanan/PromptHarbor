use promptbox_core::AppStatus;
use tauri::Manager;
use tauri_plugin_positioner::{Position, WindowExt};

struct StartupState {
    status: AppStatus,
}

#[tauri::command]
fn app_status(state: tauri::State<'_, StartupState>) -> AppStatus {
    state.status.clone()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let status = promptbox_core::initialize_runtime()
                .map(|runtime| runtime.app_status())
                .unwrap_or_else(promptbox_core::app_status_from_error);
            app.manage(StartupState { status });

            #[cfg(desktop)]
            {
                let _ = app.handle().plugin(tauri_plugin_positioner::init());

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.move_window(Position::BottomRight);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_status])
        .run(tauri::generate_context!())
        .expect("error while running PromptHarbor");
}
