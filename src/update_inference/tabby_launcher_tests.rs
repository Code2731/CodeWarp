use super::*;

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
