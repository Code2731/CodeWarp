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
#[cfg(test)]
mod tests;
mod types;

pub use api::{get_account_info, get_generation, list_models};
pub use api_types::{AuthKeyData, GenerationData};
pub use chat_stream::chat_stream;
pub use humanize::humanize_error;
pub use types::{ChatEvent, ChatMessage, OpenRouterModel, BASE_URL};
