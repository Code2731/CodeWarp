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
mod tabby_tests;
