use super::*;

#[test]
fn compare_mode_send_requires_registered_providers() {
    let (mut app, _) = App::new();
    app.ui.compare_both = true;
    app.input = "compare this".into();
    app.selected_model = None;
    app.model_options.clear();
    let before_blocks = app.blocks.len();

    let _ = app.update(Message::Send);

    assert!(
        app.status.contains("Compare 모드: OpenRouter 모델"),
        "got: {}",
        app.status
    );
    assert_eq!(app.blocks.len(), before_blocks);
}

#[test]
fn compare_completion_releases_abort_handle() {
    let (mut app, _) = App::new();
    let (_task, handle) = iced::Task::<Message>::none().abortable();
    app.abort_handle = Some(handle);
    app.ui.compare_pending = true;
    app.compare_block_ids = Some((1, 2));

    let _ = app.on_compare_responses_loaded(
        1,
        2,
        Ok("OpenRouter response".into()),
        Ok("local response".into()),
    );

    assert!(!app.ui.compare_pending);
    assert!(app.abort_handle.is_none());
    assert!(app.compare_block_ids.is_none());
}

#[test]
fn compare_completion_fills_response_blocks_by_id() {
    let (mut app, _) = App::new();
    app.blocks = vec![
        Block {
            id: 10,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(
                "OpenRouter 응답 대기 중…",
            )),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        },
        Block {
            id: 11,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(
                "Tabby 응답 대기 중…",
            )),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        },
    ];
    app.ui.compare_pending = true;
    app.compare_block_ids = Some((10, 11));

    let _ = app.on_compare_responses_loaded(
        10,
        11,
        Ok("OpenRouter response".into()),
        Ok("local response".into()),
    );

    assert_eq!(app.blocks[0].body.to_text(), "OpenRouter response");
    assert_eq!(app.blocks[1].body.to_text(), "local response");
}

#[test]
fn stale_compare_completion_is_ignored_for_new_request() {
    let (mut app, _) = App::new();
    app.ui.compare_pending = true;
    app.compare_block_ids = Some((10, 11));

    let _ = app.on_compare_responses_loaded(
        1,
        2,
        Ok("stale OpenRouter response".into()),
        Ok("stale local response".into()),
    );

    assert!(app.ui.compare_pending);
    assert_eq!(app.compare_block_ids, Some((10, 11)));
    assert!(app.conversation.is_empty());
}
