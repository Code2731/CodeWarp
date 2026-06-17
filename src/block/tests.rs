use super::*;
use iced::widget::text_editor;

fn ub(id: u64) -> Block {
    Block {
        id,
        body: BlockBody::User(format!("u{}", id)),
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    }
}
fn ab(id: u64) -> Block {
    Block {
        id,
        body: BlockBody::Assistant(text_editor::Content::with_text(&format!("a{}", id))),
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    }
}
fn tb(id: u64) -> Block {
    Block {
        id,
        body: BlockBody::ToolResult {
            name: "x".into(),
            summary: "y".into(),
            success: true,
        },
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    }
}

fn cm(role: &str, content: &str) -> crate::openrouter::ChatMessage {
    crate::openrouter::ChatMessage {
        role: role.into(),
        content: Some(content.into()),
        ..Default::default()
    }
}

#[test]
fn truncate_empty_conv() {
    let mut conv: Vec<crate::openrouter::ChatMessage> = Vec::new();
    truncate_after_last_user(&mut conv);
    assert!(conv.is_empty());
}

#[test]
fn truncate_user_only() {
    let mut conv = vec![cm("user", "hi")];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 1);
    assert_eq!(conv[0].role, "user");
}

#[test]
fn truncate_user_assistant() {
    let mut conv = vec![cm("user", "hi"), cm("assistant", "hello")];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 1);
    assert_eq!(conv[0].content.as_deref(), Some("hi"));
}

#[test]
fn truncate_keeps_last_user_intact() {
    let mut conv = vec![
        cm("user", "first"),
        cm("assistant", "answer1"),
        cm("user", "second"),
    ];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 3);
    assert_eq!(conv[2].content.as_deref(), Some("second"));
}

#[test]
fn truncate_user_tool_assistant_chain() {
    let mut conv = vec![
        cm("system", "sys"),
        cm("user", "hi"),
        cm("assistant", "let me check"),
        cm("tool", "result"),
        cm("assistant", "done"),
    ];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 2);
    assert_eq!(conv[0].role, "system");
    assert_eq!(conv[1].role, "user");
}

#[test]
fn truncate_no_user_drops_all() {
    let mut conv = vec![cm("system", "sys"), cm("assistant", "lone")];
    truncate_after_last_user(&mut conv);
    assert!(conv.is_empty());
}

#[test]
fn last_user_idx_empty() {
    assert_eq!(last_user_block_idx(&[]), None);
}

#[test]
fn last_user_idx_only_user() {
    let blocks = vec![ub(1)];
    assert_eq!(last_user_block_idx(&blocks), Some(0));
}

#[test]
fn last_user_idx_picks_last() {
    let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
    assert_eq!(last_user_block_idx(&blocks), Some(2));
}

#[test]
fn last_user_idx_no_user() {
    let blocks = vec![ab(1), tb(2)];
    assert_eq!(last_user_block_idx(&blocks), None);
}

#[test]
fn last_assistant_idx_picks_last_assistant() {
    let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
    assert_eq!(last_assistant_block_idx(&blocks), Some(3));
}

#[test]
fn last_assistant_idx_no_assistant() {
    let blocks = vec![ub(1), ub(2)];
    assert_eq!(last_assistant_block_idx(&blocks), None);
}

#[test]
fn persisted_assistant_blocks_default_to_raw_for_selection() {
    let block = persisted_to_block(session::PersistedBlock {
        id: 1,
        role: "assistant".into(),
        content: "selectable answer".into(),
        model: "local".into(),
    });

    assert_eq!(block.view_mode, ViewMode::Raw);
    assert_eq!(block.body.to_text(), "selectable answer");
}

#[test]
fn persisted_user_blocks_keep_rendered_layout() {
    let block = persisted_to_block(session::PersistedBlock {
        id: 1,
        role: "user".into(),
        content: "hello".into(),
        model: String::new(),
    });

    assert_eq!(block.view_mode, ViewMode::Rendered);
}
