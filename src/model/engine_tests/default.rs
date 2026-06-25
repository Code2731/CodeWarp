use super::super::*;

#[test]
fn engine_default_ports() {
    assert_eq!(InferenceEngine::TabbyMl.default_port(), 8080);
    assert_eq!(
        InferenceEngine::TabbyApi.default_port(),
        TABBY_API_DEFAULT_PORT
    );
    assert_eq!(InferenceEngine::Ollama.default_port(), 11434);
    assert_eq!(InferenceEngine::XLlm.default_port(), 9000);
    assert_eq!(InferenceEngine::VLlm.default_port(), 9000);
    assert_eq!(InferenceEngine::LlamaServer.default_port(), 9000);
}

#[test]
fn engine_model_namespace_rules() {
    assert!(InferenceEngine::XLlm.shares_model_namespace(InferenceEngine::VLlm));
    assert!(InferenceEngine::VLlm.shares_model_namespace(InferenceEngine::LlamaServer));
    assert!(!InferenceEngine::TabbyMl.shares_model_namespace(InferenceEngine::XLlm));
    assert!(!InferenceEngine::Custom.shares_model_namespace(InferenceEngine::TabbyMl));
    assert!(!InferenceEngine::TabbyMl.shares_model_namespace(InferenceEngine::TabbyApi));
}

#[test]
fn engine_compose_xllm() {
    let cmd = InferenceEngine::XLlm
        .compose_command("C:\\models\\Qwen", 9000)
        .unwrap();
    assert_eq!(cmd[0], "xllm");
    assert_eq!(cmd[1], "serve");
    assert!(cmd.contains(&"--model".to_string()));
    assert!(cmd.contains(&"C:\\models\\Qwen".to_string()));
    assert!(cmd.contains(&"--port".to_string()));
    assert!(cmd.contains(&"9000".to_string()));
}

#[test]
fn engine_compose_vllm() {
    let cmd = InferenceEngine::VLlm
        .compose_command("/path/to/model", 9000)
        .unwrap();
    assert_eq!(cmd[0], "vllm");
    assert_eq!(cmd[1], "serve");
    assert!(cmd.contains(&"/path/to/model".to_string()));
}

#[test]
fn engine_compose_llama_server() {
    let cmd = InferenceEngine::LlamaServer
        .compose_command("/path/model.gguf", 9000)
        .unwrap();
    assert_eq!(cmd[0], "llama-server");
    assert!(cmd.contains(&"-m".to_string()));
    assert!(cmd.contains(&"/path/model.gguf".to_string()));
}

#[test]
fn engine_compose_tabby_uses_repo_id() {
    let cmd = InferenceEngine::TabbyMl
        .compose_command("TabbyML/Qwen2.5-Coder-7B", 8080)
        .unwrap();
    assert_eq!(cmd[0], "tabby");
    assert_eq!(cmd[1], "serve");
    assert!(cmd.contains(&"--chat-model".to_string()));
    assert!(cmd.contains(&"TabbyML/Qwen2.5-Coder-7B".to_string()));
}

#[test]
fn engine_compose_tabbyapi_uses_platform_launcher() {
    let cmd = InferenceEngine::TabbyApi
        .compose_command("C:\\models\\Local-EXL2", TABBY_API_DEFAULT_PORT)
        .unwrap();
    #[cfg(windows)]
    assert_eq!(cmd[0], "Start.bat");
    #[cfg(not(windows))]
    assert_eq!(cmd[0], "./start.sh");
    assert_eq!(cmd[1], "--config");
    assert_eq!(cmd[2], "config.yml");
}

#[test]
fn engine_ollama_no_spawn() {
    assert!(
        InferenceEngine::Ollama
            .compose_command("any", 11434)
            .is_none()
    );
}

#[test]
fn engine_custom_no_compose() {
    assert!(
        InferenceEngine::Custom
            .compose_command("any", 9000)
            .is_none()
    );
}
