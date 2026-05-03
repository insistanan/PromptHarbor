use std::env;

fn main() {
    if env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("promptbox-hook {}", env!("CARGO_PKG_VERSION"));
        println!("hook_protocol {}", promptbox_core::HOOK_PROTOCOL_VERSION);
        return;
    }

    // The real capture flow starts in the hook/privacy slice. For the scaffold,
    // keep the executable harmless and fast.
    println!("promptbox-hook scaffold");
}
