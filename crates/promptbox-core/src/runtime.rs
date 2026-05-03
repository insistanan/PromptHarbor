use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

pub const APP_NAME: &str = "PromptHarbor";
pub const APP_DISPLAY_NAME: &str = "提示港 PromptHarbor";
pub const DEFAULT_LOCAL_ENDPOINT: &str = "127.0.0.1:9996";
pub const HOOK_PROTOCOL_VERSION: &str = "0.1.0";
pub const PROMPTBOX_HOME_ENV: &str = "PROMPTBOX_HOME";
pub const PROMPTBOX_HOOK_SOURCE_ENV: &str = "PROMPTBOX_HOOK_SOURCE";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub app_name: String,
    pub display_name: String,
    pub version: String,
    pub hook_protocol_version: String,
    pub local_endpoint: String,
    pub data_policy: String,
    pub promptbox_home: Option<String>,
    pub config_path: Option<String>,
    pub database_path: Option<String>,
    pub spool_path: Option<String>,
    pub logs_dir: Option<String>,
    pub hook_binary_path: Option<String>,
    pub recording_paused: bool,
    pub maybe_closed_after_hours: u64,
    pub retain_raw_hook_events: bool,
    pub raw_hook_events_retention_days: u64,
    pub autostart: bool,
    pub config_ready: bool,
    pub hook_binary_ready: bool,
    pub hook_binary_message: String,
    pub database_ready: bool,
    pub database_message: String,
    pub session_count: usize,
    pub prompt_event_count: usize,
    pub collector_ready: bool,
    pub collector_message: String,
    pub imported_spool_events: usize,
    pub received_prompt_events: usize,
    pub startup_errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptBoxPaths {
    pub home: PathBuf,
    pub config_path: PathBuf,
    pub database_path: PathBuf,
    pub spool_path: PathBuf,
    pub logs_dir: PathBuf,
    pub hook_binary_path: PathBuf,
}

impl PromptBoxPaths {
    pub fn resolve() -> Result<Self, String> {
        let home = resolve_promptbox_home()?;

        Ok(Self {
            config_path: home.join("config.toml"),
            database_path: home.join("promptbox.sqlite"),
            spool_path: home.join("spool").join("events.jsonl"),
            logs_dir: home.join("logs"),
            hook_binary_path: home.join("bin").join(hook_exe_name()),
            home,
        })
    }

    pub fn ensure_directories(&self) -> Result<(), String> {
        fs::create_dir_all(&self.home)
            .map_err(|error| format!("创建 PromptBox home 失败：{error}"))?;
        fs::create_dir_all(parent_dir(&self.spool_path)?)
            .map_err(|error| format!("创建 spool 目录失败：{error}"))?;
        fs::create_dir_all(&self.logs_dir).map_err(|error| format!("创建日志目录失败：{error}"))?;
        fs::create_dir_all(parent_dir(&self.hook_binary_path)?)
            .map_err(|error| format!("创建 hook bin 目录失败：{error}"))?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptBoxConfig {
    pub local_endpoint: String,
    pub token: String,
    pub recording_paused: bool,
    pub maybe_closed_after_hours: u64,
    pub retain_raw_hook_events: bool,
    pub raw_hook_events_retention_days: u64,
    pub autostart: bool,
}

impl PromptBoxConfig {
    pub fn new() -> Self {
        Self {
            local_endpoint: DEFAULT_LOCAL_ENDPOINT.to_string(),
            token: generate_token(),
            recording_paused: false,
            maybe_closed_after_hours: 12,
            retain_raw_hook_events: true,
            raw_hook_events_retention_days: 7,
            autostart: false,
        }
    }

    pub fn load_or_create(path: &Path) -> Result<(Self, bool), String> {
        if !path.exists() {
            let config = Self::new();
            config.write(path)?;
            return Ok((config, true));
        }

        let raw = fs::read_to_string(path)
            .map_err(|error| format!("读取 PromptBox 用户配置失败：{}：{error}", path.display()))?;
        let partial: PartialPromptBoxConfig = toml::from_str(&raw)
            .map_err(|error| format!("解析 PromptBox 用户配置失败：{}：{error}", path.display()))?;

        let (config, changed) = partial.into_config();
        if changed {
            config.write(path)?;
        }

        Ok((config, changed))
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("创建配置目录失败：{}：{error}", parent.display()))?;
        }

        let serialized = toml::to_string_pretty(self)
            .map_err(|error| format!("序列化 PromptBox 用户配置失败：{error}"))?;
        fs::write(path, serialized)
            .map_err(|error| format!("写入 PromptBox 用户配置失败：{}：{error}", path.display()))
    }
}

#[derive(Debug, Default, Deserialize)]
struct PartialPromptBoxConfig {
    local_endpoint: Option<String>,
    token: Option<String>,
    recording_paused: Option<bool>,
    maybe_closed_after_hours: Option<u64>,
    retain_raw_hook_events: Option<bool>,
    raw_hook_events_retention_days: Option<u64>,
    autostart: Option<bool>,
}

impl PartialPromptBoxConfig {
    fn into_config(self) -> (PromptBoxConfig, bool) {
        let mut changed = false;

        let local_endpoint = self.local_endpoint.unwrap_or_else(|| {
            changed = true;
            DEFAULT_LOCAL_ENDPOINT.to_string()
        });
        let token = self
            .token
            .filter(|token| !token.trim().is_empty())
            .unwrap_or_else(|| {
                changed = true;
                generate_token()
            });
        let recording_paused = self.recording_paused.unwrap_or_else(|| {
            changed = true;
            false
        });
        let maybe_closed_after_hours = self.maybe_closed_after_hours.unwrap_or_else(|| {
            changed = true;
            12
        });
        let retain_raw_hook_events = self.retain_raw_hook_events.unwrap_or_else(|| {
            changed = true;
            true
        });
        let raw_hook_events_retention_days =
            self.raw_hook_events_retention_days.unwrap_or_else(|| {
                changed = true;
                7
            });
        let autostart = self.autostart.unwrap_or_else(|| {
            changed = true;
            false
        });

        (
            PromptBoxConfig {
                local_endpoint,
                token,
                recording_paused,
                maybe_closed_after_hours,
                retain_raw_hook_events,
                raw_hook_events_retention_days,
                autostart,
            },
            changed,
        )
    }
}

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
    fn ready(path: PathBuf, version_output: String) -> Self {
        Self {
            path,
            exists: true,
            ready: true,
            message: "hook 可执行文件可用".to_string(),
            version_output: Some(version_output),
        }
    }

    fn not_ready(
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

#[derive(Debug, Clone)]
pub struct RuntimeState {
    pub paths: PromptBoxPaths,
    pub config: PromptBoxConfig,
    pub hook_binary: HookBinaryStatus,
    pub startup_errors: Vec<String>,
}

impl RuntimeState {
    pub fn initialize() -> Result<Self, String> {
        let mut startup_errors = Vec::new();
        let paths = PromptBoxPaths::resolve()?;
        paths.ensure_directories()?;

        let (config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
        let hook_binary = ensure_hook_binary(&paths).unwrap_or_else(|error| {
            let message = error;
            startup_errors.push(message.clone());
            HookBinaryStatus::not_ready(
                paths.hook_binary_path.clone(),
                paths.hook_binary_path.exists(),
                message,
                None,
            )
        });

        Ok(Self {
            paths,
            config,
            hook_binary,
            startup_errors,
        })
    }

    pub fn app_status(&self) -> AppStatus {
        AppStatus {
            app_name: APP_NAME.to_string(),
            display_name: APP_DISPLAY_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            hook_protocol_version: HOOK_PROTOCOL_VERSION.to_string(),
            local_endpoint: self.config.local_endpoint.clone(),
            data_policy: "本地优先；默认不联网；只记录用户 prompt".to_string(),
            promptbox_home: Some(path_to_string(&self.paths.home)),
            config_path: Some(path_to_string(&self.paths.config_path)),
            database_path: Some(path_to_string(&self.paths.database_path)),
            spool_path: Some(path_to_string(&self.paths.spool_path)),
            logs_dir: Some(path_to_string(&self.paths.logs_dir)),
            hook_binary_path: Some(path_to_string(&self.paths.hook_binary_path)),
            recording_paused: self.config.recording_paused,
            maybe_closed_after_hours: self.config.maybe_closed_after_hours,
            retain_raw_hook_events: self.config.retain_raw_hook_events,
            raw_hook_events_retention_days: self.config.raw_hook_events_retention_days,
            autostart: self.config.autostart,
            config_ready: true,
            hook_binary_ready: self.hook_binary.ready,
            hook_binary_message: self.hook_binary.message.clone(),
            database_ready: false,
            database_message: "数据库尚未初始化".to_string(),
            session_count: 0,
            prompt_event_count: 0,
            collector_ready: false,
            collector_message: "本地采集端点尚未启动".to_string(),
            imported_spool_events: 0,
            received_prompt_events: 0,
            startup_errors: self.startup_errors.clone(),
        }
    }
}

pub fn app_status_from_error(error: String) -> AppStatus {
    AppStatus {
        app_name: APP_NAME.to_string(),
        display_name: APP_DISPLAY_NAME.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hook_protocol_version: HOOK_PROTOCOL_VERSION.to_string(),
        local_endpoint: DEFAULT_LOCAL_ENDPOINT.to_string(),
        data_policy: "本地优先；默认不联网；只记录用户 prompt".to_string(),
        promptbox_home: None,
        config_path: None,
        database_path: None,
        spool_path: None,
        logs_dir: None,
        hook_binary_path: None,
        recording_paused: false,
        maybe_closed_after_hours: 12,
        retain_raw_hook_events: true,
        raw_hook_events_retention_days: 7,
        autostart: false,
        config_ready: false,
        hook_binary_ready: false,
        hook_binary_message: "运行时初始化失败".to_string(),
        database_ready: false,
        database_message: "数据库未初始化".to_string(),
        session_count: 0,
        prompt_event_count: 0,
        collector_ready: false,
        collector_message: "本地采集端点未启动".to_string(),
        imported_spool_events: 0,
        received_prompt_events: 0,
        startup_errors: vec![error],
    }
}

pub fn resolve_promptbox_paths() -> Result<PromptBoxPaths, String> {
    PromptBoxPaths::resolve()
}

pub fn load_config_for_hook() -> Result<PromptBoxConfig, String> {
    let paths = PromptBoxPaths::resolve()?;
    let raw = fs::read_to_string(&paths.config_path).map_err(|error| {
        format!(
            "读取 PromptBox 用户配置失败：{}：{error}",
            paths.config_path.display()
        )
    })?;
    let partial: PartialPromptBoxConfig = toml::from_str(&raw).map_err(|error| {
        format!(
            "解析 PromptBox 用户配置失败：{}：{error}",
            paths.config_path.display()
        )
    })?;

    Ok(partial.into_config().0)
}

fn ensure_hook_binary(paths: &PromptBoxPaths) -> Result<HookBinaryStatus, String> {
    let source = find_hook_source();
    let mut existing_failure = None;
    let mut source_failure = None;

    if let Some(source) = source
        .as_ref()
        .filter(|source| source.as_path() != paths.hook_binary_path.as_path())
    {
        match hook_version_output(source) {
            Ok(version_output) if hook_version_is_compatible(&version_output) => {
                if hook_source_differs(source, &paths.hook_binary_path)? {
                    if let Err(error) = copy_hook_source(source, &paths.hook_binary_path) {
                        if let Ok(target_version_output) =
                            hook_version_output(&paths.hook_binary_path)
                        {
                            if hook_version_is_compatible(&target_version_output) {
                                return Ok(HookBinaryStatus::ready(
                                    paths.hook_binary_path.clone(),
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

    if paths.hook_binary_path.exists() {
        match hook_version_output(&paths.hook_binary_path) {
            Ok(version_output) if hook_version_is_compatible(&version_output) => {
                return Ok(HookBinaryStatus::ready(
                    paths.hook_binary_path.clone(),
                    version_output,
                ));
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

    copy_hook_source(&source, &paths.hook_binary_path)?;

    let version_output = hook_version_output(&paths.hook_binary_path)?;
    if !hook_version_is_compatible(&version_output) {
        return Ok(HookBinaryStatus::not_ready(
            paths.hook_binary_path.clone(),
            true,
            "hook 可执行文件版本或协议版本不匹配".to_string(),
            Some(version_output),
        ));
    }

    Ok(HookBinaryStatus::ready(
        paths.hook_binary_path.clone(),
        version_output,
    ))
}

fn find_hook_source() -> Option<PathBuf> {
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

fn hook_source_differs(source: &Path, target: &Path) -> Result<bool, String> {
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

fn copy_hook_source(source: &Path, target: &Path) -> Result<(), String> {
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

fn hook_version_output(path: &Path) -> Result<String, String> {
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

fn hook_version_is_compatible(output: &str) -> bool {
    let expected_app = format!("promptbox-hook {}", env!("CARGO_PKG_VERSION"));
    let expected_protocol = format!("hook_protocol {HOOK_PROTOCOL_VERSION}");

    output.lines().any(|line| line.trim() == expected_app)
        && output.lines().any(|line| line.trim() == expected_protocol)
}

fn resolve_promptbox_home() -> Result<PathBuf, String> {
    if let Ok(home) = env::var(PROMPTBOX_HOME_ENV) {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    #[cfg(windows)]
    {
        let appdata = env::var("APPDATA").map_err(|error| {
            format!("无法读取 APPDATA，也没有设置 {PROMPTBOX_HOME_ENV}：{error}")
        })?;
        return Ok(PathBuf::from(appdata).join("PromptBox"));
    }

    #[cfg(not(windows))]
    {
        let home = env::var("HOME")
            .map_err(|error| format!("无法读取 HOME，也没有设置 {PROMPTBOX_HOME_ENV}：{error}"))?;
        Ok(PathBuf::from(home).join(".promptbox"))
    }
}

fn generate_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn parent_dir(path: &Path) -> Result<&Path, String> {
    path.parent()
        .ok_or_else(|| format!("路径没有父目录：{}", path.display()))
}

fn hook_exe_name() -> &'static str {
    if cfg!(windows) {
        "promptbox-hook.exe"
    } else {
        "promptbox-hook"
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
