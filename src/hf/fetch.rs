// hf/fetch.rs — HTTP fetch functions (hf child module)
use crate::hf::helpers::*;
use crate::hf::types::*;

pub(crate) async fn fetch_repo_branches(
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

pub(crate) async fn fetch_model_tree(
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

pub(crate) async fn fetch_model_info(
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

pub(crate) fn http_client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.2.0")
        .build()
        .map_err(|e| format!("HTTP client 생성 실패: {e}"))
}
