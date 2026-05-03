use promptbox_core::MAX_HOOK_BODY_BYTES;
use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

pub struct HttpRequest {
    pub method: String,
    pub path: String,
    headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(candidate, _)| candidate == &name)
            .map(|(_, value)| value.as_str())
    }
}

pub struct HttpResponse {
    status: u16,
    reason: &'static str,
    body: String,
}

impl HttpResponse {
    pub fn new(status: u16, reason: &'static str, body: String) -> Self {
        Self {
            status,
            reason,
            body,
        }
    }
}

pub fn read_http_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| format!("设置采集连接读取超时失败：{error}"))?;

    let mut buffer = Vec::new();
    let header_end = loop {
        if let Some(position) = find_header_end(&buffer) {
            break position;
        }

        if buffer.len() > 16 * 1024 {
            return Err("HTTP 请求头超过限制".to_string());
        }

        let mut chunk = [0_u8; 1024];
        let size = stream
            .read(&mut chunk)
            .map_err(|error| format!("读取采集请求失败：{error}"))?;
        if size == 0 {
            return Err("采集请求提前结束".to_string());
        }
        buffer.extend_from_slice(&chunk[..size]);
    };

    let body_start = header_end + 4;
    let header_text = String::from_utf8(buffer[..header_end].to_vec())
        .map_err(|error| format!("HTTP 请求头不是 UTF-8：{error}"))?;
    let mut lines = header_text.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "HTTP 请求缺少请求行".to_string())?;
    let request_parts = request_line.split_whitespace().collect::<Vec<_>>();
    if request_parts.len() < 2 {
        return Err("HTTP 请求行格式不正确".to_string());
    }

    let mut headers = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }

    let content_length = header_value(&headers, "content-length")
        .ok_or_else(|| "采集请求缺少 Content-Length".to_string())?
        .parse::<usize>()
        .map_err(|error| format!("Content-Length 不是有效数字：{error}"))?;
    if content_length > MAX_HOOK_BODY_BYTES {
        return Err(format!(
            "采集请求体超过限制：{content_length} bytes，大于 {MAX_HOOK_BODY_BYTES} bytes"
        ));
    }

    let mut body = buffer[body_start..].to_vec();
    while body.len() < content_length {
        let mut chunk = [0_u8; 4096];
        let size = stream
            .read(&mut chunk)
            .map_err(|error| format!("读取采集请求体失败：{error}"))?;
        if size == 0 {
            return Err("采集请求体提前结束".to_string());
        }
        body.extend_from_slice(&chunk[..size]);
    }
    body.truncate(content_length);

    Ok(HttpRequest {
        method: request_parts[0].to_string(),
        path: request_parts[1].to_string(),
        headers,
        body,
    })
}

pub fn write_http_response(stream: &mut TcpStream, response: HttpResponse) -> Result<(), String> {
    let body = response.body.as_bytes();
    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Length: {}\r\nConnection: close\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n",
        response.status,
        response.reason,
        body.len()
    );

    stream
        .write_all(headers.as_bytes())
        .and_then(|_| stream.write_all(body))
        .map_err(|error| format!("写入采集响应失败：{error}"))
}

fn header_value<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value.as_str())
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    buffer.windows(4).position(|window| window == b"\r\n\r\n")
}
