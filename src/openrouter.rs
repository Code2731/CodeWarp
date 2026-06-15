// OpenRouter HTTP/SSE 클라이언트.
// list_models: 모델 리스트
// chat_stream: SSE 토큰 + tool_call delta 스트림

mod api;
mod humanize;
mod parse;
#[cfg(test)]
mod tests;
mod types;

pub use api::{
    chat_stream, get_account_info, get_generation, list_models, AuthKeyData, GenerationData,
};
pub use humanize::humanize_error;
pub use types::{ChatEvent, ChatMessage, OpenRouterModel, BASE_URL};
