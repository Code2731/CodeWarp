use super::*;

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
