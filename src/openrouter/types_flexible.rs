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
