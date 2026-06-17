use super::*;

#[test]
fn normalize_base_default() {
    assert_eq!(normalize_base(""), "http://localhost:8080");
    assert_eq!(normalize_base("   "), "http://localhost:8080");
}

#[test]
fn normalize_base_strips_trailing_slash() {
    assert_eq!(
        normalize_base("http://localhost:8080/"),
        "http://localhost:8080"
    );
    assert_eq!(
        normalize_base("http://localhost:8080///"),
        "http://localhost:8080"
    );
}

#[test]
fn normalize_base_trims_whitespace() {
    assert_eq!(
        normalize_base("  http://example.com:9000  "),
        "http://example.com:9000"
    );
}

#[test]
fn normalize_base_passthrough() {
    assert_eq!(
        normalize_base("https://tabby.example.com"),
        "https://tabby.example.com"
    );
}

#[test]
fn normalize_base_adds_http_when_scheme_missing() {
    assert_eq!(normalize_base("localhost:8080"), "http://localhost:8080");
    assert_eq!(normalize_base("127.0.0.1:9000"), "http://127.0.0.1:9000");
}

#[test]
fn chat_base_appends_v1() {
    assert_eq!(
        chat_base("http://localhost:8080"),
        "http://localhost:8080/v1"
    );
    assert_eq!(
        chat_base("http://localhost:8080/"),
        "http://localhost:8080/v1"
    );
    assert_eq!(chat_base(""), "http://localhost:8080/v1");
}

#[test]
fn chat_base_no_double_v1() {
    assert_eq!(
        chat_base("http://localhost:9000/v1"),
        "http://localhost:9000/v1"
    );
    assert_eq!(
        chat_base("http://localhost:9000/v1/"),
        "http://localhost:9000/v1"
    );
    assert_eq!(
        chat_base("  http://x.com:8080/v1  "),
        "http://x.com:8080/v1"
    );
}

#[test]
fn humanize_connection_refused() {
    let msg = humanize_error("error sending request: Connection refused");
    assert!(msg.contains("OpenAI 호환 서버"), "got: {}", msg);
    assert!(msg.contains("localhost:5000"), "got: {}", msg);
}

#[test]
fn humanize_os_error_10061() {
    let msg = humanize_error("os error 10061");
    assert!(msg.contains("OpenAI 호환 서버"), "got: {}", msg);
}

#[test]
fn humanize_dns_failure() {
    let msg = humanize_error("dns error: nodename nor servname provided");
    assert!(msg.contains("도메인"));
}

#[test]
fn humanize_invalid_url() {
    let msg = humanize_error("builder error: relative URL without a base");
    assert!(msg.contains("URL 형식 오류"), "got: {}", msg);
    assert!(msg.contains("http://"), "got: {}", msg);
}

#[test]
fn humanize_timeout() {
    let msg = humanize_error("operation timed out");
    assert!(msg.contains("연결 시간 초과"));
    assert!(msg.contains("포트"));
}

#[test]
fn humanize_auth_401() {
    let msg = humanize_error("Tabby 401 Unauthorized: token missing");
    assert!(msg.contains("인증 실패"));
}

#[test]
fn humanize_auth_403() {
    let msg = humanize_error("Tabby 403 Forbidden: ...");
    assert!(msg.contains("인증 실패"));
}

#[test]
fn humanize_404() {
    let msg = humanize_error("Tabby 404 Not Found: page");
    assert!(msg.contains("base URL"));
}

#[test]
fn humanize_unknown_passes_through() {
    let raw = "alien error message that we don't categorize";
    assert_eq!(humanize_error(raw), raw);
}

/// list_models의 format string과 humanize_error 패턴이 동기화되어 있음을 보장.
#[test]
fn humanize_matches_list_models_format() {
    let synthetic_err = format!("Tabby {}: {}", 401, "anything");
    let msg = humanize_error(&synthetic_err);
    assert!(msg.contains("인증 실패"), "got: {}", msg);

    let synthetic_404 = format!("Tabby {}: not found", 404);
    let msg2 = humanize_error(&synthetic_404);
    assert!(msg2.contains("base URL"), "got: {}", msg2);
}

#[test]
fn parse_model_ids_supports_openai_shape() {
    let body = r#"{"object":"list","data":[{"id":"a"},{"id":"b"}]}"#;
    let ids = parse_model_ids(body).unwrap();
    assert_eq!(ids, vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn parse_model_ids_supports_legacy_name_key() {
    let body = r#"{"data":[{"name":"foo"},{"model_name":"bar"}]}"#;
    let ids = parse_model_ids(body).unwrap();
    assert_eq!(ids, vec!["foo".to_string(), "bar".to_string()]);
}

#[test]
fn parse_model_ids_supports_root_array() {
    let body = r#"[{"id":"x"},"y",{"model":"z"}]"#;
    let ids = parse_model_ids(body).unwrap();
    assert_eq!(ids, vec!["x".to_string(), "y".to_string(), "z".to_string()]);
}
