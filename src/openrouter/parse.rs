use super::types::{
    ChunkChoice, FlexibleContent, NonStreamChatResponse, normalize_non_empty_text, value_to_text,
};

use super::parse_values::extract_non_stream_content_from_value;

pub(super) use super::parse_sse::*;

pub(super) fn extract_non_stream_content(raw: &str) -> Option<String> {
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

pub(super) fn extract_stream_text(choice: &ChunkChoice) -> Option<String> {
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
