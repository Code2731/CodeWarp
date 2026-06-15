use super::types::{
    normalize_non_empty_text, value_to_text, ChunkChoice, FlexibleContent, NonStreamChatResponse,
    StreamChunk,
};

pub(crate) fn extract_non_stream_content(raw: &str) -> Option<String> {
    if let Ok(parsed) = serde_json::from_str::<NonStreamChatResponse>(raw) {
        let from_struct = parsed.choices.into_iter().find_map(|choice| {
            choice
                .message
                .and_then(|message| {
                    message
                        .content
                        .and_then(FlexibleContent::into_text)
                        .or_else(|| {
                            message
                                .reasoning_content
                                .and_then(normalize_non_empty_text)
                                .or_else(|| message.reasoning.and_then(normalize_non_empty_text))
                        })
                })
                .or_else(|| {
                    choice.text.and_then(|text| {
                        if text.trim().is_empty() {
                            None
                        } else {
                            Some(text)
                        }
                    })
                })
        });
        if from_struct.is_some() {
            return from_struct;
        }
    }
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    extract_non_stream_content_from_value(&value)
}

fn extract_non_stream_content_from_value(value: &serde_json::Value) -> Option<String> {
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

pub(crate) fn normalize_stream_payload_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let payload = trimmed
        .strip_prefix("data:")
        .map(str::trim)
        .unwrap_or(trimmed);
    if payload.is_empty() {
        None
    } else {
        Some(payload)
    }
}

pub(crate) fn flush_pending_sse_data(pending_data: &mut String) -> Option<String> {
    if pending_data.is_empty() {
        return None;
    }
    let payload = pending_data.trim_end_matches('\n').to_string();
    pending_data.clear();
    if payload.trim().is_empty() {
        None
    } else {
        Some(payload)
    }
}

pub(crate) fn consume_sse_line(line: &str, pending_data: &mut String) -> Option<String> {
    let trimmed = line.trim_end_matches('\r').trim();
    if trimmed.is_empty() {
        return flush_pending_sse_data(pending_data);
    }
    if let Some(data_part) = trimmed.strip_prefix("data:") {
        pending_data.push_str(data_part.trim_start());
        pending_data.push('\n');
        return None;
    }
    if trimmed == "data" {
        pending_data.push('\n');
        return None;
    }
    if trimmed.starts_with(':') {
        return None;
    }
    if !pending_data.is_empty() {
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            pending_data.push_str(trimmed);
            pending_data.push('\n');
            return None;
        }
        return None;
    }
    normalize_stream_payload_line(trimmed).map(str::to_string)
}

pub(crate) fn parse_stream_chunks(payload: &str) -> Vec<StreamChunk> {
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    if let Ok(parsed) = serde_json::from_str::<StreamChunk>(trimmed) {
        return vec![parsed];
    }
    let mut stream_items = Vec::new();
    let mut had_error = false;
    for item in serde_json::Deserializer::from_str(trimmed).into_iter::<StreamChunk>() {
        match item {
            Ok(chunk) => stream_items.push(chunk),
            Err(_) => {
                had_error = true;
                break;
            }
        }
    }
    if !stream_items.is_empty() && !had_error {
        return stream_items;
    }
    let mut out = Vec::new();
    for line in trimmed.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(parsed) = serde_json::from_str::<StreamChunk>(line) {
            out.push(parsed);
        }
    }
    out
}

pub(crate) fn extract_plain_stream_token(payload: &str) -> Option<String> {
    let text = payload.trim();
    if text.is_empty() {
        return None;
    }
    if text.starts_with('{') || text.starts_with('[') {
        return None;
    }
    Some(text.to_string())
}

pub(crate) fn extract_stream_text(choice: &ChunkChoice) -> Option<String> {
    if let Some(delta) = choice.delta.as_ref() {
        if let Some(content) = delta.content.as_ref() {
            let text = match content {
                FlexibleContent::Text(s) => {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.clone())
                    }
                }
                FlexibleContent::Part(part) => part.extract_text_ref(),
                FlexibleContent::Parts(parts) => {
                    let mut out = String::new();
                    for part in parts {
                        if let Some(text) = part.extract_text_ref() {
                            out.push_str(&text);
                        }
                    }
                    normalize_non_empty_text(out)
                }
            };
            if text.is_some() {
                return text;
            }
        }
        if let Some(text) = delta
            .text
            .as_ref()
            .and_then(|s| normalize_non_empty_text(s.clone()))
        {
            return Some(text);
        }
        if let Some(text) = delta.value.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = delta.output_text.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = delta
            .reasoning_content
            .as_ref()
            .and_then(|s| normalize_non_empty_text(s.clone()))
        {
            return Some(text);
        }
        if let Some(text) = delta
            .reasoning
            .as_ref()
            .and_then(|s| normalize_non_empty_text(s.clone()))
        {
            return Some(text);
        }
    }
    if let Some(text) = choice
        .text
        .as_ref()
        .and_then(|s| normalize_non_empty_text(s.clone()))
    {
        return Some(text);
    }
    if let Some(message) = choice.message.as_ref() {
        if let Some(content) = message.content.as_ref().and_then(|c| match c {
            FlexibleContent::Text(s) => normalize_non_empty_text(s.clone()),
            FlexibleContent::Part(part) => part.extract_text_ref(),
            FlexibleContent::Parts(parts) => {
                let mut out = String::new();
                for part in parts {
                    if let Some(text) = part.extract_text_ref() {
                        out.push_str(&text);
                    }
                }
                normalize_non_empty_text(out)
            }
        }) {
            return Some(content);
        }
        if let Some(text) = message.value.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = message.output_text.as_ref().and_then(value_to_text) {
            return Some(text);
        }
        if let Some(text) = message
            .reasoning_content
            .as_ref()
            .and_then(|s| normalize_non_empty_text(s.clone()))
        {
            return Some(text);
        }
        if let Some(text) = message
            .reasoning
            .as_ref()
            .and_then(|s| normalize_non_empty_text(s.clone()))
        {
            return Some(text);
        }
    }
    None
}
