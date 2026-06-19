use super::humanize_inference_spawn_error;

#[test]
fn humanize_inference_spawn_error_explains_missing_xllm_binary() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("xllm", &err);
    assert!(msg.contains("xllm"), "got: {}", msg);
    assert!(msg.to_ascii_lowercase().contains("path"), "got: {}", msg);
    assert!(
        msg.to_ascii_lowercase().contains("binary path"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_falls_back_for_other_errors() {
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let msg = humanize_inference_spawn_error("xllm", &err);
    assert!(msg.starts_with("xllm: "), "got: {}", msg);
}

#[test]
fn humanize_inference_spawn_error_handles_tabby_cmd_alias() {
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access is denied");
    let msg = humanize_inference_spawn_error("tabby.cmd", &err);
    assert!(
        msg.contains("Tabby executable could not be started"),
        "got: {}",
        msg
    );
    assert!(msg.contains("tabby.cmd"), "got: {}", msg);
}

#[test]
fn humanize_inference_spawn_error_vllm_not_found() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("vllm", &err);
    assert!(msg.contains("vllm"), "got: {}", msg);
    assert!(
        msg.to_ascii_lowercase().contains("binary path"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_llama_server_not_found() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("llama-server", &err);
    assert!(msg.contains("llama-server"), "got: {}", msg);
    assert!(
        msg.to_ascii_lowercase().contains("binary path"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_tabby_not_found_falls_back() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("tabby.exe", &err);
    assert!(msg.starts_with("tabby.exe:"), "got: {}", msg);
}

#[test]
fn humanize_inference_spawn_error_tabby_korean_access_denied() {
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "액세스가 거부됨");
    let msg = humanize_inference_spawn_error("tabby.bat", &err);
    assert!(
        msg.contains("Tabby executable could not be started"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_generic_fallback() {
    let err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
    let msg = humanize_inference_spawn_error("my-tool", &err);
    assert_eq!(msg, "my-tool: connection refused");
}
