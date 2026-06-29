use super::types_flexible::{
    FlexibleContent, FlexibleContentPart, normalize_non_empty_text, value_to_text,
};
use serde_json::json;

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
    let c: FlexibleContent = serde_json::from_value(json!([{"text": "a"}, {"text": "b"}])).unwrap();
    assert_eq!(c.into_text(), Some("ab".to_string()));
}

#[test]
fn content_parts_empty() {
    let c: FlexibleContent = serde_json::from_value(json!([])).unwrap();
    assert_eq!(c.into_text(), None);
}

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
    assert_eq!(p.extract_text_ref(), Some("prio_text".to_string()));
}
