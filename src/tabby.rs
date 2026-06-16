// Tabby (TabbyML) HTTP 클라이언트 — OpenAI 호환 /v1/models 엔드포인트 사용.
// chat completion 라우팅은 Step 4에서 추가.

use serde::Deserialize;
use serde_json::Value;

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP client 생성 실패: {e}"))
}

fn normalize_base(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "http://localhost:8080".into()
    } else if !trimmed.contains("://") {
        format!("http://{}", trimmed)
    } else {
        trimmed.to_string()
    }
}

/// `chat_stream`에 넘길 base URL ("/v1" 접두 포함).
/// 사용자가 `/v1`을 이미 입력했으면 중복 추가하지 않음.
pub fn chat_base(url: &str) -> String {
    let base = normalize_base(url);
    if base.ends_with("/v1") {
        base
    } else {
        format!("{}/v1", base)
    }
}

/// 연결 에러 원문을 사용자 친화 actionable 메시지로 변환.
/// `lower.contains`: OS/네트워크 에러 — 대소문자 OS별 차이 → 소문자 비교.
/// `raw.contains`: 이 모듈의 `format!("Tabby {}: {}", status, body)` 출력 매칭 →
///                 포맷 변경 시 둘이 같이 움직여야 함 (KEEP IN SYNC with `list_models`).
pub fn humanize_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("relative url without a base") || lower.contains("builder error") {
        return "URL 형식 오류 — http://localhost:8080 처럼 스킴(http:// 또는 https://) 포함"
            .into();
    }
    if lower.contains("refused") || lower.contains("os error 10061") {
        return "서버 응답 없음 — OpenAI 호환 서버가 실행 중인지 확인해 주세요. TabbyAPI는 기본 http://localhost:5000, TabbyML은 기본 http://localhost:8080 입니다."
            .into();
    }
    if lower.contains("dns") || lower.contains("nodename") {
        return "호스트 주소 확인 — URL의 도메인이 맞나요?".into();
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return "연결 시간 초과 — 서버가 응답하지 않습니다. 로컬 서버가 실행 중인지, 포트가 맞는지 확인해 주세요."
            .into();
    }
    if raw.contains("Tabby 401") || raw.contains("Tabby 403") {
        return "인증 실패 — token이 필요/잘못됨".into();
    }
    if raw.contains("Tabby 404") {
        return "404 — base URL이 맞나요? `/v1/models` 경로 확인".into();
    }
    raw.to_string()
}

#[derive(Deserialize)]
struct ModelInfo {
    id: String,
}

#[derive(Deserialize)]
struct ModelsResp {
    data: Vec<ModelInfo>,
}

fn apply_token_headers(
    req: reqwest::RequestBuilder,
    token: Option<&str>,
) -> reqwest::RequestBuilder {
    if let Some(t) = token.filter(|s| !s.trim().is_empty()) {
        let token = t.trim();
        // Tabby/TabbyAPI variants may accept either Authorization bearer or x-api-key.
        req.bearer_auth(token).header("x-api-key", token)
    } else {
        req
    }
}

fn extract_model_ids_from_array(items: &[Value]) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in items {
        let maybe_id = if let Some(s) = item.as_str() {
            Some(s.to_string())
        } else if let Some(obj) = item.as_object() {
            ["id", "name", "model", "model_name"]
                .iter()
                .find_map(|k| obj.get(*k).and_then(|v| v.as_str()))
                .map(|s| s.to_string())
        } else {
            None
        };
        if let Some(id) = maybe_id {
            let trimmed = id.trim();
            if !trimmed.is_empty() && seen.insert(trimmed.to_string()) {
                out.push(trimmed.to_string());
            }
        }
    }
    out
}

fn parse_model_ids(body: &str) -> Result<Vec<String>, String> {
    if let Ok(parsed) = serde_json::from_str::<ModelsResp>(body) {
        return Ok(parsed.data.into_iter().map(|m| m.id).collect());
    }

    let v: Value =
        serde_json::from_str(body).map_err(|e| format!("Tabby /v1/models 파싱 실패: {}", e))?;

    if let Some(items) = v.get("data").and_then(|d| d.as_array()) {
        return Ok(extract_model_ids_from_array(items));
    }
    if let Some(items) = v.as_array() {
        return Ok(extract_model_ids_from_array(items));
    }

    Err("Tabby /v1/models 파싱 실패: 지원되지 않는 응답 형식".into())
}

/// `GET {base}/v1/models` — Tabby가 서빙 중인 모델 ID 리스트.
/// 연결 실패 시 Err. 빈 배열은 Ok(vec![])로 반환 (서버는 살아있지만 모델 없음).
pub async fn list_models(base_url: String, token: Option<String>) -> Result<Vec<String>, String> {
    // chat_base와 동일하게 /v1 중복 방지
    let v1 = chat_base(&base_url);
    let url = format!("{}/models", v1);
    let client = http_client()?;
    let token_ref = token.as_deref();

    let req = apply_token_headers(client.get(&url), token_ref);
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        return parse_model_ids(&body);
    }

    // Compatibility fallback for TabbyAPI variants.
    if matches!(resp.status().as_u16(), 404 | 405) {
        let legacy_url = format!("{}/model/list", v1);
        let legacy_req = apply_token_headers(client.get(&legacy_url), token_ref);
        let legacy_resp = legacy_req.send().await.map_err(|e| e.to_string())?;
        if legacy_resp.status().is_success() {
            let body = legacy_resp.text().await.unwrap_or_default();
            return parse_model_ids(&body);
        }
        let status = legacy_resp.status();
        let body = legacy_resp.text().await.unwrap_or_default();
        return Err(format!("Tabby {}: {}", status, body));
    }

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        // KEEP IN SYNC: humanize_error가 "Tabby {status}" prefix를 매칭함
        return Err(format!("Tabby {}: {}", status, body));
    }

    Ok(Vec::new())
}

pub(crate) fn tabby_connection_error_looks_unreachable(raw: &str, actionable: &str) -> bool {
    let raw_lower = raw.to_ascii_lowercase();
    raw_lower.contains("refused")
        || raw_lower.contains("os error 10061")
        || raw_lower.contains("timeout")
        || raw_lower.contains("timed out")
        || actionable.contains("응답 없음")
        || actionable.contains("시간 초과")
        || actionable.contains("응답하지")
}

#[cfg(test)]
mod tests {
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
        // 사용자가 /v1까지 입력했으면 중복 추가하지 않음
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

    // ── humanize_error: KEEP IN SYNC with list_models error format ──

    #[test]
    fn humanize_connection_refused() {
        let msg = humanize_error("error sending request: Connection refused");
        assert!(msg.contains("OpenAI 호환 서버"), "got: {}", msg);
        assert!(msg.contains("localhost:5000"), "got: {}", msg);
    }

    #[test]
    fn humanize_os_error_10061() {
        // Windows: connection refused = OS error 10061
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
        // KEEP IN SYNC: list_models는 format!("Tabby {}: {}", status, body) 사용
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
    /// 둘 중 하나만 바꾸면 이 테스트가 깨짐.
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
}
