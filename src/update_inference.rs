// update_inference.rs — Inference-related tests (main.rs child module)
#[cfg(test)]
use super::*;

#[cfg(test)]
mod tests {
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
        assert!(app
            .model_options
            .iter()
            .any(|o| { o.id == "tabby-a" && o.provider == LlmProvider::OpenAICompat }));
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
        assert!(app
            .model_options
            .iter()
            .any(|o| { o.id == "tabby-a" && o.provider == LlmProvider::OpenAICompat }));
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

    #[test]
    fn tabbyapi_start_button_can_show_missing_launcher_error() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_selected_model = r"C:\models\Local-EXL2".into();
        app.inference_binary_path.clear();

        assert!(!app.can_start_inference());
        assert!(app.can_attempt_start_inference());

        let _ = app.update(Message::StartInference);

        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(
            app.status.contains("TabbyAPI 런타임"),
            "got: {}",
            app.status
        );
        assert!(app.status.contains("먼저 설치"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_start_rejects_tabbyml_binary_with_specific_guidance() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_selected_model = r"C:\models\Local-EXL2".into();
        app.inference_binary_path = r"C:\tools\tabby.exe".into();

        let _ = app.update(Message::StartInference);

        assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
        assert!(app.status.contains("EXL2"), "got: {}", app.status);
        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_binary_picker_rejects_tabby_cli_cmd() {
        let tmp = tempfile::TempDir::new().unwrap();
        let picked = tmp.path().join("tabby.cmd");
        std::fs::write(&picked, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path.clear();

        let _ = app.update(Message::InferenceBinaryPicked(Some(picked)));

        assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
        assert!(app.inference_binary_path.is_empty());
    }

    #[test]
    fn tabbyapi_binary_picker_rejects_tabby_cli_bat() {
        let tmp = tempfile::TempDir::new().unwrap();
        let picked = tmp.path().join("tabby.bat");
        std::fs::write(&picked, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path.clear();

        let _ = app.update(Message::InferenceBinaryPicked(Some(picked)));

        assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
        assert!(app.status.contains("tabby.bat"), "got: {}", app.status);
        assert!(app.inference_binary_path.is_empty());
    }

    #[test]
    fn tabbyapi_binary_picker_rejects_wrong_script_name() {
        let tmp = tempfile::TempDir::new().unwrap();
        let picked = tmp.path().join("launcher.bat");
        std::fs::write(&picked, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path.clear();

        let _ = app.update(Message::InferenceBinaryPicked(Some(picked)));

        assert!(
            app.status.contains("파일명이 올바르지"),
            "got: {}",
            app.status
        );
        assert!(app.inference_binary_path.is_empty());
    }

    #[test]
    fn tabbyapi_binary_picker_accepts_start_bat() {
        let tmp = tempfile::TempDir::new().unwrap();
        let picked = tmp.path().join("Start.bat");
        std::fs::write(&picked, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path.clear();

        let _ = app.update(Message::InferenceBinaryPicked(Some(picked.clone())));

        assert_eq!(app.inference_binary_path, picked.display().to_string());
        assert!(
            app.status.contains("script 경로 저장됨"),
            "got: {}",
            app.status
        );
    }

    #[cfg(windows)]
    #[test]
    fn tabbyapi_binary_picker_accepts_start_cmd() {
        let tmp = tempfile::TempDir::new().unwrap();
        let picked = tmp.path().join("Start.cmd");
        std::fs::write(&picked, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path.clear();

        let _ = app.update(Message::InferenceBinaryPicked(Some(picked.clone())));

        assert_eq!(app.inference_binary_path, picked.display().to_string());
        assert!(
            app.status.contains("script 경로 저장됨"),
            "got: {}",
            app.status
        );
    }

    #[test]
    fn tabbyapi_start_rejects_tabbyml_cli_without_extension() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_selected_model = r"C:\models\Local-EXL2".into();
        app.inference_binary_path = r"C:\tools\tabby".into();

        let _ = app.update(Message::StartInference);

        assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
        assert!(app.status.contains("EXL2"), "got: {}", app.status);
        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_start_rejects_tabbyml_cli_cmd_launcher() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_selected_model = r"C:\models\Local-EXL2".into();
        app.inference_binary_path = r"C:\tools\tabby.cmd".into();

        let _ = app.update(Message::StartInference);

        assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
        assert!(app.status.contains("tabby.cmd"), "got: {}", app.status);
        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_start_rejects_tabbyml_cli_bat_launcher() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_selected_model = r"C:\models\Local-EXL2".into();
        app.inference_binary_path = r"C:\tools\tabby.bat".into();

        let _ = app.update(Message::StartInference);

        assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
        assert!(app.status.contains("tabby.bat"), "got: {}", app.status);
        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_start_rejects_missing_launcher_file_with_explicit_message() {
        let tmp = tempfile::TempDir::new().unwrap();
        let missing = tmp.path().join("Start.bat");

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path = missing.display().to_string();
        app.inference_selected_model.clear();

        let _ = app.update(Message::StartInference);

        assert!(
            app.status.contains("찾을 수 없습니다"),
            "got: {}",
            app.status
        );
        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_start_rejects_launcher_directory_path() {
        let tmp = tempfile::TempDir::new().unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path = tmp.path().display().to_string();
        app.inference_selected_model.clear();

        let _ = app.update(Message::StartInference);

        assert!(app.status.contains("폴더입니다"), "got: {}", app.status);
        assert!(app.status.contains("Start.bat"), "got: {}", app.status);
        assert!(app.inference_pid.is_none());
    }

    #[test]
    fn tabbyapi_can_start_with_launcher_without_model_path() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_selected_model.clear();
        app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();

        assert!(app.can_start_inference());
        assert!(app.can_attempt_start_inference());
    }

    #[test]
    fn tabbyapi_connection_error_prompts_for_launcher_when_missing() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.tabby_url_input = "http://localhost:5000".into();
        app.inference_binary_path.clear();

        let msg = app.compose_tabby_connection_error("operation timed out");

        assert!(msg.contains("TabbyAPI 서버"), "got: {}", msg);
        assert!(msg.contains("Start.bat"), "got: {}", msg);
        assert!(msg.contains("start.sh"), "got: {}", msg);
        assert!(msg.contains("main.py"), "got: {}", msg);
    }

    #[test]
    fn tabbyapi_connection_error_points_to_runtime_logs_when_launcher_is_set() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.tabby_url_input = "http://localhost:5000".into();
        app.inference_port_input = "5000".into();
        app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();

        let msg = app.compose_tabby_connection_error("error sending request: Connection refused");

        assert!(msg.contains("TabbyAPI 서버"), "got: {}", msg);
        assert!(msg.contains("로그"), "got: {}", msg);
        assert!(msg.contains("http://localhost:5000"), "got: {}", msg);
    }

    #[test]
    fn tabbyapi_connection_error_detects_runtime_port_mismatch() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.tabby_url_input = "http://localhost:8080".into();
        app.inference_port_input = "5000".into();
        app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();

        let msg = app.compose_tabby_connection_error("operation timed out");

        assert!(msg.contains("Provider URL"), "got: {}", msg);
        assert!(msg.contains("5000"), "got: {}", msg);
        assert!(msg.contains("http://localhost:5000"), "got: {}", msg);
    }

    #[test]
    fn tabby_models_loaded_error_decrements_auto_retry_while_runtime_alive() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_pid = Some(42);
        app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();
        app.tabby_url_input = "http://localhost:5000".into();
        app.tabby_connect_retry_left = 2;

        let _ = app.update(Message::TabbyModelsLoaded(
            Err("operation timed out".into()),
        ));

        assert_eq!(app.tabby_connect_retry_left, 1);
        assert!(app.status.contains("자동 재시도"), "got: {}", app.status);
        app.inference_pid = None;
    }

    #[test]
    fn tabby_models_loaded_error_without_retry_budget_reports_failure() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_pid = Some(42);
        app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();
        app.tabby_url_input = "http://localhost:5000".into();
        app.tabby_connect_retry_left = 0;

        let _ = app.update(Message::TabbyModelsLoaded(
            Err("operation timed out".into()),
        ));

        assert_eq!(app.tabby_connect_retry_left, 0);
        assert!(app.status.contains("연결 실패"), "got: {}", app.status);
        app.inference_pid = None;
    }

    #[test]
    fn tabbyapi_bat_launcher_runs_via_cmd_in_script_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let script = tmp.path().join("Start.bat");
        std::fs::write(&script, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::TabbyApi;
        app.inference_binary_path = script.display().to_string();

        let (program, args, work_dir) = app.resolve_runtime_spawn_command(
            "Start.bat".into(),
            vec!["--config".into(), "config.yml".into()],
        );

        assert_eq!(program, "cmd.exe");
        assert_eq!(
            args,
            vec![
                "/C".to_string(),
                "Start.bat".to_string(),
                "--config".to_string(),
                "config.yml".to_string()
            ]
        );
        assert_eq!(work_dir.as_deref(), Some(tmp.path()));
    }

    #[test]
    fn non_tabby_runtime_ignores_tabbyapi_launcher_override() {
        let tmp = tempfile::TempDir::new().unwrap();
        let tabby_dir = tmp.path().join("tabbyAPI");
        std::fs::create_dir_all(&tabby_dir).unwrap();
        let launcher = tabby_dir.join("Start.bat");
        std::fs::write(&launcher, "@echo off").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::XLlm;
        app.inference_binary_path = launcher.display().to_string();

        let (program, args, work_dir) =
            app.resolve_runtime_spawn_command("xllm".into(), vec!["serve".into()]);

        assert_eq!(program, "xllm");
        assert_eq!(args, vec!["serve".to_string()]);
        assert!(work_dir.is_none());
    }

    #[test]
    fn non_tabby_runtime_keeps_custom_binary_override() {
        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::XLlm;
        app.inference_binary_path = r"C:\tools\xllm.exe".into();

        let (program, args, work_dir) =
            app.resolve_runtime_spawn_command("xllm".into(), vec!["serve".into()]);

        assert_eq!(program, r"C:\tools\xllm.exe");
        assert_eq!(args, vec!["serve".to_string()]);
        assert!(work_dir.is_none());
    }

    #[test]
    fn non_tabby_runtime_directory_override_resolves_engine_binary() {
        let tmp = tempfile::TempDir::new().unwrap();
        let runtime_dir = tmp.path().join("runtime");
        std::fs::create_dir_all(&runtime_dir).unwrap();
        #[cfg(windows)]
        let bin = runtime_dir.join("xllm.exe");
        #[cfg(not(windows))]
        let bin = runtime_dir.join("xllm");
        std::fs::write(&bin, "bin").unwrap();

        let (mut app, _) = App::new();
        app.inference_engine = InferenceEngine::XLlm;
        app.inference_binary_path = runtime_dir.display().to_string();

        let (program, args, work_dir) =
            app.resolve_runtime_spawn_command("xllm".into(), vec!["serve".into()]);

        assert_eq!(program, bin.display().to_string());
        assert_eq!(args, vec!["serve".to_string()]);
        assert!(work_dir.is_none());
    }

    #[test]
    fn tabbyapi_config_points_to_selected_model_and_local_port() {
        let runtime = tempfile::TempDir::new().unwrap();
        let launcher = runtime.path().join("start.bat");
        std::fs::write(&launcher, "@echo off").unwrap();
        let models = tempfile::TempDir::new().unwrap();
        let model = models.path().join("Local-EXL2");
        std::fs::create_dir_all(&model).unwrap();

        let config = write_tabbyapi_config_for_launcher(
            &launcher.display().to_string(),
            &model.display().to_string(),
            TABBY_API_DEFAULT_PORT,
        )
        .unwrap();
        let text = std::fs::read_to_string(config).unwrap();

        assert!(text.contains("port: 5000"), "got: {}", text);
        assert!(text.contains("disable_auth: true"), "got: {}", text);
        assert!(text.contains("model_name: 'Local-EXL2'"), "got: {}", text);
        assert!(
            text.contains(&format!("model_dir: '{}'", models.path().display())),
            "got: {}",
            text
        );
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
}
