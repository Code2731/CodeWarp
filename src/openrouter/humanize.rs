/// 연결/HTTP 에러 원문을 사용자 친화 actionable 메시지로 변환.
pub fn humanize_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if raw.contains("OpenRouter 401") || raw.contains("OpenRouter 403") {
        return "키 무효 — Settings에서 재발급/재입력".into();
    }
    if raw.contains("OpenRouter 402") {
        return "잔액 부족 — openrouter.ai/credits 에서 충전".into();
    }
    if raw.contains("OpenRouter 429") {
        return "rate limit — 잠시 후 재시도 (또는 다른 모델)".into();
    }
    if raw.contains("OpenRouter 404") {
        return "모델 ID 무효 — 모델 셀렉터에서 다시 선택".into();
    }
    if raw.contains("OpenRouter 5") {
        return "OpenRouter 서버 일시 오류 — 잠시 후 재시도".into();
    }
    if lower.contains("dns") || lower.contains("nodename") {
        return "DNS 해석 실패 — 인터넷 연결 확인".into();
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return "응답 지연 — 인터넷 연결 또는 OpenRouter 상태 확인".into();
    }
    raw.to_string()
}
