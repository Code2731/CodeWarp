use std::io::Write;

use futures_util::{Stream, StreamExt};

mod error;
mod types;
pub(crate) use error::*;
pub(crate) use types::*;

#[cfg(test)]
mod tests;

// ── Revision helpers ────────────────────────────────────────────────

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

// ── URL helpers ─────────────────────────────────────────────────────

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

fn encode_repo_file_path(input: &str) -> String {
    input
        .split('/')
        .map(encode_path_segment)
        .collect::<Vec<_>>()
        .join("/")
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

fn model_tree_url(repo_id: &str, rev: &str) -> String {
    format!(
        "{}/api/models/{}/tree/{}?recursive=true",
        HF_BASE,
        repo_id,
        encode_path_segment(rev)
    )
}

// ── HTTP fetch functions ────────────────────────────────────────────

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

async fn fetch_model_tree(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
    rev: &str,
) -> Result<ModelInfo, String> {
    let tree_url = model_tree_url(repo_id, rev);
    let mut req = client.get(&tree_url);
    if let Some(t) = token.filter(|s| !s.trim().is_empty()) {
        req = req.bearer_auth(t.trim());
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HF tree {}: {}", status, body));
    }
    let entries: Vec<TreeEntry> = resp
        .json()
        .await
        .map_err(|e| format!("repo tree parsing failed: {}", e))?;
    let siblings = entries
        .into_iter()
        .filter(|entry| {
            !entry.path.trim().is_empty()
                && !entry
                    .kind
                    .as_deref()
                    .unwrap_or_default()
                    .eq_ignore_ascii_case("directory")
        })
        .map(|entry| Sibling {
            rfilename: entry.path,
        })
        .collect();
    Ok(ModelInfo { siblings })
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
