/// HF 원문 오류를 사용자 행동 가능한 메시지로 변환.
pub fn humanize_error(raw: &str) -> String {
    let lc = raw.to_lowercase();

    if contains_status(raw, 401) || contains_status(raw, 403) {
        return "권한 없음(401/403) — Hugging Face 토큰을 저장했는지, 게이트 모델 접근 권한이 있는지 확인해 주세요.".into();
    }
    if contains_status(raw, 404) {
        if lc.contains("revision") || lc.contains("not found") {
            return "리비전/브랜치를 찾을 수 없음(404) — 프리셋 브랜치가 바뀌었을 수 있어요. 다른 프리셋으로 재시도해 주세요.".into();
        }
        return "리소스를 찾을 수 없음(404) — repo ID 또는 파일 경로를 다시 확인해 주세요.".into();
    }
    if lc.contains("timeout") || lc.contains("timed out") {
        return "요청 시간 초과 — 네트워크 상태를 확인하고 다시 시도해 주세요.".into();
    }
    if lc.contains("dns")
        || lc.contains("name or service not known")
        || lc.contains("failed to lookup address")
    {
        return "DNS 조회 실패 — 인터넷 연결 또는 DNS 설정을 확인해 주세요.".into();
    }
    if lc.contains("tls")
        || lc.contains("certificate")
        || lc.contains("handshake")
        || lc.contains("secure connection")
    {
        return "TLS/인증서 오류 — 시스템 시간/인증서 저장소/보안 SW를 확인한 뒤 다시 시도해 주세요.".into();
    }
    if lc.contains("connection reset")
        || lc.contains("connection refused")
        || lc.contains("unexpected eof")
    {
        return "연결 실패 — 잠시 후 재시도하거나 네트워크/방화벽 설정을 확인해 주세요.".into();
    }

    raw.to_string()
}

pub(crate) fn contains_status(raw: &str, code: u16) -> bool {
    let mut digits = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        if digits.len() == 3 && digits.parse::<u16>().ok() == Some(code) {
            return true;
        }
        digits.clear();
    }
    digits.len() == 3 && digits.parse::<u16>().ok() == Some(code)
}
