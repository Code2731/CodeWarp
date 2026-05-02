use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use tauri::ipc::Channel;

const KEYRING_SERVICE: &str = "codewarp";
const KEYRING_USER_OPENROUTER: &str = "openrouter_api_key";

type AppResult<T> = Result<T, String>;

fn err_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn keyring_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER_OPENROUTER).map_err(err_str)
}

fn read_api_key() -> AppResult<String> {
    let entry = keyring_entry()?;
    entry.get_password().map_err(|e| match e {
        keyring::Error::NoEntry => "API 키가 저장되어 있지 않습니다.".to_string(),
        other => other.to_string(),
    })
}

#[tauri::command]
fn set_openrouter_key(key: String) -> AppResult<()> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err("API 키가 비어 있습니다.".into());
    }
    let entry = keyring_entry()?;
    entry.set_password(trimmed).map_err(err_str)?;
    Ok(())
}

#[tauri::command]
fn has_openrouter_key() -> AppResult<bool> {
    let entry = keyring_entry()?;
    match entry.get_password() {
        Ok(_) => Ok(true),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn clear_openrouter_key() -> AppResult<()> {
    let entry = keyring_entry()?;
    match entry.delete_credential() {
        Ok(_) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    context_length: Option<u64>,
    pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("CodeWarp/0.1.0")
        .build()
        .expect("reqwest client 빌드 실패")
}

#[tauri::command]
async fn list_openrouter_models() -> AppResult<Vec<OpenRouterModel>> {
    let key = read_api_key()?;
    let client = http_client();
    let resp = client
        .get("https://openrouter.ai/api/v1/models")
        .bearer_auth(&key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .send()
        .await
        .map_err(err_str)?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenRouter {}: {}", status, body));
    }

    let parsed: OpenRouterModelsResponse = resp.json().await.map_err(err_str)?;
    Ok(parsed.data)
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequestBody<'a> {
    model: &'a str,
    messages: &'a [ChatMessage],
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct DeltaPayload {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChoicePayload {
    delta: Option<DeltaPayload>,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<ChoicePayload>,
}

#[derive(Clone, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum ChatEvent {
    Token(String),
    Done,
    Error(String),
}

#[tauri::command]
async fn chat_stream(
    model: String,
    messages: Vec<ChatMessage>,
    on_event: Channel<ChatEvent>,
) -> AppResult<()> {
    let key = read_api_key()?;
    let client = http_client();
    let body = ChatRequestBody {
        model: &model,
        messages: &messages,
        stream: true,
    };

    let resp = match client
        .post("https://openrouter.ai/api/v1/chat/completions")
        .bearer_auth(&key)
        .header("HTTP-Referer", "https://codewarp.app")
        .header("X-Title", "CodeWarp")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            let _ = on_event.send(ChatEvent::Error(e.to_string()));
            return Err(e.to_string());
        }
    };

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        let msg = format!("OpenRouter {}: {}", status, text);
        let _ = on_event.send(ChatEvent::Error(msg.clone()));
        return Err(msg);
    }

    let mut stream = resp.bytes_stream();
    let mut buffer = String::new();

    while let Some(item) = stream.next().await {
        let chunk = match item {
            Ok(b) => b,
            Err(e) => {
                let _ = on_event.send(ChatEvent::Error(e.to_string()));
                return Err(e.to_string());
            }
        };
        let text = match std::str::from_utf8(&chunk) {
            Ok(s) => s,
            Err(_) => continue,
        };
        buffer.push_str(text);

        loop {
            let Some(idx) = buffer.find('\n') else {
                break;
            };
            let line = buffer[..idx].trim_end_matches('\r').to_string();
            buffer.drain(..=idx);

            let Some(payload) = line.strip_prefix("data:") else {
                continue;
            };
            let payload = payload.trim();
            if payload.is_empty() {
                continue;
            }
            if payload == "[DONE]" {
                let _ = on_event.send(ChatEvent::Done);
                return Ok(());
            }
            match serde_json::from_str::<StreamChunk>(payload) {
                Ok(parsed) => {
                    for choice in parsed.choices {
                        if let Some(delta) = choice.delta {
                            if let Some(content) = delta.content {
                                if !content.is_empty() {
                                    let _ = on_event.send(ChatEvent::Token(content));
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // OpenRouter는 가끔 keepalive comment(': ...')를 보냄. 무시.
                }
            }
        }
    }

    let _ = on_event.send(ChatEvent::Done);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            set_openrouter_key,
            has_openrouter_key,
            clear_openrouter_key,
            list_openrouter_models,
            chat_stream,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
