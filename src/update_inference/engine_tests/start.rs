use super::*;

#[test]
fn can_start_inference_local_engine_requires_existing_model() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();

    assert!(app.can_start_inference());
}

#[test]
fn can_start_inference_local_engine_rejects_missing_model() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::VLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "missing-model".into();

    assert!(!app.can_start_inference());
}

#[test]
fn start_inference_local_engine_rejects_missing_binary_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();
    let missing_binary = tmp.path().join("missing-xllm.exe");

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();
    app.inference_binary_path = missing_binary.display().to_string();

    let _ = app.update(Message::StartInference);

    assert!(
        app.status.contains("xLLM binary was not found"),
        "got: {}",
        app.status
    );
    assert!(app.inference_pid.is_none());
}

#[test]
fn start_inference_local_engine_reports_missing_binary_inside_directory_override() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Qwen--7B");
    let runtime_dir = tmp.path().join("runtime-dir");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::create_dir_all(&runtime_dir).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();
    app.inference_binary_path = runtime_dir.display().to_string();

    let _ = app.update(Message::StartInference);

    #[cfg(windows)]
    let expected_binary = "xllm.exe";
    #[cfg(not(windows))]
    let expected_binary = "xllm";

    assert!(app.status.contains("is a directory"), "got: {}", app.status);
    assert!(app.status.contains(expected_binary), "got: {}", app.status);
    assert!(app.status.contains(&runtime_dir.display().to_string()));
    assert!(app.inference_pid.is_none());
}

#[test]
fn can_start_inference_tabby_requires_model_id() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyMl;
    app.inference_selected_model = String::new();
    assert!(!app.can_start_inference());

    app.inference_selected_model = "TabbyML/Qwen2.5-Coder-7B".into();
    assert!(app.can_start_inference());
}

#[test]
fn start_inference_tabby_rejects_local_exl2_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Local-EXL2");
    std::fs::create_dir_all(&model).unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyMl;
    app.inference_selected_model = model.display().to_string();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("TabbyAPI"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn can_start_inference_custom_requires_non_empty_command() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::Custom;
    app.inference_command_input = "   ".into();
    assert!(!app.can_start_inference());

    app.inference_command_input = "xllm serve --model X".into();
    assert!(app.can_start_inference());
}

#[test]
fn can_start_inference_ollama_always_true() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::Ollama;
    app.inference_selected_model = String::new();
    app.inference_command_input = String::new();
    assert!(app.can_start_inference());
}
