use super::*;
use crate::Message;

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

fn oai_opt(id: &str, label: &str) -> ModelOption {
    ModelOption {
        id: id.into(),
        provider: LlmProvider::OpenAICompat,
        provider_label: label.into(),
        ko_friendly: false,
        favorite: false,
        context_length: None,
        prompt_per_million: None,
        completion_per_million: None,
    }
}

#[test]
fn local_openai_compat_models_do_not_send_tool_definitions() {
    let (mut app, _) = App::new();
    app.model_options = vec![ModelOption {
        id: "local-model".into(),
        provider: LlmProvider::OpenAICompat,
        provider_label: "TabbyAPI".into(),
        ko_friendly: false,
        favorite: false,
        context_length: None,
        prompt_per_million: Some(0.0),
        completion_per_million: Some(0.0),
    }];
    app.selected_model = Some("local-model".into());

    assert!(app.tool_definitions_for_selected_model().is_none());
}

#[test]
fn selected_model_with_same_id_uses_explicit_provider_choice() {
    let (mut app, _) = App::new();
    app.model_options = vec![or_opt("shared-model"), oai_opt("shared-model", "TabbyAPI")];
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::SelectModel(oai_opt("shared-model", "TabbyAPI")));
    assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenAICompat));
    assert!(app.tool_definitions_for_selected_model().is_none());

    let _ = app.update(Message::SelectModel(or_opt("shared-model")));
    assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenRouter));
    assert!(app.tool_definitions_for_selected_model().is_some());
}

#[test]
fn resolve_provider_prefers_current_tabby_inputs_over_keystore() {
    let (mut app, _) = App::new();
    app.model_options = vec![oai_opt("local-model", "TabbyAPI")];
    app.selected_model = Some("local-model".into());
    app.selected_model_provider = Some(LlmProvider::OpenAICompat);
    app.tabby_url_input = "http://localhost:5001".into();
    app.tabby_token_input = "live-token".into();

    let (base_url, api_key) = app.resolve_provider().expect("provider resolves");

    assert_eq!(base_url, "http://localhost:5001/v1");
    assert_eq!(api_key.as_deref(), Some("live-token"));
}
