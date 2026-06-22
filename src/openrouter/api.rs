// openrouter/api.rs — REST API functions (openrouter child module)
use super::api_types::{
    http_client, AuthKeyData, AuthKeyResponse, GenerationData, GenerationResponse,
};
use super::types::{ModelsResponse, OpenRouterModel};

pub(crate) async fn get_generation(api_key: String, id: String) -> Result<GenerationData, String> {
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    let client = http_client()?;
    let resp = client
        .get(format!("https://openrouter.ai/api/v1/generation?id={id}"))
        .bearer_auth(&api_key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("generation {status}: {body}"));
    }
    let parsed: GenerationResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}

pub(crate) async fn get_account_info(api_key: String) -> Result<AuthKeyData, String> {
    let client = http_client()?;
    let resp = client
        .get("https://openrouter.ai/api/v1/auth/key")
        .bearer_auth(&api_key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter {status}: {body}"));
    }
    let parsed: AuthKeyResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}

pub(crate) async fn list_models(api_key: String) -> Result<Vec<OpenRouterModel>, String> {
    let client = http_client()?;
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .bearer_auth(&api_key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter {status}: {body}"));
    }
    let parsed: ModelsResponse = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parsed.data)
}
