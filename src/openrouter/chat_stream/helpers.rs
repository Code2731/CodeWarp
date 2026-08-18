use super::super::api_types::fetch_non_stream_fallback;
use super::super::parse::{consume_sse_line, flush_pending_sse_data};
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
            events.extend(super::process_chunk_payload(
                &payload,
                generation_id,
                last_finish_reason,
                emitted_any_token,
            ));
        }
    }
    if let Some(payload) = flush_pending_sse_data(pending_sse_data)
        && payload.trim() != "[DONE]"
    {
        events.extend(super::process_chunk_payload(
            &payload,
            generation_id,
            last_finish_reason,
            emitted_any_token,
        ));
    }
    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leftover_payload_preserves_tool_call_deltas() {
        let payload = r#"{"id":"chatcmpl-leftover","choices":[{"delta":{"tool_calls":[{"index":0,"id":"call-1","function":{"name":"read_file","arguments":"{\"path\":\"README.md\"}"}}]},"finish_reason":"tool_calls"}]}"#;
        let mut pending = String::new();
        let mut generation_id = None;
        let mut finish_reason = None;
        let mut emitted_any_token = false;

        let events = process_leftover_buffer(
            &format!("data: {payload}"),
            &mut pending,
            &mut generation_id,
            &mut finish_reason,
            &mut emitted_any_token,
        );

        assert!(events.iter().any(|event| matches!(
            event,
            ChatEvent::ToolCallDelta {
                index: 0,
                id: Some(id),
                name: Some(name),
                arguments: Some(arguments),
            } if id == "call-1" && name == "read_file" && arguments.contains("README.md")
        )));
        assert_eq!(generation_id.as_deref(), Some("chatcmpl-leftover"));
        assert_eq!(finish_reason.as_deref(), Some("tool_calls"));
        assert!(!emitted_any_token);
    }
}
