use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fmt, fs,
    fs::OpenOptions,
    io::Write,
    net::{SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
};

pub const HOOK_EVENTS_PATH: &str = "/api/hook-events";
pub const MAX_HOOK_BODY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Claude,
    Codex,
}

impl Provider {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Ok(Self::Claude),
            "codex" | "codex-cli" | "codex_cli" => Ok(Self::Codex),
            other => Err(format!("不支持的 Agent 客户端：{other}")),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

impl fmt::Display for Provider {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEvent {
    pub provider: Provider,
    pub event_name: String,
    pub session_id: String,
    pub turn_id: Option<String>,
    pub cwd: Option<String>,
    pub transcript_path: Option<String>,
    pub model: Option<String>,
    pub prompt: Option<String>,
    pub captured_at: String,
    pub raw_json: Value,
}

pub fn current_captured_at() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn normalize_hook_input(
    provider: Provider,
    input: &str,
    captured_at: String,
) -> Result<PromptEvent, String> {
    if input.as_bytes().len() > MAX_HOOK_BODY_BYTES {
        return Err(format!(
            "hook 输入超过限制：{} bytes，大于 {} bytes",
            input.as_bytes().len(),
            MAX_HOOK_BODY_BYTES
        ));
    }

    let raw_json: Value =
        serde_json::from_str(input).map_err(|error| format!("解析 hook JSON 失败：{error}"))?;
    let event_name = string_field(&raw_json, "hook_event_name")
        .unwrap_or_else(|| "UserPromptSubmit".to_string());
    let session_id = string_field(&raw_json, "session_id")
        .ok_or_else(|| "hook JSON 缺少 session_id".to_string())?;

    Ok(PromptEvent {
        provider,
        event_name,
        session_id,
        turn_id: string_field(&raw_json, "turn_id"),
        cwd: path_field(&raw_json, "cwd"),
        transcript_path: path_field(&raw_json, "transcript_path"),
        model: string_field(&raw_json, "model"),
        prompt: string_field(&raw_json, "prompt"),
        captured_at,
        raw_json,
    })
}

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

pub fn parse_local_endpoint(endpoint: &str) -> Result<SocketAddr, String> {
    let host_port = endpoint_host_port(endpoint)?;
    let mut addrs = host_port
        .to_socket_addrs()
        .map_err(|error| format!("解析本地采集端点失败：{endpoint}：{error}"))?;

    addrs
        .find(|addr| addr.ip().is_loopback())
        .ok_or_else(|| format!("本地采集端点必须绑定 loopback 地址：{endpoint}"))
}

pub fn endpoint_host_port(endpoint: &str) -> Result<String, String> {
    let trimmed = endpoint.trim();
    if trimmed.is_empty() {
        return Err("本地采集端点不能为空".to_string());
    }

    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("HTTP://"))
        .unwrap_or(trimmed);

    if without_scheme.starts_with("https://") || without_scheme.starts_with("HTTPS://") {
        return Err("本地采集端点只支持 http loopback".to_string());
    }

    let host_port = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .trim();
    if host_port.is_empty() {
        return Err("本地采集端点缺少 host:port".to_string());
    }

    Ok(host_port.to_string())
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn path_field(value: &Value, key: &str) -> Option<String> {
    string_field(value, key).map(|path| normalize_path_string(&path))
}

fn normalize_path_string(value: &str) -> String {
    let path = PathBuf::from(value);
    let normalized = fs::canonicalize(&path).unwrap_or(path);
    trim_trailing_separator(&normalized.to_string_lossy())
}

fn trim_trailing_separator(value: &str) -> String {
    let mut normalized = value.to_string();
    while normalized.len() > 3 && (normalized.ends_with('\\') || normalized.ends_with('/')) {
        normalized.pop();
    }
    normalized
}
