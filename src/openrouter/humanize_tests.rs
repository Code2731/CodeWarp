use super::humanize::*;

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
