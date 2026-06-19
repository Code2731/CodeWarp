use crate::*;
use futures_util::StreamExt;
use std::sync::Arc;

pub(super) async fn collect_chat_text(
    base_url: String,
    api_key: Option<String>,
    model: String,
    messages: Arc<Vec<ChatMessage>>,
) -> Result<String, String> {
    let stream = openrouter::chat_stream(base_url, api_key, model, messages, None);
    futures_util::pin_mut!(stream);
    let mut out = String::new();
    while let Some(event) = stream.next().await {
        match event {
            ChatEvent::Token(t) => out.push_str(&t),
            ChatEvent::Done { .. } => return Ok(out),
            ChatEvent::Error(e) => return Err(e),
            ChatEvent::ToolCallDelta { .. } => {}
        }
    }
    Ok(out)
}
