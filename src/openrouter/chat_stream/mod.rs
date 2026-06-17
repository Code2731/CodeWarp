// openrouter/chat_stream — SSE streaming chat (openrouter child module)
use std::sync::Arc;

use futures_util::{Stream, StreamExt};

use super::api_types::http_client;
use super::parse::{
    consume_sse_line, extract_non_stream_content, extract_plain_stream_token, extract_stream_text,
    parse_stream_chunks,
};
use super::types::{ChatEvent, ChatMessage, ChatRequest};

mod helpers;

pub fn chat_stream(
    base_url: String,
    api_key: Option<String>,
    model: String,
    messages: Arc<Vec<ChatMessage>>,
    tools: Option<serde_json::Value>,
) -> impl Stream<Item = ChatEvent> {
    async_stream::stream! {
        let client = match http_client() {
            Ok(c) => c,
            Err(e) => {
                yield ChatEvent::Error(e);
                return;
            }
        };
        let body = ChatRequest {
            model: &model,
            messages: &*messages,
            stream: true,
            tools: tools.as_ref(),
            tool_choice: tools.as_ref().map(|_| "auto"),
        };

        let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        const MAX_RETRIES: u32 = 3;
        let mut attempt = 0u32;
        let resp = loop {
            let mut req = client.post(&endpoint).json(&body);
            if base_url.contains("openrouter.ai") {
                req = req
                    .header("HTTP-Referer", "https://codewarp.app")
                    .header("X-Title", "CodeWarp");
            }
            req = super::api_types::apply_compat_auth_headers(req, &base_url, api_key.as_deref());
            match req.send().await {
                Ok(r) if r.status().is_success() => break r,
                Ok(r) => {
                    let status = r.status();
                    let text = r.text().await.unwrap_or_default();
                    if status.is_server_error() && attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_secs(1 << attempt);
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    yield ChatEvent::Error(format!("OpenRouter {}: {}", status, text));
                    return;
                }
                Err(e) => {
                    if attempt < MAX_RETRIES {
                        let delay = std::time::Duration::from_secs(1 << attempt);
                        tokio::time::sleep(delay).await;
                        attempt += 1;
                        continue;
                    }
                    yield ChatEvent::Error(e.to_string());
                    return;
                }
            }
        };

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut raw_capture = String::new();
        let mut last_finish_reason: Option<String> = None;
        let mut generation_id: Option<String> = None;
        let mut emitted_any_token = false;
        let mut pending_sse_data = String::new();

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
            raw_capture.push_str(text);

            loop {
                let Some(idx) = buffer.find('\n') else { break };
                let line = buffer[..idx].trim_end_matches('\r').to_string();
                buffer.drain(..=idx);

                let Some(payload) = consume_sse_line(&line, &mut pending_sse_data) else { continue };
                if payload.trim() == "[DONE]" {
                    if !emitted_any_token {
                        if let Some(content) = extract_non_stream_content(raw_capture.trim()) {
                            if !content.is_empty() {
                                emitted_any_token = true;
                                yield ChatEvent::Token(content);
                            }
                        }
                    }
                    if !emitted_any_token {
                        match helpers::fallback_to_non_stream(
                            &client, &endpoint, &base_url, api_key.as_deref(),
                            &model, &messages, tools.as_ref(),
                        ).await {
                            Ok(Some(content)) => yield ChatEvent::Token(content),
                            Ok(None) => {}
                            Err(e) => {
                                yield ChatEvent::Error(e);
                                return;
                            }
                        }
                    }
                    yield ChatEvent::Done {
                        finish_reason: last_finish_reason.clone(),
                        generation_id: generation_id.clone(),
                    };
                    return;
                }
                let parsed_chunks = parse_stream_chunks(&payload);
                if parsed_chunks.is_empty() {
                    if let Some(text) = extract_plain_stream_token(&payload) {
                        emitted_any_token = true;
                        yield ChatEvent::Token(text);
                    }
                    continue;
                }
                for parsed in parsed_chunks {
                    if generation_id.is_none() {
                        if let Some(id) = parsed.id {
                            generation_id = Some(id);
                        }
                    }
                    for choice in parsed.choices {
                        if let Some(reason) = choice.finish_reason.as_ref() {
                            last_finish_reason = Some(reason.clone());
                        }
                        if let Some(text) = extract_stream_text(&choice) {
                            emitted_any_token = true;
                            yield ChatEvent::Token(text);
                        }
                        if let Some(delta) = choice.delta {
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

        for event in helpers::process_leftover_buffer(
            &buffer, &mut pending_sse_data,
            &mut generation_id, &mut last_finish_reason, &mut emitted_any_token,
        ) {
            yield event;
        }

        if !emitted_any_token {
            if let Some(content) = extract_non_stream_content(raw_capture.trim()) {
                if !content.is_empty() {
                    emitted_any_token = true;
                    yield ChatEvent::Token(content);
                }
            }
        }

        if !emitted_any_token {
            match helpers::fallback_to_non_stream(
                &client, &endpoint, &base_url, api_key.as_deref(),
                &model, &messages, tools.as_ref(),
            ).await {
                Ok(Some(content)) => yield ChatEvent::Token(content),
                Ok(None) => {}
                Err(e) => {
                    yield ChatEvent::Error(e);
                    return;
                }
            }
        }

        yield ChatEvent::Done {
            finish_reason: last_finish_reason,
            generation_id,
        };
    }
}
