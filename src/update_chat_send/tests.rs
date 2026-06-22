use super::*;
use std::sync::Arc;

#[test]
fn send_message_returns_early_when_streaming() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.input = "hello".into();
    let before = app.conversation.len();

    let _ = app.update(Message::Send);

    assert_eq!(
        app.conversation.len(),
        before,
        "should not send while streaming"
    );
    assert_eq!(app.streaming_block_id, Some(42));
}

#[test]
fn send_message_returns_early_when_input_empty() {
    let (mut app, _) = App::new();
    app.input.clear();

    let _ = app.update(Message::Send);

    assert!(
        app.status.is_empty() || app.status == "준비됨" || app.status.starts_with("[복구됨]"),
        "unexpected status: {}",
        app.status
    );
}

#[test]
fn regenerate_last_returns_early_when_streaming() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).push(ChatMessage::user("hello"));
    app.streaming_block_id = Some(42);
    let before = app.conversation.len();

    let _ = app.update(Message::RegenerateLast);

    assert_eq!(app.conversation.len(), before);
}

#[test]
fn regenerate_last_returns_early_when_no_user_message() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();

    let _ = app.update(Message::RegenerateLast);

    assert!(
        app.status.is_empty() || app.status == "준비됨" || app.status.starts_with("[복구됨]"),
        "unexpected status: {}",
        app.status
    );
}
