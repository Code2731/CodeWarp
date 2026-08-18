use super::*;
use crate::{Block, ChatEvent, LlmProvider, ModelOption, ViewMode};

#[test]
fn chat_chunk_done_builds_content_from_streaming_raw() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    app.streaming_raw = "hello world".into();

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Done {
            finish_reason: Some("stop".into()),
            generation_id: None,
        },
    });

    assert_eq!(app.blocks[0].body.to_text(), "hello world");
    assert!(!app.blocks[0].md_items.is_empty());
    assert!(app.streaming_raw.is_empty());
    assert!(app.streaming_block_id.is_none());
    assert_eq!(app.conversation.len(), 1);
    assert_eq!(app.conversation[0].content.as_deref(), Some("hello world"));
}

#[test]
fn chat_chunk_done_empty_streaming_raw_shows_warning() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    app.streaming_raw = String::new();

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Done {
            finish_reason: Some("stop".into()),
            generation_id: None,
        },
    });

    assert_eq!(app.blocks[0].body.to_text(), "[WARN] empty response");
    assert!(app.conversation.is_empty());
}

#[test]
fn chat_chunk_error_appends_to_streaming_raw() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    app.streaming_raw = "partial text".into();
    app.mid_stream_retries = MAX_MID_STREAM_RETRIES;

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Error("server error".into()),
    });

    assert!(app.blocks[0].body.to_text().contains("partial text"));
    assert!(
        app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] server error")
    );
    assert!(app.streaming_block_id.is_none());
}

#[test]
fn chat_chunk_error_empty_streaming_raw() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Error("server error".into()),
    });

    assert!(
        app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] server error")
    );
    assert!(app.streaming_block_id.is_none());
}

#[test]
fn mid_stream_error_triggers_retry() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    app.streaming_raw = "partial text".into();
    app.mid_stream_retries = 0;

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Error("connection dropped".into()),
    });

    assert_eq!(app.mid_stream_retries, 1);
    assert!(app.blocks[0].body.to_text().is_empty());
    assert!(app.streaming_raw.is_empty());
}

#[test]
fn mid_stream_error_retries_exhausted() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    app.streaming_raw = "partial text".into();
    app.mid_stream_retries = MAX_MID_STREAM_RETRIES;

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Error("connection dropped".into()),
    });

    assert!(
        app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] connection dropped")
    );
    assert_eq!(app.mid_stream_retries, MAX_MID_STREAM_RETRIES);
    assert!(app.streaming_block_id.is_none());
}

#[test]
fn mid_stream_error_401_not_retried() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    });
    app.streaming_raw = "partial text".into();
    app.mid_stream_retries = 0;

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Error("OpenRouter 401 unauthorized".into()),
    });

    assert!(app.blocks[0].body.to_text().contains("[ERROR]"));
    assert_eq!(app.mid_stream_retries, 0);
}

#[test]
fn local_provider_auth_error_is_not_retried_or_labeled_openrouter() {
    let (mut app, _) = App::new();
    Arc::make_mut(&mut app.conversation).clear();
    app.blocks.clear();
    app.streaming_block_id = Some(42);
    app.streaming_block_idx = Some(0);
    app.blocks.push(Block {
        id: 42,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
        view_mode: ViewMode::Raw,
        md_items: Vec::new(),
        model: Some("local-model".into()),
        apply_candidates: Vec::new(),
    });
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
    app.streaming_raw = "partial text".into();
    app.mid_stream_retries = 0;

    let _ = app.update(Message::ChatChunk {
        block_id: 42,
        event: ChatEvent::Error(
            "OpenAI-compatible provider 401 Unauthorized: token missing".into(),
        ),
    });

    assert_eq!(app.mid_stream_retries, 0);
    assert!(app.status.contains("인증 실패"), "status: {}", app.status);
    assert!(
        app.status.contains("local endpoint/token"),
        "status: {}",
        app.status
    );
    assert!(
        !app.status.contains("Open Settings"),
        "status: {}",
        app.status
    );
}
