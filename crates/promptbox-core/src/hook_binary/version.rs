use super::HOOK_PROTOCOL_VERSION;
use std::{path::Path, process::Command};

pub(in crate::hook_binary) fn hook_version_output(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("运行 hook 版本检查失败：{}：{error}", path.display()))?;

    if !output.status.success() {
        return Err(format!(
            "hook 版本检查退出失败：{}：{}",
            path.display(),
            output.status
        ));
    }

    String::from_utf8(output.stdout)
        .map(|stdout| stdout.trim().to_string())
        .map_err(|error| format!("hook 版本输出不是 UTF-8：{}：{error}", path.display()))
}

pub(in crate::hook_binary) fn hook_version_is_compatible(output: &str) -> bool {
    let expected_app = format!("promptbox-hook {}", env!("CARGO_PKG_VERSION"));
    let expected_protocol = format!("hook_protocol {HOOK_PROTOCOL_VERSION}");

    output.lines().any(|line| line.trim() == expected_app)
        && output.lines().any(|line| line.trim() == expected_protocol)
}
