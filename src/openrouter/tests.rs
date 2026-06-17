use super::parse::*;
use super::types::*;

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
