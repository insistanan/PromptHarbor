use super::PromptEvent;
use std::{
    fs,
    fs::OpenOptions,
    io::Write,
    path::Path,
};

pub fn append_spool_event(path: &Path, event: &PromptEvent) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("创建 spool 目录失败：{}：{error}", parent.display()))?;
    }

    let serialized =
        serde_json::to_string(event).map_err(|error| format!("序列化 spool 事件失败：{error}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("打开 spool 文件失败：{}：{error}", path.display()))?;

    file.write_all(serialized.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|error| format!("写入 spool 文件失败：{}：{error}", path.display()))
}

pub fn import_spool_events(path: &Path) -> Result<Vec<PromptEvent>, String> {
    let events = read_spool_events(path)?;
    clear_spool_events(path)?;
    Ok(events)
}

pub fn read_spool_events(path: &Path) -> Result<Vec<PromptEvent>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|error| format!("读取 spool 文件失败：{}：{error}", path.display()))?;
    let mut events = Vec::new();

    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let event = serde_json::from_str::<PromptEvent>(line).map_err(|error| {
            format!(
                "解析 spool 文件第 {} 行失败：{}：{error}",
                index + 1,
                path.display()
            )
        })?;
        events.push(event);
    }

    Ok(events)
}

pub fn clear_spool_events(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    fs::write(path, "").map_err(|error| format!("清理 spool 文件失败：{}：{error}", path.display()))
}
