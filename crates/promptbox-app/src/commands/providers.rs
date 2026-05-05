use promptbox_core::{
    resolve_promptbox_paths, CustomProviderConfig, CustomProviderProtocol, CustomProviderSummary,
    CustomProviderUpsertInput, PromptBoxConfig,
};
use reqwest::{Client, StatusCode, Url};
use serde::Serialize;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomProviderSaveResult {
    pub saved_provider_id: String,
    pub providers: Vec<CustomProviderSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomProviderTestResult {
    pub model: String,
    pub message: String,
    pub assistant_preview: String,
}

#[tauri::command]
pub(crate) fn list_custom_providers() -> Result<Vec<CustomProviderSummary>, String> {
    let (_, config) = load_runtime_config()?;
    Ok(config.custom_provider_summaries())
}

#[tauri::command]
pub(crate) fn save_custom_provider(
    draft: CustomProviderUpsertInput,
) -> Result<CustomProviderSaveResult, String> {
    let (config_path, mut config) = load_runtime_config()?;
    let saved = config.upsert_custom_provider(draft)?;
    config.write(&config_path)?;
    Ok(CustomProviderSaveResult {
        saved_provider_id: saved.id,
        providers: config.custom_provider_summaries(),
    })
}

#[tauri::command]
pub(crate) fn delete_custom_provider(provider_id: String) -> Result<Vec<CustomProviderSummary>, String> {
    let (config_path, mut config) = load_runtime_config()?;
    config.delete_custom_provider(provider_id.trim())?;
    config.write(&config_path)?;
    Ok(config.custom_provider_summaries())
}

#[tauri::command]
pub(crate) async fn test_custom_provider(
    draft: CustomProviderUpsertInput,
) -> Result<CustomProviderTestResult, String> {
    let (_, config) = load_runtime_config()?;
    let provider = config.custom_provider_from_input(draft)?;
    send_provider_test_request(&provider).await
}

fn load_runtime_config() -> Result<(std::path::PathBuf, PromptBoxConfig), String> {
    let paths = resolve_promptbox_paths()?;
    let (config, _) = PromptBoxConfig::load_or_create(&paths.config_path)?;
    Ok((paths.config_path, config))
}

async fn send_provider_test_request(
    provider: &CustomProviderConfig,
) -> Result<CustomProviderTestResult, String> {
    match &provider.protocol {
        CustomProviderProtocol::OpenaiChat => send_openai_chat_test(provider).await,
        _ => Err(format!(
            "{} 协议暂未支持测试连接",
            provider.protocol_label()
        )),
    }
}

async fn send_openai_chat_test(
    provider: &CustomProviderConfig,
) -> Result<CustomProviderTestResult, String> {
    let endpoint = openai_chat_completions_endpoint(&provider.base_url)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("创建供应商请求客户端失败：{error}"))?;

    let response = client
        .post(endpoint.clone())
        .bearer_auth(provider.api_key.trim())
        .json(&json!({
            "model": provider.default_model,
            "messages": [
                {
                    "role": "user",
                    "content": "请只回复 ok"
                }
            ]
        }))
        .send()
        .await
        .map_err(|error| format!("请求供应商失败：{error}"))?;

    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| String::from("无法读取错误响应"));
        return Err(format_http_error(status, &body));
    }

    let payload: Value = response
        .json()
        .await
        .map_err(|error| format!("解析供应商响应失败：{error}"))?;
    let model = payload
        .get("model")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(provider.default_model.as_str())
        .to_string();
    let assistant_preview = extract_openai_output_text(&payload).unwrap_or_default();
    let preview = if assistant_preview.trim().is_empty() {
        "供应商已返回成功响应".to_string()
    } else {
        truncate_text(&assistant_preview, 120)
    };

    Ok(CustomProviderTestResult {
        model: model.clone(),
        message: format!("连接成功，模型 {model} 可用"),
        assistant_preview: preview,
    })
}

fn openai_chat_completions_endpoint(base_url: &str) -> Result<String, String> {
    let trimmed = base_url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return Err("OpenAI Chat 兼容接口地址不能为空".to_string());
    }

    let parsed =
        Url::parse(trimmed).map_err(|error| format!("OpenAI Chat 兼容接口地址格式不正确：{error}"))?;
    let path = parsed.path().trim_end_matches('/');
    if path.ends_with("/chat/completions") {
        return Ok(trimmed.to_string());
    }

    if path.is_empty() || path == "/" {
        return Ok(format!("{trimmed}/v1/chat/completions"));
    }

    Ok(format!("{trimmed}/chat/completions"))
}

fn format_http_error(status: StatusCode, body: &str) -> String {
    let message = parse_openai_error_message(body).unwrap_or_else(|| {
        let compact = body.trim().replace('\n', " ");
        if compact.is_empty() {
            "供应商未返回错误详情".to_string()
        } else {
            truncate_text(&compact, 200)
        }
    });

    match status {
        StatusCode::UNAUTHORIZED => format!("供应商鉴权失败：{message}"),
        StatusCode::FORBIDDEN => format!("供应商拒绝访问：{message}"),
        StatusCode::TOO_MANY_REQUESTS => format!("供应商请求过于频繁：{message}"),
        _ => format!("供应商请求失败（HTTP {}）：{message}", status.as_u16()),
    }
}

fn parse_openai_error_message(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    value
        .get("error")
        .and_then(|error| error.get("message"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
}

fn extract_openai_output_text(payload: &Value) -> Option<String> {
    let choices = payload.get("choices")?.as_array()?;
    let first = choices.first()?;
    let content = first.get("message")?.get("content")?;

    if let Some(text) = content.as_str() {
        let trimmed = text.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }

    let parts = content.as_array()?;
    let mut collected = Vec::new();
    for part in parts {
        if let Some(text) = part.get("text").and_then(Value::as_str) {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                collected.push(trimmed.to_string());
            }
        }
    }

    if collected.is_empty() {
        None
    } else {
        Some(collected.join("\n"))
    }
}

fn truncate_text(text: &str, max_chars: usize) -> String {
    let mut result = String::new();
    for (index, ch) in text.chars().enumerate() {
        if index >= max_chars {
            result.push_str("...");
            break;
        }
        result.push(ch);
    }
    result
}
