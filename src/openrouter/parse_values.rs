// openrouter/parse_values.rs — Value extraction helpers (openrouter child module)
use super::types::value_to_text;

pub(super) fn extract_non_stream_content_from_value(value: &serde_json::Value) -> Option<String> {
    if let Some(choices) = value.get("choices").and_then(|v| v.as_array()) {
        for choice in choices {
            if let Some(msg) = choice.get("message") {
                if let Some(content) = msg.get("content").and_then(value_to_text) {
                    return Some(content);
                }
                if let Some(content) = msg.get("value").and_then(value_to_text) {
                    return Some(content);
                }
                if let Some(content) = msg.get("output_text").and_then(value_to_text) {
                    return Some(content);
                }
                if let Some(reasoning) = msg.get("reasoning_content").and_then(value_to_text) {
                    return Some(reasoning);
                }
                if let Some(reasoning) = msg.get("reasoning").and_then(value_to_text) {
                    return Some(reasoning);
                }
            }
            if let Some(delta) = choice.get("delta") {
                if let Some(content) = delta.get("content").and_then(value_to_text) {
                    return Some(content);
                }
                if let Some(text) = delta.get("text").and_then(value_to_text) {
                    return Some(text);
                }
                if let Some(content) = delta.get("value").and_then(value_to_text) {
                    return Some(content);
                }
                if let Some(content) = delta.get("output_text").and_then(value_to_text) {
                    return Some(content);
                }
                if let Some(reasoning) = delta.get("reasoning_content").and_then(value_to_text) {
                    return Some(reasoning);
                }
                if let Some(reasoning) = delta.get("reasoning").and_then(value_to_text) {
                    return Some(reasoning);
                }
            }
            if let Some(text) = choice.get("text").and_then(value_to_text) {
                return Some(text);
            }
            if let Some(content) = choice.get("content").and_then(value_to_text) {
                return Some(content);
            }
            if let Some(content) = choice.get("value").and_then(value_to_text) {
                return Some(content);
            }
            if let Some(content) = choice.get("output_text").and_then(value_to_text) {
                return Some(content);
            }
        }
    }
    for key in [
        "output_text",
        "response",
        "text",
        "content",
        "message",
        "value",
    ] {
        if let Some(v) = value.get(key).and_then(value_to_text) {
            return Some(v);
        }
    }
    None
}
