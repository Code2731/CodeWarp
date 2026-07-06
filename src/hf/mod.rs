use std::io::Write;

use futures_util::{Stream, StreamExt};

mod error;
mod types;
pub(crate) use error::*;
pub(crate) use types::*;

mod encoding;
use encoding::{encode_path_segment, encode_repo_file_path};
mod revision;

mod fetch;
use fetch::{fetch_model_info_with_fallback, fetch_model_tree, http_client};

#[cfg(test)]
mod tests;

// ── Public download API ─────────────────────────────────────────────

struct DownloadSetup {
    client: reqwest::Client,
    info: ModelInfo,
    target_root: std::path::PathBuf,
    rev_path: String,
}

async fn init_download(
    repo_id: &str,
    dest_dir: &std::path::Path,
    token: &Option<String>,
    revision: &Option<String>,
    folder_name: &Option<String>,
) -> Result<DownloadSetup, String> {
    let client = http_client()?;
    let token_ref = token.as_deref();
    let mut rev = revision.as_deref().unwrap_or("main").to_string();
    let requested_rev = rev.clone();

    let mut info =
        fetch_model_info_with_fallback(&client, repo_id, token_ref, &mut rev, &requested_rev)
            .await?;
    match fetch_model_tree(&client, repo_id, token_ref, &rev).await {
        Ok(tree) if !tree.siblings.is_empty() => info = tree,
        Ok(_) => {}
        Err(e) => {
            return Err(format!(
                "HF file tree fetch failed for revision '{rev}': {e}"
            ));
        }
    }
    let rev_path = encode_path_segment(&rev);
    let safe_id = folder_name
        .clone()
        .unwrap_or_else(|| repo_id.replace('/', "--"));
    let target_root = dest_dir.join(&safe_id);
    std::fs::create_dir_all(&target_root).map_err(|e| format!("디렉토리 생성 실패: {e}"))?;
    Ok(DownloadSetup {
        client,
        info,
        target_root,
        rev_path,
    })
}

/// `repo_id` 예: "turboderp/Llama-3.2-1B-Instruct-exl2". siblings를
/// `dest_dir/<folder_name>/{filename}`으로 저장. revision으로 branch 선택 (EXL2 bpw).
pub(crate) fn download_repo(
    repo_id: String,
    dest_dir: std::path::PathBuf,
    token: Option<String>,
    revision: Option<String>,
    folder_name: Option<String>,
) -> impl Stream<Item = DownloadEvent> {
    async_stream::stream! {
        let setup = match init_download(&repo_id, &dest_dir, &token, &revision, &folder_name).await {
            Ok(s) => s,
            Err(e) => { yield DownloadEvent::Error(e); return; }
        };
        let total_files = setup.info.siblings.len();
        yield DownloadEvent::Started { total_files };

        for (idx, sibling) in setup.info.siblings.iter().enumerate() {
            let filename = &sibling.rfilename;
            let encoded_filename = encode_repo_file_path(filename);
            let dl_url = format!(
                "{HF_BASE}/{repo_id}/resolve/{}/{encoded_filename}",
                setup.rev_path
            );
            let mut request = setup.client.get(&dl_url);
            if let Some(t) = token.as_ref().filter(|s| !s.trim().is_empty()) {
                request = request.bearer_auth(t.trim());
            }
            let resp = match request.send().await {
                Ok(r) => r,
                Err(e) => { yield DownloadEvent::Error(e.to_string()); return; }
            };
            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                yield DownloadEvent::Error(format!("HF {status} ({filename}): {body}"));
                return;
            }
            let total_bytes = resp.content_length();
            yield DownloadEvent::FileStart {
                idx,
                name: filename.clone(),
                size: total_bytes,
            };

            let target_file = setup.target_root.join(filename);
            if let Some(parent) = target_file.parent()
                && let Err(e) = std::fs::create_dir_all(parent) {
                    yield DownloadEvent::Error(format!("디렉토리 생성 실패: {e}"));
                    return;
                }

            let mut file = match std::fs::File::create(&target_file) {
                Ok(f) => f,
                Err(e) => {
                    yield DownloadEvent::Error(format!("파일 생성 실패: {e}"));
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
                    yield DownloadEvent::Error(format!("쓰기 실패: {e}"));
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
