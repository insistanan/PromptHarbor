use std::path::Path;

pub(crate) fn hook_command(hook_path: &Path, provider: &str) -> String {
    if cfg!(windows) && provider != "codex" {
        format!(
            "cmd /d /s /c \"\"{}\" --provider {provider} || exit /b 0\"",
            hook_path.to_string_lossy()
        )
    } else {
        format!("\"{}\" --provider {provider}", hook_path.to_string_lossy())
    }
}
