use serde::{Deserialize, Serialize};

/// OpenRouter chat 호출 base URL (endpoint 직전).
pub const BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterPricing {
    pub prompt: Option<String>,
    pub completion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterModel {
    pub id: String,
    pub name: Option<String>,
    pub context_length: Option<u64>,
    pub pricing: Option<OpenRouterPricing>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ModelsResponse {
    pub(crate) data: Vec<OpenRouterModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<serde_json::Value>,
}

impl ChatMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }
    pub fn assistant_tool_calls(tool_calls: serde_json::Value) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(tool_calls),
            ..Default::default()
        }
    }
    pub fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChatEvent {
    Token(String),
    ToolCallDelta {
        index: u32,
        id: Option<String>,
        name: Option<String>,
        arguments: Option<String>,
    },
    Done {
        finish_reason: Option<String>,
        generation_id: Option<String>,
    },
    Error(String),
}

#[derive(Serialize)]
pub(crate) struct ChatRequest<'a> {
    pub(crate) model: &'a str,
    pub(crate) messages: &'a [ChatMessage],
    pub(crate) stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<&'a str>,
}

#[derive(Deserialize)]
pub(crate) struct StreamChunk {
    #[serde(default)]
    pub(crate) id: Option<String>,
    pub(crate) choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
pub(crate) struct ChunkChoice {
    pub(crate) delta: Option<DeltaPayload>,
    pub(crate) message: Option<NonStreamMessage>,
    pub(crate) text: Option<String>,
    pub(crate) finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct DeltaPayload {
    pub(crate) content: Option<FlexibleContent>,
    pub(crate) text: Option<String>,
    pub(crate) value: Option<serde_json::Value>,
    pub(crate) output_text: Option<serde_json::Value>,
    pub(crate) reasoning_content: Option<String>,
    pub(crate) reasoning: Option<String>,
    pub(crate) tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Deserialize)]
pub(crate) struct ToolCallDelta {
    pub(crate) index: u32,
    pub(crate) id: Option<String>,
    pub(crate) function: Option<ToolCallFunctionDelta>,
}

#[derive(Deserialize)]
pub(crate) struct ToolCallFunctionDelta {
    pub(crate) name: Option<String>,
    pub(crate) arguments: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct NonStreamChatResponse {
    pub(crate) choices: Vec<NonStreamChoice>,
}

#[derive(Deserialize)]
pub(crate) struct NonStreamChoice {
    pub(crate) message: Option<NonStreamMessage>,
    pub(crate) text: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct NonStreamMessage {
    pub(crate) content: Option<FlexibleContent>,
    #[serde(default)]
    pub(crate) value: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) output_text: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) reasoning_content: Option<String>,
    #[serde(default)]
    pub(crate) reasoning: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum FlexibleContent {
    Text(String),
    Part(FlexibleContentPart),
    Parts(Vec<FlexibleContentPart>),
}

#[derive(Deserialize)]
pub(crate) struct FlexibleContentPart {
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) content: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) value: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) output_text: Option<serde_json::Value>,
}

impl FlexibleContent {
    pub(crate) fn into_text(self) -> Option<String> {
        match self {
            FlexibleContent::Text(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(s)
                }
            }
            FlexibleContent::Part(part) => part.into_text(),
            FlexibleContent::Parts(parts) => {
                let mut out = String::new();
                for part in parts {
                    if let Some(text) = part.into_text() {
                        out.push_str(&text);
                    }
                }
                if out.trim().is_empty() {
                    None
                } else {
                    Some(out)
                }
            }
        }
    }
}

impl FlexibleContentPart {
    pub(crate) fn into_text(self) -> Option<String> {
        if let Some(text) = self.text.and_then(normalize_non_empty_text) {
            return Some(text);
        }
        if let Some(text) = self.content.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = self.value.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = self.output_text.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        None
    }
    pub(crate) fn extract_text_ref(&self) -> Option<String> {
        if let Some(text) = self
            .text
            .as_ref()
            .and_then(|s| normalize_non_empty_text(s.clone()))
        {
            return Some(text);
        }
        if let Some(text) = self.content.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = self.value.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = self.output_text.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        None
    }
}

pub(crate) fn value_to_text(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => normalize_non_empty_text(s.clone()),
        serde_json::Value::Array(items) => {
            let mut out = String::new();
            for item in items {
                if let Some(text) = value_to_text(item) {
                    out.push_str(&text);
                }
            }
            normalize_non_empty_text(out)
        }
        serde_json::Value::Object(map) => {
            for key in [
                "text",
                "content",
                "value",
                "output_text",
                "message",
                "response",
            ] {
                if let Some(v) = map.get(key).and_then(value_to_text) {
                    return Some(v);
                }
            }
            if let Some(v) = map.get("parts").and_then(value_to_text) {
                return Some(v);
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn normalize_non_empty_text(text: String) -> Option<String> {
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}
