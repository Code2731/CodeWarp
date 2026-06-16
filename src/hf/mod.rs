use std::io::Write;

use futures_util::{Stream, StreamExt};

mod error;
mod types;
pub(crate) use error::*;
pub(crate) use types::*;

mod helpers;
pub(crate) use helpers::*;

mod fetch;
pub(crate) use fetch::*;

#[cfg(test)]
mod tests;

// ── Public download API ─────────────────────────────────────────────

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

        let mut info: ModelInfo = match fetch_model_info(&client, &repo_id, token_ref, &rev).await {
            Ok(v) => v,
            Err(e) => {
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
        match fetch_model_tree(&client, &repo_id, token_ref, &rev).await {
            Ok(tree) if !tree.siblings.is_empty() => {
                info = tree;
            }
            Ok(_) => {}
            Err(e) => {
                yield DownloadEvent::Error(format!(
                    "HF file tree fetch failed for revision '{}': {}",
                    rev, e
                ));
                return;
            }
        }
        let total_files = info.siblings.len();
        yield DownloadEvent::Started { total_files };
        let rev_path = encode_path_segment(&rev);

        let safe_id = folder_name.unwrap_or_else(|| repo_id.replace('/', "--"));
        let target_root = dest_dir.join(&safe_id);
        if let Err(e) = std::fs::create_dir_all(&target_root) {
            yield DownloadEvent::Error(format!("디렉토리 생성 실패: {}", e));
            return;
        }

        for (idx, sibling) in info.siblings.iter().enumerate() {
            let filename = &sibling.rfilename;
            let encoded_filename = encode_repo_file_path(filename);
            let dl_url = format!(
                "{}/{}/resolve/{}/{}",
                HF_BASE, repo_id, rev_path, encoded_filename
            );
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
