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
        let rev = revision.as_deref().unwrap_or("main");

        // 1) siblings 메타 (revision 쿼리 파라미터로 branch 지정)
        let info_url = if rev == "main" {
            format!("{}/api/models/{}", HF_BASE, repo_id)
        } else {
            format!("{}/api/models/{}?revision={}", HF_BASE, repo_id, rev)
        };
        let mut req = client.get(&info_url);
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
            yield DownloadEvent::Error(format!("HF {}: {}", status, body));
            return;
        }
        let info: ModelInfo = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                yield DownloadEvent::Error(format!("repo info 파싱 실패: {}", e));
                return;
            }
        };
        let total_files = info.siblings.len();
        yield DownloadEvent::Started { total_files };

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
            let dl_url = format!("{}/{}/resolve/{}/{}", HF_BASE, repo_id, rev, filename);
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
    use super::{contains_status, humanize_error};

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
}
