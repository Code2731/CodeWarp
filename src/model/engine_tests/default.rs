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

// ── Tests migrated from types.rs inline mod tests ────────────────

#[test]
fn label_xllm() {
    assert_eq!(InferenceEngine::XLlm.label(), "xLLM");
}

#[test]
fn label_vllm() {
    assert_eq!(InferenceEngine::VLlm.label(), "vLLM");
}

#[test]
fn label_llama_server() {
    assert_eq!(InferenceEngine::LlamaServer.label(), "llama-server");
}

#[test]
fn label_tabby_ml() {
    assert_eq!(InferenceEngine::TabbyMl.label(), "TabbyML");
}

#[test]
fn label_tabby_api() {
    assert_eq!(InferenceEngine::TabbyApi.label(), "TabbyAPI (EXL2)");
}

#[test]
fn label_ollama() {
    assert_eq!(
        InferenceEngine::Ollama.label(),
        "Ollama (이미 떠있는 daemon)"
    );
}

#[test]
fn label_custom() {
    assert_eq!(InferenceEngine::Custom.label(), "Custom (직접 명령)");
}

#[test]
fn default_port_tabby_ml() {
    assert_eq!(InferenceEngine::TabbyMl.default_port(), 8080);
}

#[test]
fn default_port_tabby_api() {
    assert_eq!(InferenceEngine::TabbyApi.default_port(), 5000);
}

#[test]
fn default_port_ollama() {
    assert_eq!(InferenceEngine::Ollama.default_port(), 11434);
}

#[test]
fn default_port_fallback() {
    for eng in &[
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::Custom,
    ] {
        assert_eq!(eng.default_port(), 9000, "engine: {eng:?}");
    }
}

#[test]
fn namespace_same_variant() {
    for eng in InferenceEngine::ALL {
        assert!(eng.shares_model_namespace(*eng), "engine: {eng:?}");
    }
}

#[test]
fn namespace_xllm_vllm_llama_server_cross() {
    let group = [
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
    ];
    for a in &group {
        for b in &group {
            assert!(
                a.shares_model_namespace(*b),
                "{a:?} should share with {b:?}"
            );
        }
    }
}

#[test]
fn namespace_tabby_ml_only_self() {
    let others = [
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::TabbyApi,
        InferenceEngine::Ollama,
        InferenceEngine::Custom,
    ];
    for o in &others {
        assert!(
            !InferenceEngine::TabbyMl.shares_model_namespace(*o),
            "TabbyMl should not share with {o:?}"
        );
        assert!(
            !o.shares_model_namespace(InferenceEngine::TabbyMl),
            "{o:?} should not share with TabbyMl"
        );
    }
}

#[test]
fn namespace_tabby_api_only_self() {
    let others = [
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::TabbyMl,
        InferenceEngine::Ollama,
        InferenceEngine::Custom,
    ];
    for o in &others {
        assert!(
            !InferenceEngine::TabbyApi.shares_model_namespace(*o),
            "TabbyApi should not share with {o:?}"
        );
        assert!(
            !o.shares_model_namespace(InferenceEngine::TabbyApi),
            "{o:?} should not share with TabbyApi"
        );
    }
}

#[test]
fn namespace_ollama_only_self() {
    let others = [
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::TabbyMl,
        InferenceEngine::TabbyApi,
        InferenceEngine::Custom,
    ];
    for o in &others {
        assert!(
            !InferenceEngine::Ollama.shares_model_namespace(*o),
            "Ollama should not share with {o:?}"
        );
        assert!(
            !o.shares_model_namespace(InferenceEngine::Ollama),
            "{o:?} should not share with Ollama"
        );
    }
}

#[test]
fn namespace_custom_only_self() {
    let others = [
        InferenceEngine::XLlm,
        InferenceEngine::VLlm,
        InferenceEngine::LlamaServer,
        InferenceEngine::TabbyMl,
        InferenceEngine::TabbyApi,
        InferenceEngine::Ollama,
    ];
    for o in &others {
        assert!(
            !InferenceEngine::Custom.shares_model_namespace(*o),
            "Custom should not share with {o:?}"
        );
        assert!(
            !o.shares_model_namespace(InferenceEngine::Custom),
            "{o:?} should not share with Custom"
        );
    }
}

const TEST_MODEL: &str = "mistral-7b";
const TEST_PORT: u16 = 4321;

#[test]
fn compose_xllm() {
    let cmd = InferenceEngine::XLlm.compose_command(TEST_MODEL, TEST_PORT);
    assert_eq!(
        cmd,
        Some(vec![
            "xllm".into(),
            "serve".into(),
            "--model".into(),
            TEST_MODEL.into(),
            "--port".into(),
            "4321".into()
        ])
    );
}

#[test]
fn compose_vllm() {
    let cmd = InferenceEngine::VLlm.compose_command(TEST_MODEL, TEST_PORT);
    assert_eq!(
        cmd,
        Some(vec![
            "vllm".into(),
            "serve".into(),
            TEST_MODEL.into(),
            "--port".into(),
            "4321".into()
        ])
    );
}

#[test]
fn compose_llama_server() {
    let cmd = InferenceEngine::LlamaServer.compose_command(TEST_MODEL, TEST_PORT);
    assert_eq!(
        cmd,
        Some(vec![
            "llama-server".into(),
            "-m".into(),
            TEST_MODEL.into(),
            "--port".into(),
            "4321".into()
        ])
    );
}

#[test]
fn compose_tabby_ml() {
    let cmd = InferenceEngine::TabbyMl.compose_command(TEST_MODEL, TEST_PORT);
    assert_eq!(
        cmd,
        Some(vec![
            "tabby".into(),
            "serve".into(),
            "--model".into(),
            TEST_MODEL.into(),
            "--chat-model".into(),
            TEST_MODEL.into(),
        ])
    );
}

#[test]
fn compose_tabby_api() {
    let cmd = InferenceEngine::TabbyApi.compose_command(TEST_MODEL, TEST_PORT);
    #[cfg(windows)]
    assert_eq!(
        cmd,
        Some(vec![
            "Start.bat".into(),
            "--config".into(),
            "config.yml".into()
        ])
    );
    #[cfg(not(windows))]
    assert_eq!(
        cmd,
        Some(vec![
            "./start.sh".into(),
            "--config".into(),
            "config.yml".into()
        ])
    );
}

#[test]
fn compose_ollama() {
    assert_eq!(
        InferenceEngine::Ollama.compose_command(TEST_MODEL, TEST_PORT),
        None
    );
}

#[test]
fn compose_custom() {
    assert_eq!(
        InferenceEngine::Custom.compose_command(TEST_MODEL, TEST_PORT),
        None
    );
}
