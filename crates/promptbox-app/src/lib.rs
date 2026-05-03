use promptbox_core::AppStatus;
use tauri::Manager;
use tauri_plugin_positioner::{Position, WindowExt};

#[tauri::command]
fn app_status() -> AppStatus {
    promptbox_core::app_status()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(desktop)]
            {
                app.handle().plugin(tauri_plugin_positioner::init());

                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.as_ref().window().move_window(Position::BottomRight);
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![app_status])
        .run(tauri::generate_context!())
        .expect("error while running PromptHarbor");
}
