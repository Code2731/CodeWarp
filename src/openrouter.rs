// OpenRouter HTTP/SSE 클라이언트.
// list_models: 모델 리스트
// chat_stream: SSE 토큰 + tool_call delta 스트림

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

/// OpenRouter chat 호출 base URL (endpoint 직전).
pub const BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterPricing {
    pub prompt: Option<String>,
    pub completion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: Option<String>,
    pub context_length: Option<u64>,
    pub pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<OpenRouterModel>,
}

/// OpenRouter chat 메시지. role/content 외에 tool 호출 결과 message 형태도 지원.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// `role: "tool"` 메시지에서 응답할 호출 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// `role: "tool"` 메시지에서 함수명 (선택)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `role: "assistant"` 메시지에서 직전에 모델이 호출한 도구 목록
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }

    pub fn assistant_tool_calls(tool_calls: serde_json::Value) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(tool_calls),
            ..Default::default()
        }
    }

    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// 본문 토큰
    Token(String),
    /// 도구 호출 delta (id/name/arguments는 chunk별로 부분 도착)
    ToolCallDelta {
        index: u32,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    /// 정상 종료 (finish_reason + generation_id 포함)
    Done {
        finish_reason: Option<String>,
        generation_id: Option<String>,
    },
    Error(String),
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<&'a str>,
}

#[derive(Deserialize)]
struct StreamChunk {
    #[serde(default)]
    id: Option<String>,
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
struct ChunkChoice {
    delta: Option<DeltaPayload>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct DeltaPayload {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Deserialize)]
struct ToolCallDelta {
    index: u32,
    id: Option<String>,
    function: Option<ToolCallFunctionDelta>,
}

#[derive(Deserialize)]
struct ToolCallFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .build()
        .expect("reqwest client 빌드 실패")
}

/// OpenRouter 키의 사용량/한도 정보 (`/api/v1/auth/key`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AuthKeyData {
    pub usage: Option<f64>,
    pub limit: Option<f64>,
}

#[derive(Deserialize)]
struct AuthKeyResponse {
    data: AuthKeyData,
}

/// /api/v1/generation 응답 — 한 라운드의 실제 비용/토큰.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GenerationData {
    pub model: Option<String>,
    pub total_cost: Option<f64>,
    pub native_tokens_prompt: Option<u64>,
    pub native_tokens_completion: Option<u64>,
}

#[derive(Deserialize)]
struct GenerationResponse {
    data: GenerationData,
}

pub async fn get_generation(api_key: String, id: String) -> Result<GenerationData, String> {
    // OpenRouter는 generation 직후 약간의 지연이 있어 200~500ms 대기 후 조회.
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    let client = http_client();
    let resp = client
        .get(format!("https://openrouter.ai/api/v1/generation?id={}", id))
        .bearer_auth(&api_key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("generation {}: {}", status, body));
    }
    let parsed: GenerationResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}

pub async fn get_account_info(api_key: String) -> Result<AuthKeyData, String> {
    let client = http_client();
    let resp = client
        .get("https://openrouter.ai/api/v1/auth/key")
        .bearer_auth(&api_key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter {}: {}", status, body));
    }
    let parsed: AuthKeyResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}

pub async fn list_models(api_key: String) -> Result<Vec<OpenRouterModel>, String> {
    let client = http_client();
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .bearer_auth(&api_key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        // KEEP IN SYNC: humanize_error가 "OpenRouter {status}" prefix를 매칭함
        return Err(format!("OpenRouter {}: {}", status, body));
    }

    let parsed: ModelsResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}

/// 연결/HTTP 에러 원문을 사용자 친화 actionable 메시지로 변환.
/// `lower.contains`: OS/네트워크 에러 — 대소문자 OS별 차이.
/// `raw.contains`: 이 모듈의 `format!("OpenRouter {}: ...")` 출력 매칭 →
///                 포맷 변경 시 둘이 같이 움직여야 함 (KEEP IN SYNC with list_models /
///                 get_account_info / get_generation / chat_stream).
pub fn humanize_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if raw.contains("OpenRouter 401") || raw.contains("OpenRouter 403") {
        return "키 무효 — Settings에서 재발급/재입력".into();
    }
    if raw.contains("OpenRouter 402") {
        return "잔액 부족 — openrouter.ai/credits 에서 충전".into();
    }
    if raw.contains("OpenRouter 429") {
        return "rate limit — 잠시 후 재시도 (또는 다른 모델)".into();
    }
    if raw.contains("OpenRouter 404") {
        return "모델 ID 무효 — 모델 셀렉터에서 다시 선택".into();
    }
    if raw.contains("OpenRouter 5") {
        // 5xx
        return "OpenRouter 서버 일시 오류 — 잠시 후 재시도".into();
    }
    if lower.contains("dns") || lower.contains("nodename") {
        return "DNS 해석 실패 — 인터넷 연결 확인".into();
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return "응답 지연 — 인터넷 연결 또는 OpenRouter 상태 확인".into();
    }
    raw.to_string()
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;

    #[test]
    fn humanize_401() {
        let msg = humanize_error("OpenRouter 401 Unauthorized: ...");
        assert!(msg.contains("키 무효"));
    }

    #[test]
    fn humanize_402_credits() {
        let msg = humanize_error("OpenRouter 402 Payment Required: ...");
        assert!(msg.contains("잔액 부족"));
        assert!(msg.contains("/credits"));
    }

    #[test]
    fn humanize_429_rate_limit() {
        let msg = humanize_error("OpenRouter 429 Too Many Requests: ...");
        assert!(msg.contains("rate limit"));
    }

    #[test]
    fn humanize_404_model_invalid() {
        let msg = humanize_error("OpenRouter 404 Not Found: model xyz");
        assert!(msg.contains("모델 ID"));
    }

    #[test]
    fn humanize_5xx_server_error() {
        let msg = humanize_error("OpenRouter 503 Service Unavailable");
        assert!(msg.contains("서버 일시 오류"));
        let msg2 = humanize_error("OpenRouter 502 Bad Gateway: ...");
        assert!(msg2.contains("서버 일시 오류"));
    }

    #[test]
    fn humanize_dns() {
        let msg = humanize_error("dns error: failed to resolve host");
        assert!(msg.contains("DNS"));
    }

    #[test]
    fn humanize_timeout() {
        let msg = humanize_error("operation timed out");
        assert!(msg.contains("응답 지연"));
    }

    #[test]
    fn humanize_unknown_passes_through() {
        let raw = "weird thing happened";
        assert_eq!(humanize_error(raw), raw);
    }

    /// KEEP IN SYNC 검증 — list_models의 format!이 humanize_error 패턴과 동기화됨.
    #[test]
    fn humanize_matches_list_models_format() {
        for status in [401, 402, 403, 404, 429, 500, 502, 503] {
            let synthetic = format!("OpenRouter {}: anything", status);
            let msg = humanize_error(&synthetic);
            assert_ne!(msg, synthetic, "status {} should be humanized", status);
        }
    }
}

/// OpenAI 호환 `/chat/completions` 스트림.
/// `base_url`은 endpoint 직전까지 (예: `https://openrouter.ai/api/v1`,
/// `http://localhost:8080/v1`). `api_key`가 None이면 bearer auth 생략 (Tabby 등).
pub fn chat_stream(
    base_url: String,
    api_key: Option<String>,
    model: String,
    messages: Vec<ChatMessage>,
    tools: Option<serde_json::Value>,
) -> impl Stream<Item = ChatEvent> {
    async_stream::stream! {
        let client = http_client();
        let body = ChatRequest {
            model: &model,
            messages: &messages,
            stream: true,
            tools: tools.as_ref(),
            tool_choice: tools.as_ref().map(|_| "auto"),
        };

        let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let mut req = client.post(&endpoint).json(&body);
        // OpenRouter ranking/식별 헤더 — 다른 OpenAI 호환 endpoint(Tabby 등)에는 보내지 않음
        if base_url.contains("openrouter.ai") {
            req = req
                .header("HTTP-Referer", "https://codewarp.app")
                .header("X-Title", "CodeWarp");
        }
        if let Some(k) = api_key.as_ref().filter(|s| !s.trim().is_empty()) {
            req = req.bearer_auth(k);
        }

        let resp = match req.send().await
        {
            Ok(r) => r,
            Err(e) => {
                yield ChatEvent::Error(e.to_string());
                return;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            yield ChatEvent::Error(format!("OpenRouter {}: {}", status, text));
            return;
        }

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut last_finish_reason: Option<String> = None;
        let mut generation_id: Option<String> = None;

        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(b) => b,
                Err(e) => {
                    yield ChatEvent::Error(e.to_string());
                    return;
                }
            };
            let text = match std::str::from_utf8(&chunk) {
                Ok(s) => s,
                Err(_) => continue,
            };
            buffer.push_str(text);

            loop {
                let Some(idx) = buffer.find('\n') else { break };
                let line = buffer[..idx].trim_end_matches('\r').to_string();
                buffer.drain(..=idx);

                let Some(payload) = line.strip_prefix("data:") else { continue };
                let payload = payload.trim();
                if payload.is_empty() { continue; }
                if payload == "[DONE]" {
                    yield ChatEvent::Done {
                        finish_reason: last_finish_reason.clone(),
                        generation_id: generation_id.clone(),
                    };
                    return;
                }
                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(payload) {
                    if generation_id.is_none() {
                        if let Some(id) = parsed.id {
                            generation_id = Some(id);
                        }
                    }
                    for choice in parsed.choices {
                        if let Some(reason) = choice.finish_reason {
                            last_finish_reason = Some(reason);
                        }
                        if let Some(delta) = choice.delta {
                            if let Some(content) = delta.content {
                                if !content.is_empty() {
                                    yield ChatEvent::Token(content);
                                }
                            }
                            if let Some(calls) = delta.tool_calls {
                                for call in calls {
                                    let (name, arguments) = match call.function {
                                        Some(f) => (f.name, f.arguments),
                                        None => (None, None),
                                    };
                                    yield ChatEvent::ToolCallDelta {
                                        index: call.index,
                                        id: call.id,
                                        name,
                                        arguments,
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }

        yield ChatEvent::Done {
            finish_reason: last_finish_reason,
            generation_id,
        };
    }
}
