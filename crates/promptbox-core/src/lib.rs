use serde::Serialize;

pub const APP_NAME: &str = "PromptHarbor";
pub const APP_DISPLAY_NAME: &str = "提示港 PromptHarbor";
pub const DEFAULT_LOCAL_ENDPOINT: &str = "127.0.0.1:9996";
pub const HOOK_PROTOCOL_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_name: String,
    pub display_name: String,
    pub version: String,
    pub hook_protocol_version: String,
    pub local_endpoint: String,
    pub data_policy: String,
}

pub fn app_status() -> AppStatus {
    AppStatus {
        app_name: APP_NAME.to_string(),
        display_name: APP_DISPLAY_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hook_protocol_version: HOOK_PROTOCOL_VERSION.to_string(),
        local_endpoint: DEFAULT_LOCAL_ENDPOINT.to_string(),
        data_policy: "本地优先；默认不联网；只记录用户 prompt".to_string(),
    }
}
