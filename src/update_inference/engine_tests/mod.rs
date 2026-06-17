use super::*;

mod runtime;
mod select;
mod start;

#[test]
fn model_dir_changed_clears_stale_local_model_selection() {
    let old_dir = tempfile::TempDir::new().unwrap();
    let new_dir = tempfile::TempDir::new().unwrap();
    let model = old_dir.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = old_dir.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::ModelDirChanged(
        new_dir.path().display().to_string(),
    ));
    assert!(app.inference_selected_model.is_empty());
}

#[test]
fn model_dir_changed_keeps_selection_for_tabby_engine() {
    let new_dir = tempfile::TempDir::new().unwrap();
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyMl;
    app.inference_selected_model = "TabbyML/Qwen2.5-Coder-7B".into();

    let _ = app.update(Message::ModelDirChanged(
        new_dir.path().display().to_string(),
    ));
    assert_eq!(app.inference_selected_model, "TabbyML/Qwen2.5-Coder-7B");
}

#[test]
fn model_dir_picked_clears_stale_local_model_selection() {
    let old_dir = tempfile::TempDir::new().unwrap();
    let new_dir = tempfile::TempDir::new().unwrap();
    let model = old_dir.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::LlamaServer;
    app.model_dir_input = old_dir.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::ModelDirPicked(Some(new_dir.path().to_path_buf())));
    assert!(app.inference_selected_model.is_empty());
}

#[test]
fn model_dir_picked_none_keeps_selection() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::ModelDirPicked(None));
    assert_eq!(app.inference_selected_model, "Qwen--7B");
}
