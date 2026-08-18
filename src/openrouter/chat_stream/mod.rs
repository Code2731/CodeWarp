// openrouter/chat_stream — SSE streaming chat (openrouter child module)
use std::sync::Arc;

use futures_util::{Stream, StreamExt};

use super::api_types::{
    HTTP_REQUEST_TIMEOUT, MAX_PROVIDER_RESPONSE_BYTES, STREAM_IDLE_TIMEOUT, http_client,
    provider_label, read_response_text_bounded,
};
use super::parse::{
    consume_sse_line, extract_non_stream_content, extract_plain_stream_token, extract_stream_text,
    parse_stream_chunks,
};
use super::types::{ChatEvent, ChatMessage, ChatRequest};

mod helpers;

pub(super) fn decode_utf8_chunk(pending: &mut Vec<u8>, chunk: &[u8]) -> Result<Option<String>, ()> {
    pending.extend_from_slice(chunk);
    match std::str::from_utf8(pending) {
        Ok(text) => {
            let decoded = text.to_owned();
            pending.clear();
            Ok(Some(decoded))
        }
        Err(error) if error.error_len().is_none() => {
            let valid_up_to = error.valid_up_to();
            if valid_up_to == 0 {
                return Ok(None);
            }
            let bytes = std::mem::take(pending);
            let (valid, incomplete) = bytes.split_at(valid_up_to);
            let decoded = std::str::from_utf8(valid).map_err(|_| ())?.to_owned();
            pending.extend_from_slice(incomplete);
            Ok(Some(decoded))
        }
        Err(_) => Err(()),
    }
}

fn append_bounded_capture(target: &mut String, text: &str, max_bytes: usize) {
    let remaining = max_bytes.saturating_sub(target.len());
    if remaining == 0 {
        return;
    }
    if text.len() <= remaining {
        target.push_str(text);
        return;
    }

    let mut end = remaining;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    target.push_str(&text[..end]);
}

pub(super) async fn next_stream_item<S>(
    stream: &mut S,
    deadline: std::time::Duration,
) -> Result<Option<S::Item>, ()>
where
    S: Stream + Unpin,
{
    tokio::time::timeout(deadline, stream.next())
        .await
        .map_err(|_| ())
}

async fn send_chat_request_with_retry(
    client: &reqwest::Client,
    endpoint: &str,
    model: &str,
    messages: &[ChatMessage],
    tools: Option<&serde_json::Value>,
    api_key: Option<&str>,
    base_url: &str,
) -> Result<reqwest::Response, ChatEvent> {
    const MAX_RETRIES: u32 = 3;
    let body = ChatRequest {
        model,
        messages,
        stream: true,
        tools,
        tool_choice: tools.map(|_| "auto"),
    };
    let mut attempt = 0u32;
    loop {
        let mut req = client.post(endpoint).json(&body);
        if base_url.contains("openrouter.ai") {
            req = req
                .header("HTTP-Referer", "https://codewarp.app")
                .header("X-Title", "CodeWarp");
        }
        req = super::api_types::apply_compat_auth_headers(req, base_url, api_key);
        match tokio::time::timeout(HTTP_REQUEST_TIMEOUT, req.send()).await {
            Err(_) => {
                if attempt < MAX_RETRIES {
                    tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    attempt += 1;
                    continue;
                }
                return Err(ChatEvent::Error(format!(
                    "요청 초기 응답 시간 초과 — {}초 이내에 서버 응답이 없습니다",
                    HTTP_REQUEST_TIMEOUT.as_secs()
                )));
            }
            Ok(Ok(r)) if r.status().is_success() => return Ok(r),
            Ok(Ok(r)) => {
                let status = r.status();
                let text = read_response_text_bounded(r)
                    .await
                    .unwrap_or_else(|error| format!("[{error}]"));
                if status.is_server_error() && attempt < MAX_RETRIES {
                    tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    attempt += 1;
                    continue;
                }
                return Err(ChatEvent::Error(format!(
                    "{} {status}: {text}",
                    provider_label(base_url)
                )));
            }
            Ok(Err(e)) => {
                if attempt < MAX_RETRIES {
                    tokio::time::sleep(std::time::Duration::from_secs(1 << attempt)).await;
                    attempt += 1;
                    continue;
                }
                return Err(ChatEvent::Error(e.to_string()));
            }
        }
    }
}

fn process_chunk_payload(
    payload: &str,
    generation_id: &mut Option<String>,
    last_finish_reason: &mut Option<String>,
    emitted_any_token: &mut bool,
) -> Vec<ChatEvent> {
    let mut events = Vec::new();
    let parsed_chunks = parse_stream_chunks(payload);
    if parsed_chunks.is_empty() {
        if let Some(text) = extract_plain_stream_token(payload) {
            *emitted_any_token = true;
            events.push(ChatEvent::Token(text));
        }
        return events;
    }
    for parsed in parsed_chunks {
        if generation_id.is_none()
            && let Some(id) = parsed.id
        {
            *generation_id = Some(id);
        }
        for choice in parsed.choices {
            if let Some(reason) = choice.finish_reason.as_ref() {
                *last_finish_reason = Some(reason.clone());
            }
            if let Some(text) = extract_stream_text(&choice) {
                *emitted_any_token = true;
                events.push(ChatEvent::Token(text));
            }
            if let Some(delta) = choice.delta
                && let Some(calls) = delta.tool_calls
            {
                for call in calls {
                    let (name, arguments) = match call.function {
                        Some(f) => (f.name, f.arguments),
                        None => (None, None),
                    };
                    events.push(ChatEvent::ToolCallDelta {
                        index: call.index,
                        id: call.id,
                        name,
                        arguments,
                    });
                }
            }
        }
    }
    events
}

#[allow(clippy::too_many_arguments)]
async fn finalize_with_fallback(
    emitted_any_token: &mut bool,
    raw_capture: &str,
    client: &reqwest::Client,
    endpoint: &str,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    messages: &[ChatMessage],
    tools: Option<&serde_json::Value>,
) -> Result<Vec<ChatEvent>, ChatEvent> {
    let mut events = Vec::new();
    if !*emitted_any_token
        && let Some(content) = extract_non_stream_content(raw_capture.trim())
        && !content.is_empty()
    {
        *emitted_any_token = true;
        events.push(ChatEvent::Token(content));
    }
    if !*emitted_any_token {
        match helpers::fallback_to_non_stream(
            client, endpoint, base_url, api_key, model, messages, tools,
        )
        .await
        {
            Ok(Some(content)) => events.push(ChatEvent::Token(content)),
            Ok(None) => {}
            Err(e) => return Err(ChatEvent::Error(e)),
        }
    }
    Ok(events)
}

pub(crate) fn chat_stream(
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
        let endpoint = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let resp = match send_chat_request_with_retry(
            &client, &endpoint, &model, &messages,
            tools.as_ref(), api_key.as_deref(), &base_url,
        ).await {
            Ok(r) => r,
            Err(e) => {
                yield e;
                return;
            }
        };

        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut raw_capture = String::new();
        let mut last_finish_reason: Option<String> = None;
        let mut generation_id: Option<String> = None;
        let mut emitted_any_token = false;
        let mut pending_sse_data = String::new();
        let mut pending_utf8 = Vec::new();

        loop {
            let item = match next_stream_item(&mut stream, STREAM_IDLE_TIMEOUT).await {
                Ok(Some(item)) => item,
                Ok(None) => break,
                Err(_) => {
                    yield ChatEvent::Error(format!(
                        "스트리밍 응답이 {}초 동안 도착하지 않았습니다",
                        STREAM_IDLE_TIMEOUT.as_secs()
                    ));
                    return;
                }
            };
            let chunk = match item {
                Ok(b) => b,
                Err(e) => {
                    yield ChatEvent::Error(e.to_string());
                    return;
                }
            };
            let text = match decode_utf8_chunk(&mut pending_utf8, &chunk) {
                Ok(Some(text)) => text,
                Ok(None) => continue,
                Err(()) => {
                    yield ChatEvent::Error("스트리밍 응답이 올바른 UTF-8이 아닙니다".into());
                    return;
                }
            };
            buffer.push_str(&text);
            append_bounded_capture(&mut raw_capture, &text, MAX_PROVIDER_RESPONSE_BYTES);

            loop {
                let Some(idx) = buffer.find('\n') else { break };
                let line = buffer[..idx].trim_end_matches('\r').to_string();
                buffer.drain(..=idx);

                let Some(payload) = consume_sse_line(&line, &mut pending_sse_data) else { continue };
                if payload.trim() == "[DONE]" {
                    match finalize_with_fallback(
                        &mut emitted_any_token, &raw_capture,
                        &client, &endpoint, &base_url, api_key.as_deref(),
                        &model, &messages, tools.as_ref(),
                    ).await {
                        Ok(events) => {
                            for event in events {
                                yield event;
                            }
                            yield ChatEvent::Done {
                                finish_reason: last_finish_reason,
                                generation_id,
                            };
                        }
                        Err(e) => {
                            yield e;
                            return;
                        }
                    }
                    return;
                }
                for event in process_chunk_payload(
                    &payload,
                    &mut generation_id, &mut last_finish_reason, &mut emitted_any_token,
                ) {
                    yield event;
                }
            }
        }

        if !pending_utf8.is_empty() {
            yield ChatEvent::Error("스트리밍 응답의 UTF-8 데이터가 완전하지 않습니다".into());
            return;
        }

        for event in helpers::process_leftover_buffer(
            &buffer, &mut pending_sse_data,
            &mut generation_id, &mut last_finish_reason, &mut emitted_any_token,
        ) {
            yield event;
        }

        match finalize_with_fallback(
            &mut emitted_any_token, &raw_capture,
            &client, &endpoint, &base_url, api_key.as_deref(),
            &model, &messages, tools.as_ref(),
        ).await {
            Ok(events) => {
                for event in events {
                    yield event;
                }
                yield ChatEvent::Done {
                    finish_reason: last_finish_reason,
                    generation_id,
                };
            }
            Err(e) => {
                yield e;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::append_bounded_capture;

    #[test]
    fn bounded_capture_preserves_utf8_boundaries() {
        let mut capture = String::from("a");

        append_bounded_capture(&mut capture, "한글", 4);

        assert_eq!(capture, "a한");
    }

    #[test]
    fn bounded_capture_does_not_grow_after_reaching_limit() {
        let mut capture = String::from("abcd");

        append_bounded_capture(&mut capture, "more", 4);

        assert_eq!(capture, "abcd");
    }
}
