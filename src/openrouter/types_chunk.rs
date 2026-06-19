use serde::Deserialize;

use super::types::FlexibleContent;

#[derive(Deserialize)]
pub(super) struct StreamChunk {
    #[serde(default)]
    pub(crate) id: Option<String>,
    pub(crate) choices: Vec<ChunkChoice>,
}

#[derive(Deserialize)]
pub(super) struct ChunkChoice {
    pub(crate) delta: Option<DeltaPayload>,
    pub(crate) message: Option<NonStreamMessage>,
    pub(crate) text: Option<String>,
    pub(crate) finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct DeltaPayload {
    pub(crate) content: Option<FlexibleContent>,
    pub(crate) text: Option<String>,
    pub(crate) value: Option<serde_json::Value>,
    pub(crate) output_text: Option<serde_json::Value>,
    pub(crate) reasoning_content: Option<String>,
    pub(crate) reasoning: Option<String>,
    pub(crate) tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Deserialize)]
pub(super) struct ToolCallDelta {
    pub(crate) index: u32,
    pub(crate) id: Option<String>,
    pub(crate) function: Option<ToolCallFunctionDelta>,
}

#[derive(Deserialize)]
pub(super) struct ToolCallFunctionDelta {
    pub(crate) name: Option<String>,
    pub(crate) arguments: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct NonStreamChatResponse {
    pub(crate) choices: Vec<NonStreamChoice>,
}

#[derive(Deserialize)]
pub(super) struct NonStreamChoice {
    pub(crate) message: Option<NonStreamMessage>,
    pub(crate) text: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct NonStreamMessage {
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
