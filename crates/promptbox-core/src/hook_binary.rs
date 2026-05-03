use std::path::Path;

mod source;
mod status;
mod version;

pub use status::HookBinaryStatus;
pub(crate) use source::hook_exe_name;

use source::{
    copy_hook_source, find_hook_source, hook_source_differs, PROMPTBOX_HOOK_SOURCE_ENV,
};
use version::{hook_version_is_compatible, hook_version_output};

pub const HOOK_PROTOCOL_VERSION: &str = "0.1.0";

pub(crate) struct HookBinaryManager;

impl HookBinaryManager {
    pub(crate) fn ensure(target: &Path) -> Result<HookBinaryStatus, String> {
        let source = find_hook_source();
        let mut existing_failure = None;
        let mut source_failure = None;

        if let Some(source) = source
            .as_ref()
            .filter(|source| source.as_path() != target)
        {
            match hook_version_output(source) {
                Ok(version_output) if hook_version_is_compatible(&version_output) => {
                    if hook_source_differs(source, target)? {
                        if let Err(error) = copy_hook_source(source, target) {
                            if let Ok(target_version_output) = hook_version_output(target) {
                                if hook_version_is_compatible(&target_version_output) {
                                    return Ok(HookBinaryStatus::ready(
                                        target.to_path_buf(),
                                        target_version_output,
                                    ));
                                }
                            }
                            return Err(error);
                        }
                    }
                }
                Ok(version_output) => {
                    source_failure = Some(format!(
                        "当前运行目录中的 hook 可执行文件版本或协议版本不匹配：{version_output}"
                    ));
                }
                Err(error) => {
                    source_failure = Some(error);
                }
            }
        }

        if target.exists() {
            match hook_version_output(target) {
                Ok(version_output) if hook_version_is_compatible(&version_output) => {
                    return Ok(HookBinaryStatus::ready(target.to_path_buf(), version_output));
                }
                Ok(version_output) => {
                    existing_failure = Some(format!(
                        "现有 hook 可执行文件版本或协议版本不匹配：{version_output}"
                    ));
                }
                Err(error) => {
                    existing_failure = Some(error);
                }
            }
        }

        let source = source.ok_or_else(|| {
            let prefix = existing_failure
                .as_ref()
                .map(|failure| format!("{failure}；"))
                .unwrap_or_default();
            format!(
                "{prefix}未找到可用于更新的 hook 可执行文件，请先构建 {} 或设置 {}",
                hook_exe_name(),
                PROMPTBOX_HOOK_SOURCE_ENV
            )
        })?;

        if let Some(source_failure) = source_failure {
            return Err(format!(
                "{}；{}",
                existing_failure.unwrap_or_else(|| "稳定位置 hook 可执行文件不可用".to_string()),
                source_failure
            ));
        }

        copy_hook_source(&source, target)?;

        let version_output = hook_version_output(target)?;
        if !hook_version_is_compatible(&version_output) {
            return Ok(HookBinaryStatus::not_ready(
                target.to_path_buf(),
                true,
                "hook 可执行文件版本或协议版本不匹配".to_string(),
                Some(version_output),
            ));
        }

        Ok(HookBinaryStatus::ready(target.to_path_buf(), version_output))
    }
}
