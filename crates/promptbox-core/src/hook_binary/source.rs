use std::{
    env, fs,
    path::{Path, PathBuf},
};

pub(in crate::hook_binary) const PROMPTBOX_HOOK_SOURCE_ENV: &str = "PROMPTBOX_HOOK_SOURCE";

pub(in crate::hook_binary) fn find_hook_source() -> Option<PathBuf> {
    if let Ok(source) = env::var(PROMPTBOX_HOOK_SOURCE_ENV) {
        let source = PathBuf::from(source);
        if source.is_file() {
            return Some(source);
        }
    }

    let current_exe = env::current_exe().ok()?;
    let sibling = current_exe.with_file_name(hook_exe_name());
    sibling.is_file().then_some(sibling)
}

pub(in crate::hook_binary) fn hook_source_differs(
    source: &Path,
    target: &Path,
) -> Result<bool, String> {
    if !target.exists() {
        return Ok(true);
    }

    let source_meta = fs::metadata(source)
        .map_err(|error| format!("读取 hook 源文件元信息失败：{}：{error}", source.display()))?;
    let target_meta = fs::metadata(target).map_err(|error| {
        format!(
            "读取 hook 目标文件元信息失败：{}：{error}",
            target.display()
        )
    })?;
    if source_meta.len() != target_meta.len() {
        return Ok(true);
    }

    let source_bytes = fs::read(source)
        .map_err(|error| format!("读取 hook 源文件失败：{}：{error}", source.display()))?;
    let target_bytes = fs::read(target)
        .map_err(|error| format!("读取 hook 目标文件失败：{}：{error}", target.display()))?;

    Ok(source_bytes != target_bytes)
}

pub(in crate::hook_binary) fn copy_hook_source(source: &Path, target: &Path) -> Result<(), String> {
    if source == target {
        return Ok(());
    }

    fs::copy(source, target).map_err(|error| {
        format!(
            "更新 hook 可执行文件失败：{} -> {}：{error}",
            source.display(),
            target.display()
        )
    })?;
    Ok(())
}

pub(crate) fn hook_exe_name() -> &'static str {
    if cfg!(windows) {
        "promptbox-hook.exe"
    } else {
        "promptbox-hook"
    }
}
