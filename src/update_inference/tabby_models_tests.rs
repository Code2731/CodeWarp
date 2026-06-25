use super::*;

fn or_opt(id: &str) -> ModelOption {
    ModelOption {
        id: id.into(),
        provider: LlmProvider::OpenRouter,
        provider_label: String::new(),
        ko_friendly: false,
        favorite: false,
        context_length: None,
        prompt_per_million: None,
        completion_per_million: None,
    }
}

#[test]
fn select_downloaded_model_defaults_to_tabbyapi_port() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut app, _) = App::new();
    app.model_dir_input = tmp.path().display().to_string();
    let model = tmp.path().join("Local-EXL2");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let _ = app.update(Message::SelectDownloadedModel("Local-EXL2".into()));

    assert_eq!(app.inference_engine, InferenceEngine::TabbyApi);
    assert_eq!(app.inference_port_input, TABBY_API_DEFAULT_PORT.to_string());
    assert_eq!(app.tabby_url_input, "http://localhost:5000");
    assert!(app.inference_selected_model.ends_with("Local-EXL2"));
    if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir()) {
        assert_eq!(app.inference_binary_path, launcher.display().to_string());
        assert!(app.can_start_inference());
    } else {
        assert!(app.inference_binary_path.is_empty());
        assert!(!app.can_start_inference());
    }
    assert!(app.can_attempt_start_inference());
}

#[cfg(windows)]
#[test]
fn find_tabbyapi_launcher_accepts_start_cmd() {
    let tmp = tempfile::TempDir::new().unwrap();
    let launcher = tmp.path().join("Start.cmd");
    std::fs::write(&launcher, "@echo off").unwrap();

    let found = find_tabbyapi_launcher(tmp.path());
    let found = found.expect("expected launcher");
    assert_eq!(found.parent(), Some(tmp.path()));
    let name = found
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    assert!(name.eq_ignore_ascii_case("start.cmd"), "got: {}", name);
}

#[test]
fn selecting_tabbyapi_runtime_sets_provider_endpoint() {
    let (mut app, _) = App::new();
    app.tabby_url_input = "http://localhost:8080".into();
    app.openai_compat_label = "TabbyML".into();

    let _ = app.update(Message::SelectInferenceEngine(InferenceEngine::TabbyApi));

    assert_eq!(app.inference_port_input, TABBY_API_DEFAULT_PORT.to_string());
    assert_eq!(app.tabby_url_input, "http://localhost:5000");
    assert_eq!(app.openai_compat_label, "TabbyAPI");
}

#[test]
fn tabbyapi_port_change_syncs_loopback_provider_url() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_port_input = "5000".into();
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::InferencePortChanged("5001".into()));

    assert_eq!(app.inference_port_input, "5001");
    assert_eq!(app.tabby_url_input, "http://localhost:5001");
}

#[test]
fn tabbyapi_port_change_does_not_override_non_loopback_provider_url() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_port_input = "5000".into();
    app.tabby_url_input = "http://192.168.0.20:5000".into();

    let _ = app.update(Message::InferencePortChanged("5001".into()));

    assert_eq!(app.inference_port_input, "5001");
    assert_eq!(app.tabby_url_input, "http://192.168.0.20:5000");
}

#[test]
fn saved_shared_model_prefers_tabby_when_tabby_url_is_set() {
    let (mut app, _) = App::new();
    app.selected_model = Some("shared-model".into());
    app.selected_model_provider = None;
    app.tabby_url_input = "http://localhost:5000".into();
    app.model_options = vec![or_opt("shared-model")];

    let _ = app.update(Message::TabbyModelsLoaded(Ok(vec!["shared-model".into()])));

    assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenAICompat));
}

#[test]
fn tabby_models_loaded_selects_first_local_model() {
    let (mut app, _) = App::new();
    app.model_options.clear();
    app.selected_model = Some("openrouter-model".into());
    app.openai_compat_label = "TabbyAPI".into();

    let _ = app.update(Message::TabbyModelsLoaded(Ok(vec![
        "tabby-a".into(),
        "tabby-b".into(),
    ])));

    assert_eq!(app.selected_model.as_deref(), Some("tabby-a"));
    assert!(
        app.model_options
            .iter()
            .any(|o| { o.id == "tabby-a" && o.provider == LlmProvider::OpenAICompat })
    );
}

#[test]
fn openrouter_models_loaded_preserves_existing_tabby_selection() {
    let (mut app, _) = App::new();
    app.model_options = vec![ModelOption {
        id: "tabby-a".into(),
        provider: LlmProvider::OpenAICompat,
        provider_label: "TabbyAPI".into(),
        ko_friendly: false,
        favorite: false,
        context_length: None,
        prompt_per_million: Some(0.0),
        completion_per_million: Some(0.0),
    }];
    app.selected_model = Some("tabby-a".into());
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::ModelsLoaded(Ok(vec![OpenRouterModel {
        id: "openrouter-a".into(),
        name: None,
        context_length: None,
        pricing: None,
    }])));

    assert_eq!(app.selected_model.as_deref(), Some("tabby-a"));
    assert!(
        app.model_options
            .iter()
            .any(|o| { o.id == "tabby-a" && o.provider == LlmProvider::OpenAICompat })
    );
}

#[test]
fn openrouter_models_loaded_waits_for_tabby_when_saved_selection_not_loaded_yet() {
    let (mut app, _) = App::new();
    app.model_options.clear();
    app.selected_model = Some("tabby-a".into());
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::ModelsLoaded(Ok(vec![OpenRouterModel {
        id: "openrouter-a".into(),
        name: None,
        context_length: None,
        pricing: None,
    }])));

    assert_eq!(app.selected_model.as_deref(), Some("tabby-a"));
}
