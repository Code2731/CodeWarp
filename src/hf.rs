// HuggingFace Hub 다운로드 — siblings 리스트 + 파일별 stream 다운로드.
// Iced Task::run으로 받아 view에 진행률 표시.

use std::io::Write;

use futures_util::{Stream, StreamExt};
use serde::Deserialize;

const HF_BASE: &str = "https://huggingface.co";
const PROGRESS_BYTES: u64 = 1024 * 1024; // 1MB마다 progress emit

#[derive(Deserialize)]
struct ModelInfo {
    siblings: Vec<Sibling>,
}

#[derive(Deserialize)]
struct Sibling {
    rfilename: String,
}

#[derive(Deserialize)]
struct RepoRefs {
    branches: Vec<RepoBranch>,
}

#[derive(Deserialize)]
struct RepoBranch {
    name: String,
}

/// 다운로드 진행 이벤트 — Stream으로 emit.
#[derive(Debug, Clone)]
pub enum DownloadEvent {
    /// 메타 fetched, 곧 파일 다운로드 시작.
    Started {
        total_files: usize,
    },
    /// 새 파일 시작.
    FileStart {
        idx: usize,
        name: String,
        size: Option<u64>,
    },
    /// 1MB마다 또는 파일 끝에 emit.
    FileProgress {
        idx: usize,
        bytes_done: u64,
        bytes_total: Option<u64>,
    },
    FileDone,
    AllDone,
    Error(String),
}

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

fn contains_status(raw: &str, code: u16) -> bool {
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

fn normalize_revision_name(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn extract_bpw_value(s: &str) -> Option<f32> {
    let lower = s.to_lowercase();
    let bpw_idx = lower.find("bpw")?;
    let prefix = &lower[..bpw_idx];
    let bytes = prefix.as_bytes();
    let mut end = bytes.len();
    while end > 0 && !bytes[end - 1].is_ascii_digit() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }
    let mut start = end;
    while start > 0 {
        let b = bytes[start - 1];
        if b.is_ascii_digit() || b == b'.' {
            start -= 1;
        } else {
            break;
        }
    }
    if start >= end {
        return None;
    }
    prefix[start..end].parse::<f32>().ok()
}

fn choose_revision_fallback(requested: &str, branches: &[String]) -> Option<String> {
    if branches.is_empty() {
        return None;
    }

    if let Some(hit) = branches
        .iter()
        .find(|b| b.eq_ignore_ascii_case(requested))
        .cloned()
    {
        return Some(hit);
    }

    let requested_norm = normalize_revision_name(requested);
    if !requested_norm.is_empty() {
        if let Some(hit) = branches
            .iter()
            .find(|b| normalize_revision_name(b) == requested_norm)
            .cloned()
        {
            return Some(hit);
        }
    }

    if let Some(target) = extract_bpw_value(requested) {
        let mut best: Option<(f32, String)> = None;
        for b in branches {
            if let Some(v) = extract_bpw_value(b) {
                let dist = (v - target).abs();
                match &best {
                    Some((best_dist, best_name))
                        if dist > *best_dist || (dist == *best_dist && b >= best_name) => {}
                    _ => best = Some((dist, b.clone())),
                }
            }
        }
        if let Some((_, name)) = best {
            return Some(name);
        }
    }

    branches
        .iter()
        .find(|b| b.eq_ignore_ascii_case("main"))
        .cloned()
        .or_else(|| branches.first().cloned())
}

fn format_branch_suggestions(branches: &[String], limit: usize) -> String {
    let shown: Vec<&str> = branches
        .iter()
        .map(|b| b.trim())
        .filter(|b| !b.is_empty())
        .take(limit)
        .collect();
    if shown.is_empty() {
        return String::new();
    }
    let mut text = shown.join(", ");
    if branches.len() > shown.len() {
        text.push_str(&format!(" ... +{} more", branches.len() - shown.len()));
    }
    text
}

fn annotate_revision_not_found_error(base: &str, requested: &str, branches: &[String]) -> String {
    let suggested = format_branch_suggestions(branches, 8);
    if suggested.is_empty() {
        return base.to_string();
    }
    format!(
        "{} (requested revision: '{}'; available branches: {})",
        base, requested, suggested
    )
}

fn encode_path_segment(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn model_info_url(repo_id: &str, rev: &str) -> String {
    if rev == "main" {
        format!("{}/api/models/{}", HF_BASE, repo_id)
    } else {
        format!(
            "{}/api/models/{}/revision/{}",
            HF_BASE,
            repo_id,
            encode_path_segment(rev)
        )
    }
}

async fn fetch_repo_branches(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
) -> Option<Vec<String>> {
    let refs_url = format!("{}/api/models/{}/refs", HF_BASE, repo_id);
    let mut req = client.get(&refs_url);
    if let Some(t) = token.filter(|s| !s.trim().is_empty()) {
        req = req.bearer_auth(t.trim());
    }
    let resp = req.send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let refs: RepoRefs = resp.json().await.ok()?;
    let out: Vec<String> = refs
        .branches
        .into_iter()
        .map(|b| b.name)
        .filter(|name| !name.trim().is_empty())
        .collect();
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

async fn fetch_model_info(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
    rev: &str,
) -> Result<ModelInfo, String> {
    let info_url = model_info_url(repo_id, rev);
    let mut req = client.get(&info_url);
    if let Some(t) = token.filter(|s| !s.trim().is_empty()) {
        req = req.bearer_auth(t.trim());
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HF {}: {}", status, body));
    }
    resp.json()
        .await
        .map_err(|e| format!("repo info 파싱 실패: {}", e))
}

fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .build()
        .map_err(|e| format!("HTTP client 생성 실패: {e}"))
}

/// `repo_id` 예: "turboderp/Llama-3.2-1B-Instruct-exl2". siblings를
/// `dest_dir/<folder_name>/{filename}`으로 저장. revision으로 branch 선택 (EXL2 bpw).
pub fn download_repo(
    repo_id: String,
    dest_dir: std::path::PathBuf,
    token: Option<String>,
    revision: Option<String>,
    folder_name: Option<String>,
) -> impl Stream<Item = DownloadEvent> {
    async_stream::stream! {
        let client = match http_client() {
            Ok(c) => c,
            Err(e) => { yield DownloadEvent::Error(e); return; }
        };
        let token_ref = token.as_deref();
        let mut rev = revision.as_deref().unwrap_or("main").to_string();
        let requested_rev = rev.clone();

        // 1) siblings 메타 (revision 쿼리 파라미터로 branch 지정)
        let info: ModelInfo = match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
            Ok(v) => v,
            Err(e) => {
                // EXL2 프리셋처럼 branch가 바뀐 경우 404면 refs에서 fallback branch를 찾아 1회 재시도.
                if rev != "main" && contains_status(&e, 404) {
                    if let Some(branches) = fetch_repo_branches(&client, &repo_id, token_ref).await {
                        if let Some(fallback) = choose_revision_fallback(&rev, &branches) {
                            if !fallback.eq_ignore_ascii_case(&rev) {
                                rev = fallback;
                                match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
                                    Ok(v2) => v2,
                                    Err(e2) => {
                                        let prev = rev.clone();
                                        rev = "main".to_string();
                                        match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
                                            Ok(v3) => v3,
                                            Err(e3) => {
                                                let decorated = annotate_revision_not_found_error(
                                                    &format!(
                                                        "{} (fallback retry from '{}' to '{}' failed; main fallback from '{}' failed: {}; requested revision: '{}')",
                                                        e2, requested_rev, prev, prev, e3, requested_rev
                                                    ),
                                                    &requested_rev,
                                                    &branches,
                                                );
                                                yield DownloadEvent::Error(decorated);
                                                return;
                                            }
                                        }
                                    }
                                }
                            } else {
                                rev = "main".to_string();
                                match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
                                    Ok(v3) => v3,
                                    Err(e3) => {
                                        let decorated = annotate_revision_not_found_error(
                                            &format!(
                                                "{} (fallback matched requested revision '{}'; main fallback failed: {}; requested revision: '{}')",
                                                e, requested_rev, e3, requested_rev
                                            ),
                                            &requested_rev,
                                            &branches,
                                        );
                                        yield DownloadEvent::Error(decorated);
                                        return;
                                    }
                                }
                            }
                        } else {
                            rev = "main".to_string();
                            match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
                                Ok(v3) => v3,
                                Err(e3) => {
                                    let decorated = annotate_revision_not_found_error(
                                        &format!(
                                            "{} (no fallback branch match; main fallback failed: {}; requested revision: '{}')",
                                            e, e3, requested_rev
                                        ),
                                        &requested_rev,
                                        &branches,
                                    );
                                    yield DownloadEvent::Error(decorated);
                                    return;
                                }
                            }
                        }
                    } else {
                        rev = "main".to_string();
                        match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
                            Ok(v3) => v3,
                            Err(e3) => {
                                yield DownloadEvent::Error(format!(
                                    "{} (fallback lookup failed: branch refs unavailable; main fallback failed: {}; requested revision: '{}')",
                                    e, e3, requested_rev
                                ));
                                return;
                            }
                        }
                    }
                } else {
                    yield DownloadEvent::Error(e);
                    return;
                }
            }
        };
        let total_files = info.siblings.len();
        yield DownloadEvent::Started { total_files };
        let rev_path = encode_path_segment(&rev);

        // 2) 다운로드 디렉토리 보장
        let safe_id = folder_name.unwrap_or_else(|| repo_id.replace('/', "--"));
        let target_root = dest_dir.join(&safe_id);
        if let Err(e) = std::fs::create_dir_all(&target_root) {
            yield DownloadEvent::Error(format!("디렉토리 생성 실패: {}", e));
            return;
        }

        // 3) 파일별 스트림 다운로드
        for (idx, sibling) in info.siblings.iter().enumerate() {
            let filename = &sibling.rfilename;
            let dl_url = format!("{}/{}/resolve/{}/{}", HF_BASE, repo_id, rev_path, filename);
            let mut req = client.get(&dl_url);
            if let Some(t) = token.as_ref().filter(|s| !s.trim().is_empty()) {
                req = req.bearer_auth(t.trim());
            }
            let resp = match req.send().await {
                Ok(r) => r,
                Err(e) => { yield DownloadEvent::Error(e.to_string()); return; }
            };
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                yield DownloadEvent::Error(format!("HF {} ({}): {}", status, filename, body));
                return;
            }
            let total_bytes = resp.content_length();
            yield DownloadEvent::FileStart {
                idx,
                name: filename.clone(),
                size: total_bytes,
            };

            // 하위 디렉토리도 보장 (예: "model-00001-of-00002.safetensors"는 plain이지만
            // "tokenizer/sub.json" 같은 path가 있을 수 있음)
            let target_file = target_root.join(filename);
            if let Some(parent) = target_file.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    yield DownloadEvent::Error(format!("디렉토리 생성 실패: {}", e));
                    return;
                }
            }

            let mut file = match std::fs::File::create(&target_file) {
                Ok(f) => f,
                Err(e) => {
                    yield DownloadEvent::Error(format!("파일 생성 실패: {}", e));
                    return;
                }
            };

            let mut bytes_done: u64 = 0;
            let mut last_emit: u64 = 0;
            let mut stream = resp.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(b) => b,
                    Err(e) => { yield DownloadEvent::Error(e.to_string()); return; }
                };
                if let Err(e) = file.write_all(&chunk) {
                    yield DownloadEvent::Error(format!("쓰기 실패: {}", e));
                    return;
                }
                bytes_done += chunk.len() as u64;
                if bytes_done - last_emit >= PROGRESS_BYTES {
                    yield DownloadEvent::FileProgress {
                        idx,
                        bytes_done,
                        bytes_total: total_bytes,
                    };
                    last_emit = bytes_done;
                }
            }
            // 파일 끝에 한 번 더 (마지막 < 1MB 잔여 표시)
            yield DownloadEvent::FileProgress {
                idx,
                bytes_done,
                bytes_total: total_bytes,
            };
            yield DownloadEvent::FileDone;
        }

        yield DownloadEvent::AllDone;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        annotate_revision_not_found_error, choose_revision_fallback, contains_status,
        encode_path_segment, extract_bpw_value, format_branch_suggestions, humanize_error,
        model_info_url, normalize_revision_name,
    };

    #[test]
    fn humanize_auth() {
        let msg = humanize_error("HF 401: unauthorized");
        assert!(msg.contains("토큰"));
    }

    #[test]
    fn humanize_auth_lowercase_prefix() {
        let msg = humanize_error("hf 403: forbidden");
        assert!(msg.contains("권한"));
    }

    #[test]
    fn humanize_auth_status_text() {
        let msg = humanize_error("request failed with status 401 Unauthorized");
        assert!(msg.contains("401/403"));
    }

    #[test]
    fn humanize_404_revision() {
        let msg = humanize_error("HF 404: revision not found");
        assert!(msg.contains("리비전"));
    }

    #[test]
    fn humanize_timeout() {
        let msg = humanize_error("operation timed out");
        assert!(msg.contains("시간 초과"));
    }

    #[test]
    fn contains_status_matches_standalone_code() {
        assert!(contains_status("status=404 not found", 404));
        assert!(!contains_status("file size 1404 bytes", 404));
    }

    #[test]
    fn normalize_revision_name_compacts_symbols() {
        assert_eq!(normalize_revision_name("4.0-bpw"), "40bpw");
        assert_eq!(normalize_revision_name(" 4_0 BPW "), "40bpw");
    }

    #[test]
    fn extract_bpw_value_parses_number() {
        assert_eq!(extract_bpw_value("4.0bpw"), Some(4.0));
        assert_eq!(extract_bpw_value("exl2-6.5bpw"), Some(6.5));
        assert_eq!(extract_bpw_value("main"), None);
    }

    #[test]
    fn choose_revision_fallback_prefers_closest_bpw() {
        let branches = vec![
            "3.0bpw".to_string(),
            "4.5bpw".to_string(),
            "6.0bpw".to_string(),
        ];
        assert_eq!(
            choose_revision_fallback("4.0bpw", &branches).as_deref(),
            Some("4.5bpw")
        );
    }

    #[test]
    fn choose_revision_fallback_uses_main_when_no_bpw_match() {
        let branches = vec!["dev".to_string(), "main".to_string()];
        assert_eq!(
            choose_revision_fallback("unknown-branch", &branches).as_deref(),
            Some("main")
        );
    }

    #[test]
    fn format_branch_suggestions_limits_and_counts() {
        let branches = vec![
            "a".to_string(),
            "b".to_string(),
            "c".to_string(),
            "d".to_string(),
        ];
        assert_eq!(format_branch_suggestions(&branches, 2), "a, b ... +2 more");
    }

    #[test]
    fn annotate_revision_not_found_error_appends_requested_and_candidates() {
        let branches = vec!["main".to_string(), "4.0bpw".to_string()];
        let text =
            annotate_revision_not_found_error("HF 404: revision not found", "4bpw", &branches);
        assert!(text.contains("requested revision: '4bpw'"));
        assert!(text.contains("available branches: main, 4.0bpw"));
    }

    #[test]
    fn encode_path_segment_keeps_safe_revision_names() {
        assert_eq!(encode_path_segment("4.0bpw"), "4.0bpw");
        assert_eq!(encode_path_segment("main"), "main");
    }

    #[test]
    fn encode_path_segment_escapes_reserved_chars() {
        assert_eq!(
            encode_path_segment("branch with/slash"),
            "branch%20with%2Fslash"
        );
    }

    #[test]
    fn model_info_url_uses_revision_path_endpoint() {
        assert_eq!(
            model_info_url("owner/repo", "4.0bpw"),
            "https://huggingface.co/api/models/owner/repo/revision/4.0bpw"
        );
    }

    #[test]
    fn model_info_url_keeps_main_on_base_model_endpoint() {
        assert_eq!(
            model_info_url("owner/repo", "main"),
            "https://huggingface.co/api/models/owner/repo"
        );
    }
}
