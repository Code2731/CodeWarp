// OpenRouter HTTP/SSE 클라이언트.
// list_models: 모델 리스트
// chat_stream: SSE 토큰 + tool_call delta 스트림

use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

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
    /// 정상 종료 (finish_reason 포함)
    Done {
        finish_reason: Option<String>,
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
        return Err(format!("OpenRouter {}: {}", status, body));
    }

    let parsed: ModelsResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}

pub fn chat_stream(
    api_key: String,
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

        let resp = match client
            .post("https://openrouter.ai/api/v1/chat/completions")
            .bearer_auth(&api_key)
            .header("HTTP-Referer", "https://codewarp.app")
            .header("X-Title", "CodeWarp")
            .json(&body)
            .send()
            .await
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
                    yield ChatEvent::Done { finish_reason: last_finish_reason.clone() };
                    return;
                }
                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(payload) {
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

        yield ChatEvent::Done { finish_reason: last_finish_reason };
    }
}
