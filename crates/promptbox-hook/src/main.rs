use promptbox_core::{
    append_spool_event, current_captured_at, endpoint_host_port, normalize_hook_input,
    parse_local_endpoint, resolve_promptbox_paths, PromptBoxConfig, PromptEvent, Provider,
    HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
use std::{
    env,
    io::{self, Read, Write},
    net::TcpStream,
    time::Duration,
};

fn main() {
    let args = env::args().collect::<Vec<_>>();

    if args.iter().any(|arg| arg == "--version" || arg == "-V") {
        println!("promptbox-hook {}", env!("CARGO_PKG_VERSION"));
        println!("hook_protocol {}", promptbox_core::HOOK_PROTOCOL_VERSION);
        return;
    }

    if args.iter().any(|arg| arg == "--config-path") {
        match resolve_promptbox_paths() {
            Ok(paths) => println!("{}", paths.config_path.display()),
            Err(error) => eprintln!("{error}"),
        }
        return;
    }

    if args.iter().any(|arg| arg == "--home") {
        match resolve_promptbox_paths() {
            Ok(paths) => println!("{}", paths.home.display()),
            Err(error) => eprintln!("{error}"),
        }
        return;
    }

    let provider = match provider_from_args(&args) {
        Ok(provider) => provider,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    let config = match promptbox_core::load_config_for_hook() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if config.recording_paused {
        return;
    }

    let input = match read_hook_stdin() {
        Ok(input) => input,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    let event = match normalize_hook_input(provider, &input, current_captured_at()) {
        Ok(event) => event,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };

    if deliver_event(&config, &event).is_ok() {
        return;
    }

    match resolve_promptbox_paths() {
        Ok(paths) => {
            if let Err(error) = append_spool_event(&paths.spool_path, &event) {
                eprintln!("{error}");
            }
        }
        Err(error) => eprintln!("{error}"),
    }
}

fn provider_from_args(args: &[String]) -> Result<Provider, String> {
    for (index, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix("--provider=") {
            return Provider::parse(value);
        }

        if arg == "--provider" {
            let value = args
                .get(index + 1)
                .ok_or_else(|| "缺少 --provider 参数值".to_string())?;
            return Provider::parse(value);
        }
    }

    Err("缺少 --provider 参数，例如 --provider claude 或 --provider codex".to_string())
}

fn read_hook_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .lock()
        .take((MAX_HOOK_BODY_BYTES + 1) as u64)
        .read_to_string(&mut input)
        .map_err(|error| format!("读取 hook stdin 失败：{error}"))?;

    if input.trim().is_empty() {
        return Err("hook stdin 为空".to_string());
    }

    Ok(input)
}

fn deliver_event(config: &PromptBoxConfig, event: &PromptEvent) -> Result<(), String> {
    let addr = parse_local_endpoint(&config.local_endpoint)?;
    let host_port = endpoint_host_port(&config.local_endpoint)?;
    let body =
        serde_json::to_vec(event).map_err(|error| format!("序列化 hook 事件失败：{error}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(1200))
        .map_err(|error| format!("连接本地采集端点失败：{}：{error}", config.local_endpoint))?;

    stream
        .set_read_timeout(Some(Duration::from_millis(1200)))
        .map_err(|error| format!("设置本地采集端点读取超时失败：{error}"))?;
    stream
        .set_write_timeout(Some(Duration::from_millis(1200)))
        .map_err(|error| format!("设置本地采集端点写入超时失败：{error}"))?;

    let headers = format!(
        "POST {HOOK_EVENTS_PATH} HTTP/1.1\r\nHost: {host_port}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        config.token,
        body.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(&body))
        .map_err(|error| format!("投递 hook 事件失败：{error}"))?;

    let mut response = [0_u8; 128];
    let size = stream
        .read(&mut response)
        .map_err(|error| format!("读取本地采集端点响应失败：{error}"))?;
    let response = String::from_utf8_lossy(&response[..size]);
    if response.starts_with("HTTP/1.1 2") || response.starts_with("HTTP/1.0 2") {
        Ok(())
    } else {
        Err("本地采集端点返回非 2xx 状态".to_string())
    }
}
