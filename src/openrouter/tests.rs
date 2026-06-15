use super::humanize::*;
use super::parse::*;
use super::types::*;

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

#[test]
fn humanize_matches_list_models_format() {
    for status in [401, 402, 403, 404, 429, 500, 502, 503] {
        let synthetic = format!("OpenRouter {}: anything", status);
        let msg = humanize_error(&synthetic);
        assert_ne!(msg, synthetic, "status {} should be humanized", status);
    }
}

#[test]
fn extract_non_stream_content_reads_openai_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","content":"hello"}}]
    }"#;
    assert_eq!(extract_non_stream_content(raw).as_deref(), Some("hello"));
}

#[test]
fn extract_non_stream_content_reads_text_choice_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"text":"hello from text field"}]
    }"#;
    assert_eq!(
        extract_non_stream_content(raw).as_deref(),
        Some("hello from text field")
    );
}

#[test]
fn extract_non_stream_content_reads_array_content_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","content":[{"type":"text","text":"hello "},{"type":"output_text","text":"world"}]}}]
    }"#;
    assert_eq!(
        extract_non_stream_content(raw).as_deref(),
        Some("hello world")
    );
}

#[test]
fn stream_chunk_supports_array_delta_content() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"delta":{"content":[{"type":"text","text":"hello"}]},"finish_reason":null}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("hello"));
}

#[test]
fn stream_chunk_supports_choice_text_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"text":"hello from choice text","finish_reason":null}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("hello from choice text"));
}

#[test]
fn stream_chunk_supports_reasoning_content_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"delta":{"reasoning_content":"reasoning token"},"finish_reason":null}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("reasoning token"));
}

#[test]
fn stream_chunk_supports_reasoning_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"delta":{"reasoning":"reasoning token"},"finish_reason":null}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("reasoning token"));
}

#[test]
fn stream_chunk_supports_xllm_delta_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"delta":{"role":"assistant","content":"안녕하세요","tool_calls":[]}}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("안녕하세요"));
}

#[test]
fn stream_chunk_supports_nested_delta_value_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"delta":{"content":{"value":"hello from delta value"}}}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("hello from delta value"));
}

#[test]
fn stream_chunk_supports_top_level_delta_output_text_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"delta":{"output_text":"hello from delta output_text"}}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("hello from delta output_text"));
}

#[test]
fn stream_chunk_supports_nested_message_value_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","content":{"value":"hello from message value"}}}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("hello from message value"));
}

#[test]
fn stream_chunk_supports_message_reasoning_content_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","reasoning_content":"reasoning from message"}}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("reasoning from message"));
}

#[test]
fn stream_chunk_supports_message_reasoning_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","reasoning":"reasoning plain"}}]
    }"#;
    let parsed: StreamChunk = serde_json::from_str(raw).expect("valid stream chunk");
    let token = parsed.choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("reasoning plain"));
}

#[test]
fn extract_non_stream_content_reads_reasoning_content_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","reasoning_content":"hello from reasoning"}}]
    }"#;
    assert_eq!(
        extract_non_stream_content(raw).as_deref(),
        Some("hello from reasoning")
    );
}

#[test]
fn extract_non_stream_content_reads_nested_message_value_shape() {
    let raw = r#"{
        "id":"chatcmpl-x",
        "choices":[{"index":0,"message":{"role":"assistant","content":{"value":"hello from nested value"}}}]
    }"#;
    assert_eq!(
        extract_non_stream_content(raw).as_deref(),
        Some("hello from nested value")
    );
}

#[test]
fn extract_non_stream_content_reads_top_level_response_shape() {
    let raw = r#"{"response":"hello from top-level response"}"#;
    assert_eq!(
        extract_non_stream_content(raw).as_deref(),
        Some("hello from top-level response")
    );
}

#[test]
fn extract_non_stream_content_reads_top_level_message_shape() {
    let raw = r#"{"message":{"content":"hello from top-level message"}}"#;
    assert_eq!(
        extract_non_stream_content(raw).as_deref(),
        Some("hello from top-level message")
    );
}

#[test]
fn extract_non_stream_content_returns_none_for_invalid_shape() {
    let raw = r#"{"object":"list","data":[]}"#;
    assert!(extract_non_stream_content(raw).is_none());
}

#[test]
fn normalize_stream_payload_line_accepts_sse_data_prefix() {
    assert_eq!(
        normalize_stream_payload_line("data: {\"choices\":[]}"),
        Some("{\"choices\":[]}")
    );
}

#[test]
fn normalize_stream_payload_line_accepts_jsonl_without_data_prefix() {
    assert_eq!(
        normalize_stream_payload_line("{\"choices\":[]}"),
        Some("{\"choices\":[]}")
    );
}

#[test]
fn normalize_stream_payload_line_ignores_blank_lines() {
    assert_eq!(normalize_stream_payload_line("   "), None);
}

#[test]
fn consume_sse_line_joins_multiline_data_until_blank_line() {
    let mut pending = String::new();
    assert_eq!(consume_sse_line("data: {\"a\":1}", &mut pending), None);
    assert_eq!(consume_sse_line("data: {\"b\":2}", &mut pending), None);
    assert_eq!(
        consume_sse_line("", &mut pending).as_deref(),
        Some("{\"a\":1}\n{\"b\":2}")
    );
    assert!(pending.is_empty());
}

#[test]
fn consume_sse_line_ignores_non_data_fields_inside_event() {
    let mut pending = String::new();
    assert_eq!(consume_sse_line("data: hello", &mut pending), None);
    assert_eq!(consume_sse_line("event: message", &mut pending), None);
    assert_eq!(consume_sse_line(":keepalive", &mut pending), None);
    assert_eq!(consume_sse_line("", &mut pending).as_deref(), Some("hello"));
}

#[test]
fn consume_sse_line_passes_jsonl_when_not_in_event() {
    let mut pending = String::new();
    assert_eq!(
        consume_sse_line("{\"choices\":[]}", &mut pending).as_deref(),
        Some("{\"choices\":[]}")
    );
    assert!(pending.is_empty());
}

#[test]
fn consume_sse_line_keeps_raw_json_lines_inside_event() {
    let mut pending = String::new();
    assert_eq!(consume_sse_line("data: {\"a\":1}", &mut pending), None);
    assert_eq!(consume_sse_line("{\"b\":2}", &mut pending), None);
    assert_eq!(
        consume_sse_line("", &mut pending).as_deref(),
        Some("{\"a\":1}\n{\"b\":2}")
    );
    assert!(pending.is_empty());
}

#[test]
fn parse_stream_chunks_accepts_single_json_payload() {
    let payload = r#"{"id":"x","choices":[{"index":0,"delta":{"content":"hi"}}]}"#;
    let chunks = parse_stream_chunks(payload);
    assert_eq!(chunks.len(), 1);
    let token = chunks[0].choices.iter().find_map(extract_stream_text);
    assert_eq!(token.as_deref(), Some("hi"));
}

#[test]
fn parse_stream_chunks_accepts_multiline_json_payload() {
    let payload = concat!(
        r#"{"id":"x","choices":[{"index":0,"delta":{"content":"hello "}}]}"#,
        "\n",
        r#"{"id":"x","choices":[{"index":0,"delta":{"content":"world"}}]}"#
    );
    let chunks = parse_stream_chunks(payload);
    assert_eq!(chunks.len(), 2);
    let tokens: Vec<String> = chunks
        .iter()
        .filter_map(|c| c.choices.iter().find_map(extract_stream_text))
        .collect();
    assert_eq!(tokens, vec!["hello ".to_string(), "world".to_string()]);
}

#[test]
fn parse_stream_chunks_accepts_concatenated_json_payload() {
    let payload = concat!(
        r#"{"id":"x","choices":[{"index":0,"delta":{"content":"one "}}]}"#,
        r#"{"id":"x","choices":[{"index":0,"delta":{"content":"two"}}]}"#
    );
    let chunks = parse_stream_chunks(payload);
    assert_eq!(chunks.len(), 2);
    let tokens: Vec<String> = chunks
        .iter()
        .filter_map(|c| c.choices.iter().find_map(extract_stream_text))
        .collect();
    assert_eq!(tokens, vec!["one ".to_string(), "two".to_string()]);
}

#[test]
fn extract_plain_stream_token_accepts_raw_text() {
    assert_eq!(
        extract_plain_stream_token(" hello from sse ").as_deref(),
        Some("hello from sse")
    );
}

#[test]
fn extract_plain_stream_token_rejects_json_like_payload() {
    assert_eq!(
        extract_plain_stream_token(r#"{"choices":[]}"#).as_deref(),
        None
    );
    assert_eq!(extract_plain_stream_token("[DONE]").as_deref(), None);
}
