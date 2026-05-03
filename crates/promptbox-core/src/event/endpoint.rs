use std::net::{SocketAddr, ToSocketAddrs};

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
