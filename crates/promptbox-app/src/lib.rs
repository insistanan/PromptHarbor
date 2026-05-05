use tauri::Manager;

mod autostart;
mod collector;
mod commands;
mod desktop;
mod ingestion;
mod local_http;
mod startup;
mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .on_window_event(|window, event| {
            desktop::prevent_close_and_hide(window, event);
        })
        .setup(|app| {
            let hook_source = startup::bundled_hook_source(app);
            app.manage(startup::initialize_startup_state(hook_source.as_deref()));

            #[cfg(desktop)]
            {
                let _ = app.handle().plugin(tauri_plugin_positioner::init());

                if let Some(window) = app.get_webview_window("main") {
                    let _ = desktop::position_window_at_work_area_bottom_right(&window);
                }
            }

            desktop::setup_tray(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::app_status,
            commands::sessions::list_sessions,
            commands::sessions::archive_session,
            commands::sessions::delete_session,
            commands::drafts::get_draft,
            commands::drafts::list_drafts,
            commands::drafts::get_draft_by_id,
            commands::drafts::create_draft,
            commands::drafts::delete_draft,
            commands::drafts::save_draft,
            commands::drafts::save_draft_by_id,
            commands::drafts::mark_draft_copied,
            commands::drafts::mark_draft_copied_by_id,
            commands::history::list_prompt_history,
            commands::history::read_prompt_attachment_data_url,
            commands::history::search_prompts,
            commands::skills::list_skills,
            commands::skills::read_skill_detail,
            commands::skills::translate_skill,
            commands::skills::list_imported_skill_packages,
            commands::skills::import_skill_zip,
            commands::skills::import_skill_zip_bytes,
            commands::skills::install_imported_skill,
            commands::skills::export_skill_to_library,
            commands::skills::transfer_skill,
            commands::skills::delete_skill,
            commands::skills::delete_imported_skill_package,
            commands::providers::list_custom_providers,
            commands::providers::save_custom_provider,
            commands::providers::delete_custom_provider,
            commands::providers::test_custom_provider,
            commands::providers::optimize_prompt_with_custom_provider,
            commands::runtime::set_recording_paused,
            commands::runtime::update_runtime_config,
            commands::hooks::hook_status,
            commands::hooks::install_hook,
            commands::hooks::uninstall_hook,
            commands::desktop::open_project_path
        ])
        .run(tauri::generate_context!())
        .expect("error while running PromptHarbor");
}
