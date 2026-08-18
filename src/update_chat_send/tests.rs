use super::*;
use crate::{LlmProvider, ModelOption};
use iced::widget::text_editor::{Action, Edit};
use std::sync::Arc;

#[test]
fn input_actions_keep_editor_and_state_in_sync_for_unicode_text() {
    let (mut app, _) = App::new();

    let _ = app.update(Message::InputAction(Action::Edit(Edit::Insert('한'))));
    let _ = app.update(Message::InputAction(Action::Edit(Edit::Insert('글'))));

    assert_eq!(app.input, "한글");
    assert_eq!(app.editor_content.text(), "한글");
}

#[test]
fn input_action_edits_external_value_without_reversing_text() {
    let (mut app, _) = App::new();

    let _ = app.update(Message::InputChanged("abc".into()));
    let _ = app.update(Message::InputAction(Action::Edit(Edit::Backspace)));

    assert_eq!(app.input, "ab");
    assert_eq!(app.editor_content.text(), "ab");
}

#[test]
fn paste_and_newline_actions_preserve_unicode_order() {
    let (mut app, _) = App::new();
    let pasted = "첫 줄 😊\n두 번째 줄";

    let _ = app.update(Message::InputAction(Action::Edit(Edit::Paste(Arc::new(
        pasted.into(),
    )))));
    let _ = app.update(Message::InputAction(Action::Edit(Edit::Enter)));
    let _ = app.update(Message::InputAction(Action::Edit(Edit::Insert('끝'))));

    assert_eq!(app.input, "첫 줄 😊\n두 번째 줄\n끝");
    assert_eq!(app.editor_content.text(), app.input);
}

#[test]
fn cursor_edit_does_not_reset_to_the_end_or_reverse_text() {
    let (mut app, _) = App::new();

    let _ = app.update(Message::InputAction(Action::Edit(Edit::Paste(Arc::new(
        "ab".into(),
    )))));
    let _ = app.update(Message::InputAction(Action::Move(
        iced::widget::text_editor::Motion::Left,
    )));
    let _ = app.update(Message::InputAction(Action::Edit(Edit::Insert('X'))));

    assert_eq!(app.input, "aXb");
    assert_eq!(app.editor_content.text(), "aXb");
}

#[test]
fn edit_last_user_places_cursor_at_end() {
    let (mut app, _) = App::new();
    app.blocks.push(Block {
        id: 1,
        body: BlockBody::User("hello".into()),
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    Arc::make_mut(&mut app.conversation).push(ChatMessage::user("hello"));

    let _ = app.update(Message::EditLastUser);
    let _ = app.update(Message::InputAction(Action::Edit(Edit::Insert('!'))));

    assert_eq!(app.input, "hello!");
    assert_eq!(app.editor_content.text(), "hello!");
}

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
    let before_status = app.status.clone();

    let _ = app.update(Message::Send);

    assert_eq!(app.status, before_status);
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
    let before_status = app.status.clone();

    let _ = app.update(Message::RegenerateLast);

    assert_eq!(app.status, before_status);
}

#[test]
fn send_message_preserves_prompt_whitespace_and_newlines() {
    let (mut app, _) = App::new();
    app.model_options = vec![ModelOption {
        id: "local-model".into(),
        provider: LlmProvider::OpenAICompat,
        provider_label: "Ollama".into(),
        ko_friendly: false,
        favorite: false,
        context_length: None,
        prompt_per_million: Some(0.0),
        completion_per_million: Some(0.0),
    }];
    app.selected_model = Some("local-model".into());
    app.selected_model_provider = Some(LlmProvider::OpenAICompat);
    app.tabby_url_input = "http://127.0.0.1:11434".into();
    app.input = "  code block  \n".into();

    let _ = app.send_message();

    assert_eq!(
        app.conversation
            .last()
            .and_then(|message| message.content.as_deref()),
        Some("  code block  \n")
    );
    assert_eq!(app.blocks[0].body.to_text(), "  code block  \n");
}
