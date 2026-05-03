use std::env;

fn main() {
    if env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("promptbox-hook {}", env!("CARGO_PKG_VERSION"));
        println!("hook_protocol {}", promptbox_core::HOOK_PROTOCOL_VERSION);
        return;
    }

    if env::args().any(|arg| arg == "--config-path") {
        match promptbox_core::resolve_promptbox_paths() {
            Ok(paths) => println!("{}", paths.config_path.display()),
            Err(error) => eprintln!("{error}"),
        }
        return;
    }

    if env::args().any(|arg| arg == "--home") {
        match promptbox_core::resolve_promptbox_paths() {
            Ok(paths) => println!("{}", paths.home.display()),
            Err(error) => eprintln!("{error}"),
        }
        return;
    }

    let config = match promptbox_core::load_config_for_hook() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if config.recording_paused {
        return;
    }

    // The real capture flow starts in the hook/privacy slice. Until then,
    // keep the executable harmless, silent, and fast.
}
