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
    Started { total_files: usize },
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
    FileDone { idx: usize },
    AllDone,
    Error(String),
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .build()
        .expect("reqwest client 빌드 실패")
}

/// `repo_id` 예: "TabbyML/Qwen2.5-Coder-7B". 모든 siblings를
/// `dest_dir/<repo--id>/{filename}`으로 저장. token이 Some이면 bearer 첨부 (gated repo).
pub fn download_repo(
    repo_id: String,
    dest_dir: std::path::PathBuf,
    token: Option<String>,
) -> impl Stream<Item = DownloadEvent> {
    async_stream::stream! {
        let client = http_client();

        // 1) siblings 메타
        let info_url = format!("{}/api/models/{}", HF_BASE, repo_id);
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

        // 2) 다운로드 디렉토리 보장 (repo id의 '/'를 '--'로)
        let safe_id = repo_id.replace('/', "--");
        let target_root = dest_dir.join(&safe_id);
        if let Err(e) = std::fs::create_dir_all(&target_root) {
            yield DownloadEvent::Error(format!("디렉토리 생성 실패: {}", e));
            return;
        }

        // 3) 파일별 스트림 다운로드
        for (idx, sibling) in info.siblings.iter().enumerate() {
            let filename = &sibling.rfilename;
            let dl_url = format!("{}/{}/resolve/main/{}", HF_BASE, repo_id, filename);
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
            yield DownloadEvent::FileDone { idx };
        }

        yield DownloadEvent::AllDone;
    }
}
