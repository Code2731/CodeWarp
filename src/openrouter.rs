// OpenRouter HTTP/SSE 클라이언트.
// list_models: 모델 리스트
// chat_stream: SSE 토큰 스트림

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    Token(String),
    Done,
    Error(String),
}

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
struct ChunkChoice {
    delta: Option<DeltaPayload>,
}

#[derive(Deserialize)]
struct DeltaPayload {
    content: Option<String>,
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
) -> impl Stream<Item = ChatEvent> {
    async_stream::stream! {
        let client = http_client();
        let body = ChatRequest {
            model: &model,
            messages: &messages,
            stream: true,
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
                    yield ChatEvent::Done;
                    return;
                }
                if let Ok(parsed) = serde_json::from_str::<StreamChunk>(payload) {
                    for choice in parsed.choices {
                        if let Some(delta) = choice.delta {
                            if let Some(content) = delta.content {
                                if !content.is_empty() {
                                    yield ChatEvent::Token(content);
                                }
                            }
                        }
                    }
                }
            }
        }

        yield ChatEvent::Done;
    }
}
