use promptbox_core::{HookAdapterStatus, Provider};

#[tauri::command]
pub(crate) fn hook_status(provider: String) -> Result<HookAdapterStatus, String> {
    let provider = Provider::parse(&provider)?;
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::detect_user_hook(provider, &paths.hook_binary_path)
}

#[tauri::command]
pub(crate) fn install_hook(provider: String) -> Result<HookAdapterStatus, String> {
    let provider = Provider::parse(&provider)?;
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::install_user_hook(provider, &paths.hook_binary_path)
}

#[tauri::command]
pub(crate) fn uninstall_hook(provider: String) -> Result<HookAdapterStatus, String> {
    let provider = Provider::parse(&provider)?;
    let paths = promptbox_core::resolve_promptbox_paths()?;
    promptbox_core::uninstall_user_hook(provider, &paths.hook_binary_path)
}
