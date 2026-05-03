use serde::Serialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookBinaryStatus {
    pub path: PathBuf,
    pub exists: bool,
    pub ready: bool,
    pub message: String,
    pub version_output: Option<String>,
}

impl HookBinaryStatus {
    pub(crate) fn ready(path: PathBuf, version_output: String) -> Self {
        Self {
            path,
            exists: true,
            ready: true,
            message: "hook 可执行文件可用".to_string(),
            version_output: Some(version_output),
        }
    }

    pub(crate) fn not_ready(
        path: PathBuf,
        exists: bool,
        message: String,
        version_output: Option<String>,
    ) -> Self {
        Self {
            path,
            exists,
            ready: false,
            message,
            version_output,
        }
    }
}
