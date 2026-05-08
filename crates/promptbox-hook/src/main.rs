use promptbox_core::{
    append_spool_event, current_captured_at, endpoint_host_port, normalize_hook_input,
    parse_local_endpoint, resolve_promptbox_paths, PromptBoxConfig, PromptEvent, Provider,
    HOOK_EVENTS_PATH, MAX_HOOK_BODY_BYTES,
};
use std::{
    env,
    io::{self, Read, Write},
    net::TcpStream,
    panic,
    time::Duration,
};

fn main() {
    panic::set_hook(Box::new(|_| {
        eprintln!("promptbox-hook 内部异常，已按 fail-open 退出");
    }));
    let _ = panic::catch_unwind(run);
}

fn run() {
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
    collect_complete_json_input(io::stdin().lock())
}

fn collect_complete_json_input<R: Read>(mut reader: R) -> Result<String, String> {
    // Codex 在 Windows 下可能会延后关闭 hook stdin；这里只读到首个完整 JSON 对象，
    // 避免 hook 进程为了等待 EOF 一直挂到 Codex 退出时才被强制清理。
    let mut input = Vec::new();
    let mut chunk = [0_u8; 1];
    let mut started = false;
    let mut object_depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    loop {
        let read = reader
            .read(&mut chunk)
            .map_err(|error| format!("读取 hook stdin 失败：{error}"))?;
        if read == 0 {
            break;
        }

        let byte = chunk[0];
        if !started {
            if byte.is_ascii_whitespace() {
                continue;
            }
            if byte != b'{' {
                return Err("hook stdin 根节点必须是 JSON object".to_string());
            }
            started = true;
            object_depth = 1;
            push_hook_byte(&mut input, byte)?;
            continue;
        }

        push_hook_byte(&mut input, byte)?;

        if in_string {
            match byte {
                b'\\' if !escaped => escaped = true,
                b'"' if !escaped => in_string = false,
                _ => escaped = false,
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' => object_depth += 1,
            b'}' => {
                object_depth = object_depth
                    .checked_sub(1)
                    .ok_or_else(|| "hook stdin JSON 结构非法".to_string())?;
                if object_depth == 0 {
                    break;
                }
            }
            _ => {}
        }
    }

    if !started {
        return Err("hook stdin 为空".to_string());
    }
    if object_depth != 0 || in_string {
        return Err("hook stdin JSON 未完整结束".to_string());
    }

    String::from_utf8(input).map_err(|error| format!("hook stdin 不是有效 UTF-8：{error}"))
}

fn push_hook_byte(input: &mut Vec<u8>, byte: u8) -> Result<(), String> {
    input.push(byte);
    if input.len() > MAX_HOOK_BODY_BYTES {
        return Err(format!(
            "hook 输入超过限制：{} bytes，大于 {} bytes",
            input.len(),
            MAX_HOOK_BODY_BYTES
        ));
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::collect_complete_json_input;
    use std::io::Cursor;

    #[test]
    fn collect_complete_json_input_stops_after_first_object() {
        let input = Cursor::new(br#"  {"session_id":"demo","prompt":"hello"} trailing"#.to_vec());
        let parsed = collect_complete_json_input(input).unwrap();

        assert_eq!(parsed, r#"{"session_id":"demo","prompt":"hello"}"#);
    }

    #[test]
    fn collect_complete_json_input_keeps_nested_content() {
        let input = Cursor::new(
            br#"{"session_id":"demo","prompt":"hello \"world\"","meta":{"items":[1,2,3]}}"#
                .to_vec(),
        );
        let parsed = collect_complete_json_input(input).unwrap();

        assert_eq!(
            parsed,
            r#"{"session_id":"demo","prompt":"hello \"world\"","meta":{"items":[1,2,3]}}"#
        );
    }

    #[test]
    fn collect_complete_json_input_rejects_empty_stdin() {
        let error = collect_complete_json_input(Cursor::new(Vec::<u8>::new())).unwrap_err();

        assert_eq!(error, "hook stdin 为空");
    }

    #[test]
    fn collect_complete_json_input_rejects_incomplete_json() {
        let error =
            collect_complete_json_input(Cursor::new(br#"{"session_id":"demo""#.to_vec()))
                .unwrap_err();

        assert_eq!(error, "hook stdin JSON 未完整结束");
    }

    #[test]
    fn collect_complete_json_input_rejects_non_object_root() {
        let error =
            collect_complete_json_input(Cursor::new(br#"["not","object"]"#.to_vec())).unwrap_err();

        assert_eq!(error, "hook stdin 根节点必须是 JSON object");
    }
}
