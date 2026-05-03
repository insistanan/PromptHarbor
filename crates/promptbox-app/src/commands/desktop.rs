use crate::desktop;

#[tauri::command]
pub(crate) fn open_project_path(path: String) -> Result<(), String> {
    desktop::open_project_path(path)
}
