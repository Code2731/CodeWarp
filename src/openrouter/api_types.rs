// openrouter/api_types.rs — API response types and helpers (openrouter child module)
use serde::Deserialize;

use super::types::ChatMessage;

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
        .user_agent("CodeWarp/0.2.0")
        .build()
        .map_err(|e| format!("HTTP client 생성 실패: {e}"))
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
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter {status}: {text}"));
    }
    let raw = resp.text().await.unwrap_or_default();
    Ok(extract_non_stream_content(raw.trim()))
}
