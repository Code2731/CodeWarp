use super::*;

#[test]
fn compare_mode_send_requires_registered_providers() {
    let (mut app, _) = App::new();
    app.compare_both = true;
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
