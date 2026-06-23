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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn standard_chat_completion() {
        let v = json!({"choices": [{"message": {"content": "Hello"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn streaming_delta_content() {
        let v = json!({"choices": [{"delta": {"content": "Hello"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn text_only_choice() {
        let v = json!({"choices": [{"text": "Hello"}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn reasoning_content_in_message() {
        let v = json!({"choices": [{"message": {"reasoning_content": "thinking..."}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("thinking...")
        );
    }

    #[test]
    fn reasoning_in_message() {
        let v = json!({"choices": [{"message": {"reasoning": "deep think"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("deep think")
        );
    }

    #[test]
    fn top_level_message_key() {
        let v = json!({"message": {"content": "Hello"}});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn top_level_output_text() {
        let v = json!({"output_text": "Hello"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn nested_object_content_via_value_to_text() {
        let v =
            json!({"choices": [{"message": {"content": [{"text": "Hello"}, {"text": "World"}]}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("HelloWorld")
        );
    }

    #[test]
    fn null_content_returns_none() {
        let v = json!({"choices": [{"message": {"content": null}}]});
        assert_eq!(extract_non_stream_content_from_value(&v), None);
    }

    #[test]
    fn empty_choices_returns_none() {
        let v = json!({"choices": []});
        assert_eq!(extract_non_stream_content_from_value(&v), None);
    }

    #[test]
    fn invalid_shape_returns_none() {
        let v = json!({"foo": "bar"});
        assert_eq!(extract_non_stream_content_from_value(&v), None);
    }

    #[test]
    fn xllm_delta_value() {
        let v = json!({"choices": [{"delta": {"value": "Hello"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn multiple_choices_returns_first_match() {
        let v = json!({"choices": [
            {"message": {"content": "first"}},
            {"message": {"content": "second"}}
        ]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("first")
        );
    }

    #[test]
    fn first_choice_empty_second_has_content() {
        let v = json!({"choices": [
            {"message": {"content": null}},
            {"message": {"content": "second"}}
        ]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("second")
        );
    }

    #[test]
    fn content_as_array_of_parts() {
        let v =
            json!({"choices": [{"message": {"content": [{"text": "Hello"}, {"text": "World"}]}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("HelloWorld")
        );
    }

    #[test]
    fn delta_text_key() {
        let v = json!({"choices": [{"delta": {"text": "Hi"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hi")
        );
    }

    #[test]
    fn delta_output_text() {
        let v = json!({"choices": [{"delta": {"output_text": "Hi"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hi")
        );
    }

    #[test]
    fn delta_reasoning_content() {
        let v = json!({"choices": [{"delta": {"reasoning_content": "think"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("think")
        );
    }

    #[test]
    fn delta_reasoning() {
        let v = json!({"choices": [{"delta": {"reasoning": "think"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("think")
        );
    }

    #[test]
    fn message_value_key() {
        let v = json!({"choices": [{"message": {"value": "val"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("val")
        );
    }

    #[test]
    fn message_output_text() {
        let v = json!({"choices": [{"message": {"output_text": "out"}}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("out")
        );
    }

    #[test]
    fn choice_content_key() {
        let v = json!({"choices": [{"content": "Hi"}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("Hi")
        );
    }

    #[test]
    fn choice_value_key() {
        let v = json!({"choices": [{"value": "val"}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("val")
        );
    }

    #[test]
    fn choice_output_text() {
        let v = json!({"choices": [{"output_text": "out"}]});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("out")
        );
    }

    #[test]
    fn top_level_response() {
        let v = json!({"response": "resp"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("resp")
        );
    }

    #[test]
    fn top_level_text() {
        let v = json!({"text": "txt"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("txt")
        );
    }

    #[test]
    fn top_level_content() {
        let v = json!({"content": "cnt"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("cnt")
        );
    }

    #[test]
    fn top_level_value() {
        let v = json!({"value": "val"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("val")
        );
    }

    #[test]
    fn top_level_precedence_after_choices() {
        let v = json!({"choices": [{"message": {"content": "from_choices"}}], "output_text": "from_top"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("from_choices")
        );
    }

    #[test]
    fn choices_no_match_falls_through_to_top_level() {
        let v = json!({"choices": [{"foo": "bar"}], "output_text": "fallback"});
        assert_eq!(
            extract_non_stream_content_from_value(&v).as_deref(),
            Some("fallback")
        );
    }

    #[test]
    fn empty_content_string_returns_none() {
        let v = json!({"choices": [{"message": {"content": "  "}}]});
        assert_eq!(extract_non_stream_content_from_value(&v), None);
    }
}
