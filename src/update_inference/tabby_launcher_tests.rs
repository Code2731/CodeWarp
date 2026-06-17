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
