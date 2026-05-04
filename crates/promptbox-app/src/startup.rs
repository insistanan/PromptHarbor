use crate::{collector, state::StartupState};
use promptbox_core::{clear_spool_events, read_spool_events, AppStatus, PromptStore, RuntimeState};
use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};
use tauri::{path::BaseDirectory, Manager};

#[cfg(windows)]
const BUNDLED_HOOK_RESOURCE: &str = "resources/promptbox-hook.exe";
#[cfg(not(windows))]
const BUNDLED_HOOK_RESOURCE: &str = "resources/promptbox-hook";

pub(crate) fn bundled_hook_source(app: &tauri::App) -> Option<PathBuf> {
    let path = app
        .path()
        .resolve(BUNDLED_HOOK_RESOURCE, BaseDirectory::Resource)
        .ok()?;

    path.is_file().then_some(path)
}

pub(crate) fn initialize_startup_state(hook_source: Option<&Path>) -> StartupState {
    let collector_state = collector::CollectorState::new();
    let recording_paused = Arc::new(AtomicBool::new(false));
    let (status, store) = match promptbox_core::initialize_runtime_with_hook_source(hook_source) {
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

    import_spool_events(runtime, &store, &mut status);
    start_collector(runtime, store.clone(), recording_paused, collector_state, &mut status);

    (status, Some(store))
}

fn import_spool_events(runtime: &RuntimeState, store: &PromptStore, status: &mut AppStatus) {
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
}

fn start_collector(
    runtime: &RuntimeState,
    store: PromptStore,
    recording_paused: Arc<AtomicBool>,
    collector_state: collector::CollectorState,
    status: &mut AppStatus,
) {
    match collector::start_local_collector(
        &runtime.config.local_endpoint,
        &runtime.config.token,
        store,
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
}
