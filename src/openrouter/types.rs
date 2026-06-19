use serde::{Deserialize, Serialize};

/// OpenRouter chat 호출 base URL (endpoint 직전).
pub(crate) const BASE_URL: &str = "https://openrouter.ai/api/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OpenRouterPricing {
    pub prompt: Option<String>,
    pub completion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct OpenRouterModel {
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
pub(crate) struct ChatMessage {
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
    pub(crate) fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }
    pub(crate) fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: Some(content.into()),
            ..Default::default()
        }
    }
    pub(crate) fn assistant_tool_calls(tool_calls: serde_json::Value) -> Self {
        Self {
            role: "assistant".into(),
            content: None,
            tool_calls: Some(tool_calls),
            ..Default::default()
        }
    }
    pub(crate) fn tool_result(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: Some(content.into()),
            tool_call_id: Some(tool_call_id.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) enum ChatEvent {
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
pub(super) struct ChatRequest<'a> {
    pub(crate) model: &'a str,
    pub(crate) messages: &'a [ChatMessage],
    pub(crate) stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tools: Option<&'a serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<&'a str>,
}

pub(super) use super::types_chunk::*;
pub(super) use super::types_flexible::*;
