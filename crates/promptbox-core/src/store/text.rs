use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn bool_to_i64(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

pub(super) fn session_db_id(
    connection: &Connection,
    provider: &str,
    session_id: &str,
) -> Result<i64, String> {
    connection
        .query_row(
            "select id from sessions where provider = ?1 and session_id = ?2",
            params![provider, session_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("读取 Agent 会话 ID 失败：{error}"))?
        .ok_or_else(|| "写入后没有找到 Agent 会话".to_string())
}

pub(super) fn prompt_hash(prompt: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.trim().as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(super) fn is_low_info_prompt(prompt: &str) -> bool {
    let normalized = prompt.trim().to_ascii_lowercase();
    normalized.chars().count() <= 8
        || matches!(
            normalized.as_str(),
            "同意" | "继续" | "好的" | "收到" | "hi" | "hello" | "你好" | "可以" | "好"
        )
}

pub(super) fn title_from_prompt(prompt: &str, session_id: &str) -> String {
    if is_low_info_prompt(prompt) {
        return short_session_title(session_id);
    }

    let collapsed = prompt.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title = collapsed.chars().take(48).collect::<String>();
    if collapsed.chars().count() > 48 {
        title.push_str("...");
    }

    if title.trim().is_empty() {
        short_session_title(session_id)
    } else {
        title
    }
}

pub(super) fn short_session_title(session_id: &str) -> String {
    let short = session_id.chars().take(8).collect::<String>();
    if short.is_empty() {
        "未命名会话".to_string()
    } else {
        format!("会话 {short}")
    }
}

pub(super) fn short_session_id(session_id: &str) -> String {
    session_id.chars().take(8).collect::<String>()
}

pub(super) fn provider_label(provider: &str) -> &'static str {
    match provider {
        "claude" => "Claude Code",
        "codex" => "Codex CLI",
        _ => "未知 Agent",
    }
}

pub(super) fn contains_query(value: &str, query: &str) -> bool {
    value
        .to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
}

pub(super) fn snippet(value: &str) -> String {
    let collapsed = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut snippet = collapsed.chars().take(160).collect::<String>();
    if collapsed.chars().count() > 160 {
        snippet.push_str("...");
    }
    snippet
}

pub(super) fn project_name_from_cwd(cwd: &str) -> String {
    Path::new(cwd)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(cwd)
        .to_string()
}
