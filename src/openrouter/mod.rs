// OpenRouter HTTP/SSE 클라이언트.
// list_models: 모델 리스트
// chat_stream: SSE 토큰 + tool_call delta 스트림

mod api;
mod api_types;
mod chat_stream;
mod humanize;
#[cfg(test)]
mod humanize_tests;
mod parse;
mod parse_sse;
mod parse_values;
#[cfg(test)]
mod sse_tests;
#[cfg(test)]
mod tests;
mod types;
mod types_chunk;
mod types_flexible;

pub(crate) use api::{get_account_info, get_generation, list_models};
pub(crate) use api_types::{AuthKeyData, GenerationData};
pub(crate) use chat_stream::chat_stream;
pub(crate) use humanize::humanize_error;
pub(crate) use types::{BASE_URL, ChatEvent, ChatMessage, OpenRouterModel};
