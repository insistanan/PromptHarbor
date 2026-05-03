use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

mod endpoint;
mod normalize;
mod spool;

pub use endpoint::{endpoint_host_port, parse_local_endpoint};
pub use normalize::normalize_hook_input;
pub use spool::{append_spool_event, clear_spool_events, import_spool_events, read_spool_events};

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
