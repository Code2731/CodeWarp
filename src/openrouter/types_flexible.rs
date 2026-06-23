use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum FlexibleContent {
    Text(String),
    Part(FlexibleContentPart),
    Parts(Vec<FlexibleContentPart>),
}

#[derive(Deserialize)]
pub(super) struct FlexibleContentPart {
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

pub(super) fn value_to_text(value: &serde_json::Value) -> Option<String> {
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

pub(super) fn normalize_non_empty_text(text: String) -> Option<String> {
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── normalize_non_empty_text ──────────────────────────────────────
    #[test]
    fn normalize_empty_string() {
        assert_eq!(normalize_non_empty_text(String::new()), None);
    }
    #[test]
    fn normalize_whitespace_only() {
        assert_eq!(normalize_non_empty_text("   ".to_string()), None);
    }
    #[test]
    fn normalize_non_empty() {
        assert_eq!(
            normalize_non_empty_text("hello".to_string()),
            Some("hello".to_string())
        );
    }

    // ── value_to_text ─────────────────────────────────────────────────
    #[test]
    fn value_to_text_string() {
        assert_eq!(value_to_text(&json!("hi")), Some("hi".to_string()));
    }
    #[test]
    fn value_to_text_array() {
        assert_eq!(
            value_to_text(&json!(["a", "b", "c"])),
            Some("abc".to_string())
        );
    }
    #[test]
    fn value_to_text_object_text_key() {
        assert_eq!(
            value_to_text(&json!({"text": "abc"})),
            Some("abc".to_string())
        );
    }
    #[test]
    fn value_to_text_object_content_key() {
        assert_eq!(
            value_to_text(&json!({"content": "xyz"})),
            Some("xyz".to_string())
        );
    }
    #[test]
    fn value_to_text_object_message_key() {
        assert_eq!(
            value_to_text(&json!({"message": {"content": "deep"}})),
            Some("deep".to_string())
        );
    }
    #[test]
    fn value_to_text_object_response_key() {
        assert_eq!(
            value_to_text(&json!({"response": "ok"})),
            Some("ok".to_string())
        );
    }
    #[test]
    fn value_to_text_object_parts_array() {
        assert_eq!(
            value_to_text(&json!({"parts": [{"text": "a"}, {"text": "b"}]})),
            Some("ab".to_string())
        );
    }
    #[test]
    fn value_to_text_empty_string() {
        assert_eq!(value_to_text(&json!("")), None);
    }
    #[test]
    fn value_to_text_empty_array() {
        assert_eq!(value_to_text(&json!([])), None);
    }
    #[test]
    fn value_to_text_null() {
        assert_eq!(value_to_text(&json!(null)), None);
    }
    #[test]
    fn value_to_text_number() {
        assert_eq!(value_to_text(&json!(42)), None);
    }
    #[test]
    fn value_to_text_bool() {
        assert_eq!(value_to_text(&json!(false)), None);
    }
    #[test]
    fn value_to_text_nested_object() {
        assert_eq!(
            value_to_text(&json!({"message": {"content": "hello"}})),
            Some("hello".to_string())
        );
    }
    #[test]
    fn value_to_text_unknown_object() {
        assert_eq!(value_to_text(&json!({"foo": "bar"})), None);
    }

    // ── FlexibleContent::into_text ────────────────────────────────────
    #[test]
    fn content_text_non_empty() {
        let c: FlexibleContent = serde_json::from_value(json!("hello")).unwrap();
        assert_eq!(c.into_text(), Some("hello".to_string()));
    }
    #[test]
    fn content_text_whitespace() {
        let c: FlexibleContent = serde_json::from_value(json!("  ")).unwrap();
        assert_eq!(c.into_text(), None);
    }
    #[test]
    fn content_part_with_text() {
        let c: FlexibleContent = serde_json::from_value(json!({"text": "part_text"})).unwrap();
        assert_eq!(c.into_text(), Some("part_text".to_string()));
    }
    #[test]
    fn content_parts_multiple() {
        let c: FlexibleContent = serde_json::from_value(json!([
            {"text": "a"},
            {"text": "b"}
        ]))
        .unwrap();
        assert_eq!(c.into_text(), Some("ab".to_string()));
    }
    #[test]
    fn content_parts_empty() {
        let c: FlexibleContent = serde_json::from_value(json!([])).unwrap();
        assert_eq!(c.into_text(), None);
    }

    // ── FlexibleContentPart::into_text ────────────────────────────────
    #[test]
    fn part_into_text_field() {
        let p = FlexibleContentPart {
            text: Some("from_text".to_string()),
            content: None,
            value: None,
            output_text: None,
        };
        assert_eq!(p.into_text(), Some("from_text".to_string()));
    }
    #[test]
    fn part_into_content_field() {
        let p = FlexibleContentPart {
            text: None,
            content: Some(json!("from_content")),
            value: None,
            output_text: None,
        };
        assert_eq!(p.into_text(), Some("from_content".to_string()));
    }
    #[test]
    fn part_into_content_array() {
        let p = FlexibleContentPart {
            text: None,
            content: Some(json!(["a", "b"])),
            value: None,
            output_text: None,
        };
        assert_eq!(p.into_text(), Some("ab".to_string()));
    }
    #[test]
    fn part_into_value_field() {
        let p = FlexibleContentPart {
            text: None,
            content: None,
            value: Some(json!("from_value")),
            output_text: None,
        };
        assert_eq!(p.into_text(), Some("from_value".to_string()));
    }
    #[test]
    fn part_into_output_text_field() {
        let p = FlexibleContentPart {
            text: None,
            content: None,
            value: None,
            output_text: Some(json!("from_output")),
        };
        assert_eq!(p.into_text(), Some("from_output".to_string()));
    }
    #[test]
    fn part_into_all_none() {
        let p = FlexibleContentPart {
            text: None,
            content: None,
            value: None,
            output_text: None,
        };
        assert_eq!(p.into_text(), None);
    }

    // ── FlexibleContentPart::extract_text_ref ─────────────────────────
    #[test]
    fn part_extract_text_field() {
        let p = FlexibleContentPart {
            text: Some("ref_text".to_string()),
            content: None,
            value: None,
            output_text: None,
        };
        assert_eq!(p.extract_text_ref(), Some("ref_text".to_string()));
    }
    #[test]
    fn part_extract_content_field() {
        let p = FlexibleContentPart {
            text: None,
            content: Some(json!("ref_content")),
            value: None,
            output_text: None,
        };
        assert_eq!(p.extract_text_ref(), Some("ref_content".to_string()));
    }
    #[test]
    fn part_extract_value_field() {
        let p = FlexibleContentPart {
            text: None,
            content: None,
            value: Some(json!("ref_value")),
            output_text: None,
        };
        assert_eq!(p.extract_text_ref(), Some("ref_value".to_string()));
    }
    #[test]
    fn part_extract_output_text_field() {
        let p = FlexibleContentPart {
            text: None,
            content: None,
            value: None,
            output_text: Some(json!("ref_output")),
        };
        assert_eq!(p.extract_text_ref(), Some("ref_output".to_string()));
    }
    #[test]
    fn part_extract_all_none() {
        let p = FlexibleContentPart {
            text: None,
            content: None,
            value: None,
            output_text: None,
        };
        assert_eq!(p.extract_text_ref(), None);
    }
    #[test]
    fn part_extract_priority_text_over_content() {
        let p = FlexibleContentPart {
            text: Some("prio_text".to_string()),
            content: Some(json!("ignored")),
            value: None,
            output_text: None,
        };
        // text field wins over content
        assert_eq!(p.extract_text_ref(), Some("prio_text".to_string()));
    }
}
