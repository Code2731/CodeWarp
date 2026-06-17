use super::parse::*;

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
