// Tabby (TabbyML) HTTP 클라이언트 — OpenAI 호환 /v1/models 엔드포인트 사용.
// chat completion 라우팅은 Step 4에서 추가.

use serde::Deserialize;

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("reqwest client 빌드 실패")
}

fn normalize_base(url: &str) -> String {
    let trimmed = url.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        "http://localhost:8080".into()
    } else {
        trimmed.to_string()
    }
}

/// `chat_stream`에 넘길 base URL ("/v1" 접두 포함).
pub fn chat_base(url: &str) -> String {
    format!("{}/v1", normalize_base(url))
}

/// 연결 에러 원문을 사용자 친화 actionable 메시지로 변환.
pub fn humanize_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("refused") || lower.contains("os error 10061") {
        return "서버 응답 없음 — `tabby serve` 실행 중인지 확인 (기본 8080)".into();
    }
    if lower.contains("dns") || lower.contains("nodename") {
        return "호스트 주소 확인 — URL의 도메인이 맞나요?".into();
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return "응답 지연 — 서버는 살아있지만 5초 내 응답 없음".into();
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

/// `GET {base}/v1/models` — Tabby가 서빙 중인 모델 ID 리스트.
/// 연결 실패 시 Err. 빈 배열은 Ok(vec![])로 반환 (서버는 살아있지만 모델 없음).
pub async fn list_models(base_url: String, token: Option<String>) -> Result<Vec<String>, String> {
    let base = normalize_base(&base_url);
    let url = format!("{}/v1/models", base);
    let client = http_client();
    let mut req = client.get(&url);
    if let Some(t) = token.as_ref().filter(|s| !s.trim().is_empty()) {
        req = req.bearer_auth(t.trim());
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Tabby {}: {}", status, body));
    }
    let parsed: ModelsResp = resp
        .json()
        .await
        .map_err(|e| format!("Tabby /v1/models 파싱 실패: {}", e))?;
    Ok(parsed.data.into_iter().map(|m| m.id).collect())
}
