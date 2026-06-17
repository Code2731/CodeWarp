use super::*;

#[test]
fn select_inference_engine_keeps_selection_within_local_namespace() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::SelectInferenceEngine(InferenceEngine::VLlm));
    assert_eq!(app.inference_selected_model, "Qwen--7B");
}

#[test]
fn select_inference_engine_clears_selection_across_namespaces() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::SelectInferenceEngine(InferenceEngine::TabbyMl));
    assert!(app.inference_selected_model.is_empty());
}
