// hf/fetch.rs — HTTP fetch functions (hf child module)
use crate::hf::encoding::{model_info_url, model_tree_url};
use crate::hf::revision::{annotate_revision_not_found_error, choose_revision_fallback};
use crate::hf::types::{HF_BASE, ModelInfo, RepoRefs, Sibling, TreeEntry};

pub(super) async fn fetch_repo_branches(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
) -> Option<Vec<String>> {
    let refs_url = format!("{HF_BASE}/api/models/{repo_id}/refs");
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
    if out.is_empty() { None } else { Some(out) }
}

pub(super) async fn fetch_model_tree(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
    rev: &str,
) -> Result<ModelInfo, String> {
    let tree_url = model_tree_url(repo_id, rev);
    let mut request = client.get(&tree_url);
    if let Some(t) = token.filter(|s| !s.trim().is_empty()) {
        request = request.bearer_auth(t.trim());
    }
    let resp = request.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HF tree {status}: {body}"));
    }
    let entries: Vec<TreeEntry> = resp
        .json()
        .await
        .map_err(|e| format!("repo tree parsing failed: {e}"))?;
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

pub(super) async fn fetch_model_info(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
    rev: &str,
) -> Result<ModelInfo, String> {
    let info_url = model_info_url(repo_id, rev);
    let mut request = client.get(&info_url);
    if let Some(t) = token.filter(|s| !s.trim().is_empty()) {
        request = request.bearer_auth(t.trim());
    }
    let resp = request.send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("HF {status}: {body}"));
    }
    resp.json()
        .await
        .map_err(|e| format!("repo info 파싱 실패: {e}"))
}

pub(super) async fn fetch_model_info_with_fallback(
    client: &reqwest::Client,
    repo_id: &str,
    token: Option<&str>,
    rev: &mut String,
    requested_rev: &str,
) -> Result<ModelInfo, String> {
    match fetch_model_info(client, repo_id, token, rev).await {
        Ok(v) => Ok(v),
        Err(e) => {
            if rev.as_str() != "main" && crate::hf::error::contains_status(&e, 404) {
                if let Some(branches) = fetch_repo_branches(client, repo_id, token).await {
                    if let Some(fallback) = choose_revision_fallback(rev, &branches) {
                        if fallback.eq_ignore_ascii_case(rev) {
                            *rev = "main".to_string();
                            match fetch_model_info(client, repo_id, token, rev).await {
                                Ok(v3) => Ok(v3),
                                Err(e3) => Err(annotate_revision_not_found_error(
                                    &format!(
                                        "{e} (fallback matched requested revision '{requested_rev}'; main fallback failed: {e3}; requested revision: '{requested_rev}')"
                                    ),
                                    requested_rev,
                                    &branches,
                                )),
                            }
                        } else {
                            *rev = fallback;
                            match fetch_model_info(client, repo_id, token, rev).await {
                                Ok(v2) => Ok(v2),
                                Err(e2) => {
                                    let prev = rev.clone();
                                    *rev = "main".to_string();
                                    match fetch_model_info(client, repo_id, token, rev).await {
                                        Ok(v3) => Ok(v3),
                                        Err(e3) => Err(annotate_revision_not_found_error(
                                            &format!(
                                                "{e2} (fallback retry from '{requested_rev}' to '{prev}' failed; main fallback from '{prev}' failed: {e3}; requested revision: '{requested_rev}')"
                                            ),
                                            requested_rev,
                                            &branches,
                                        )),
                                    }
                                }
                            }
                        }
                    } else {
                        *rev = "main".to_string();
                        match fetch_model_info(client, repo_id, token, rev).await {
                            Ok(v3) => Ok(v3),
                            Err(e3) => Err(annotate_revision_not_found_error(
                                &format!(
                                    "{e} (no fallback branch match; main fallback failed: {e3}; requested revision: '{requested_rev}')"
                                ),
                                requested_rev,
                                &branches,
                            )),
                        }
                    }
                } else {
                    *rev = "main".to_string();
                    match fetch_model_info(client, repo_id, token, rev).await {
                        Ok(v3) => Ok(v3),
                        Err(e3) => Err(format!(
                            "{e} (fallback lookup failed: branch refs unavailable; main fallback failed: {e3}; requested revision: '{requested_rev}')"
                        )),
                    }
                }
            } else {
                Err(e)
            }
        }
    }
}

pub(super) fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .build()
        .map_err(|e| format!("HTTP client 생성 실패: {e}"))
}
