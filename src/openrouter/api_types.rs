// openrouter/api_types.rs — API response types and helpers (openrouter child module)
use futures_util::StreamExt;
use serde::Deserialize;
use std::time::Duration;

use super::types::ChatMessage;

pub(super) const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const MAX_PROVIDER_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct AuthKeyData {
    pub usage: Option<f64>,
    pub limit: Option<f64>,
}

#[derive(Deserialize)]
pub(super) struct AuthKeyResponse {
    pub(crate) data: AuthKeyData,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(crate) struct GenerationData {
    pub model: Option<String>,
    pub total_cost: Option<f64>,
    pub native_tokens_prompt: Option<u64>,
    pub native_tokens_completion: Option<u64>,
}

#[derive(Deserialize)]
pub(super) struct GenerationResponse {
    pub(crate) data: GenerationData,
}

pub(super) fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent(concat!("CodeWarp/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .build()
        .map_err(|e| format!("HTTP client 생성 실패: {e}"))
}

pub(super) fn provider_label(base_url: &str) -> &'static str {
    if base_url.to_ascii_lowercase().contains("openrouter.ai") {
        "OpenRouter"
    } else {
        "OpenAI-compatible provider"
    }
}

pub(super) fn apply_compat_auth_headers(
    mut req: reqwest::RequestBuilder,
    base_url: &str,
    api_key: Option<&str>,
) -> reqwest::RequestBuilder {
    if let Some(k) = api_key.filter(|s| !s.trim().is_empty()) {
        let token = k.trim();
        req = req.bearer_auth(token);
        if !base_url.contains("openrouter.ai") {
            req = req.header("x-api-key", token);
        }
    }
    req
}

pub(crate) async fn read_response_text_bounded(
    response: reqwest::Response,
) -> Result<String, String> {
    if response
        .content_length()
        .is_some_and(|length| length > MAX_PROVIDER_RESPONSE_BYTES as u64)
    {
        return Err(format!(
            "provider response exceeds {} bytes",
            MAX_PROVIDER_RESPONSE_BYTES
        ));
    }

    let mut body = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("provider response read failed: {error}"))?;
        if body.len().saturating_add(chunk.len()) > MAX_PROVIDER_RESPONSE_BYTES {
            return Err(format!(
                "provider response exceeds {} bytes",
                MAX_PROVIDER_RESPONSE_BYTES
            ));
        }
        body.extend_from_slice(&chunk);
    }

    String::from_utf8(body)
        .map_err(|error| format!("provider response is not valid UTF-8: {error}"))
}

pub(super) async fn fetch_non_stream_fallback(
    client: &reqwest::Client,
    endpoint: &str,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    messages: &[ChatMessage],
    tools: Option<&serde_json::Value>,
) -> Result<Option<String>, String> {
    use super::parse::extract_non_stream_content;
    let mut payload = serde_json::json!({
        "model": model,
        "messages": messages,
        "stream": false
    });
    if let Some(tool_defs) = tools {
        payload["tools"] = tool_defs.clone();
        payload["tool_choice"] = serde_json::json!("auto");
    }
    let mut req = client.post(endpoint).json(&payload);
    if base_url.contains("openrouter.ai") {
        req = req
            .header("HTTP-Referer", "https://codewarp.app")
            .header("X-Title", "CodeWarp");
    }
    req = apply_compat_auth_headers(req, base_url, api_key);
    let resp = req
        .timeout(HTTP_REQUEST_TIMEOUT)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = read_response_text_bounded(resp)
            .await
            .unwrap_or_else(|error| format!("[{error}]"));
        return Err(format!("{} {status}: {text}", provider_label(base_url)));
    }
    let raw = read_response_text_bounded(resp).await?;
    Ok(extract_non_stream_content(raw.trim()))
}
