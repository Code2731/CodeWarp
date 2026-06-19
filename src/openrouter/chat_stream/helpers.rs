use super::super::api_types::fetch_non_stream_fallback;
use super::super::parse::{
    consume_sse_line, extract_plain_stream_token, extract_stream_text, flush_pending_sse_data,
    parse_stream_chunks,
};
use super::super::types::{ChatEvent, ChatMessage};

pub(super) async fn fallback_to_non_stream(
    client: &reqwest::Client,
    endpoint: &str,
    base_url: &str,
    api_key: Option<&str>,
    model: &str,
    messages: &[ChatMessage],
    tools: Option<&serde_json::Value>,
) -> Result<Option<String>, String> {
    match fetch_non_stream_fallback(client, endpoint, base_url, api_key, model, messages, tools)
        .await
    {
        Ok(Some(content)) if !content.is_empty() => Ok(Some(content)),
        Ok(_) => Ok(None),
        Err(e) => Err(e),
    }
}

pub(super) fn process_leftover_buffer(
    buffer: &str,
    pending_sse_data: &mut String,
    generation_id: &mut Option<String>,
    last_finish_reason: &mut Option<String>,
    emitted_any_token: &mut bool,
) -> Vec<ChatEvent> {
    let mut events = Vec::new();
    if !buffer.trim().is_empty() {
        for line in buffer.lines() {
            let Some(payload) = consume_sse_line(line, pending_sse_data) else {
                continue;
            };
            if payload.trim() == "[DONE]" {
                continue;
            }
            let parsed_chunks = parse_stream_chunks(&payload);
            if parsed_chunks.is_empty() {
                if let Some(text) = extract_plain_stream_token(&payload) {
                    *emitted_any_token = true;
                    events.push(ChatEvent::Token(text));
                }
                continue;
            }
            for parsed in parsed_chunks {
                if generation_id.is_none() {
                    if let Some(id) = parsed.id {
                        *generation_id = Some(id);
                    }
                }
                for choice in parsed.choices {
                    if let Some(reason) = choice.finish_reason.as_ref() {
                        *last_finish_reason = Some(reason.clone());
                    }
                    if let Some(text) = extract_stream_text(&choice) {
                        *emitted_any_token = true;
                        events.push(ChatEvent::Token(text));
                    }
                }
            }
        }
    }
    if let Some(payload) = flush_pending_sse_data(pending_sse_data) {
        if payload.trim() != "[DONE]" {
            let parsed_chunks = parse_stream_chunks(&payload);
            if parsed_chunks.is_empty() {
                if let Some(text) = extract_plain_stream_token(&payload) {
                    *emitted_any_token = true;
                    events.push(ChatEvent::Token(text));
                }
            }
            for parsed in parsed_chunks {
                if generation_id.is_none() {
                    if let Some(id) = parsed.id {
                        *generation_id = Some(id);
                    }
                }
                for choice in parsed.choices {
                    if let Some(reason) = choice.finish_reason.as_ref() {
                        *last_finish_reason = Some(reason.clone());
                    }
                    if let Some(text) = extract_stream_text(&choice) {
                        *emitted_any_token = true;
                        events.push(ChatEvent::Token(text));
                    }
                }
            }
        }
    }
    events
}
