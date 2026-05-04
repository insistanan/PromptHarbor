#[cfg(target_os = "windows")]
use std::process::Command as ProcessCommand;

#[cfg(target_os = "windows")]
pub(crate) fn apply_autostart_setting(enabled: bool) -> Result<(), String> {
    const RUN_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";
    const VALUE_NAME: &str = "PromptHarbor";

    if enabled {
        let current_exe =
            std::env::current_exe().map_err(|error| format!("读取当前程序路径失败：{error}"))?;
        let command_value = format!("\"{}\"", current_exe.display());
        let output = ProcessCommand::new("reg")
            .args([
                "add",
                RUN_KEY,
                "/v",
                VALUE_NAME,
                "/t",
                "REG_SZ",
                "/d",
                &command_value,
                "/f",
            ])
            .output()
            .map_err(|error| format!("写入 Windows 开机启动项失败：{error}"))?;
        if !output.status.success() {
            return Err(format!(
                "写入 Windows 开机启动项失败：{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        return Ok(());
    }

    let output = ProcessCommand::new("reg")
        .args(["delete", RUN_KEY, "/v", VALUE_NAME, "/f"])
        .output()
        .map_err(|error| format!("删除 Windows 开机启动项失败：{error}"))?;
    if !output.status.success() {
        return Ok(());
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn apply_autostart_setting(_enabled: bool) -> Result<(), String> {
    Ok(())
}
