use super::*;

#[test]
fn ko_friendly_known_models() {
    assert!(is_korean_friendly("openai/gpt-4o"));
    assert!(is_korean_friendly("anthropic/claude-3.5-sonnet"));
    assert!(is_korean_friendly("google/gemini-1.5-pro"));
    assert!(is_korean_friendly("qwen/qwen2.5-coder-7b"));
    assert!(is_korean_friendly("meta-llama/llama-3.1-70b-instruct"));
    assert!(is_korean_friendly("upstage/solar-10.7b"));
    assert!(is_korean_friendly("LGAI-EXAONE/EXAONE-3.5-7.8B"));
    assert!(is_korean_friendly("deepseek/deepseek-v3"));
}

#[test]
fn ko_friendly_negative() {
    assert!(!is_korean_friendly("mistralai/mistral-7b"));
    assert!(!is_korean_friendly("openai/gpt-3.5-turbo"));
    assert!(!is_korean_friendly("starcoder2:7b"));
}

#[test]
fn categorize_coding_models() {
    let cats = categorize_model("qwen/qwen2.5-coder-7b");
    assert!(cats.contains(&ModelCategory::Coding));
}

#[test]
fn categorize_reasoning_models() {
    let cats = categorize_model("deepseek/deepseek-r1");
    assert!(cats.contains(&ModelCategory::Reasoning));
}

#[test]
fn categorize_general_fallback() {
    let cats = categorize_model("mistralai/mistral-7b-instruct");
    assert!(cats.contains(&ModelCategory::General));
}

#[test]
fn parse_price_per_million_typical() {
    let p = parse_price_per_million(Some("0.000005"));
    assert!(matches!(p, Some(v) if (v - 5.0).abs() < 1e-9));
}

#[test]
fn parse_price_per_million_free() {
    let p = parse_price_per_million(Some("0"));
    assert_eq!(p, Some(0.0));
}

#[test]
fn parse_price_per_million_invalid() {
    assert_eq!(parse_price_per_million(None), None);
    assert_eq!(parse_price_per_million(Some("")), None);
    assert_eq!(parse_price_per_million(Some("abc")), None);
}

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
fn display_openrouter_basic() {
    let m = or_opt("gpt-4o");
    let s = format!("{}", m);
    assert!(s.starts_with("[OR]"), "got: {}", s);
    assert!(s.contains("gpt-4o"));
}

#[test]
fn display_openai_compat_with_label() {
    let m = oai_opt("qwen2.5-coder", "xLLM");
    let s = format!("{}", m);
    assert!(s.starts_with("[xLLM]"), "got: {}", s);
    assert!(s.contains("qwen2.5-coder"));
}

#[test]
fn display_openai_compat_empty_label_defaults_to_local() {
    let m = oai_opt("starcoder", "");
    let s = format!("{}", m);
    assert!(s.starts_with("[Local]"), "got: {}", s);
}

#[test]
fn display_openai_compat_whitespace_label_defaults() {
    let m = oai_opt("foo", "   ");
    let s = format!("{}", m);
    assert!(s.starts_with("[Local]"), "got: {}", s);
}

#[test]
fn display_combined_tags() {
    let mut m = or_opt("claude-3.5-sonnet");
    m.ko_friendly = true;
    m.favorite = true;
    m.context_length = Some(200_000);
    m.prompt_per_million = Some(3.0);
    m.completion_per_million = Some(15.0);
    let s = format!("{}", m);
    assert!(s.contains("[OR]"));
    assert!(s.contains("[KO]"));
    assert!(s.contains("★"));
    assert!(s.contains("200k"));
    assert!(s.contains("$3.00/$15.00"));
}

#[test]
fn display_openai_compat_free_marker() {
    let mut m = oai_opt("local-model", "xLLM");
    m.prompt_per_million = Some(0.0);
    m.completion_per_million = Some(0.0);
    let s = format!("{}", m);
    assert!(s.contains("[xLLM]"));
    assert!(s.contains("free"));
}

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
    assert!(InferenceEngine::Ollama
        .compose_command("any", 11434)
        .is_none());
}

#[test]
fn engine_custom_no_compose() {
    assert!(InferenceEngine::Custom
        .compose_command("any", 9000)
        .is_none());
}

#[test]
fn list_models_empty_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    assert!(list_downloaded_models(tmp.path()).is_empty());
}

#[test]
fn list_models_returns_subdirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let qwen = tmp.path().join("Qwen--Qwen2.5-Coder-7B");
    std::fs::create_dir_all(&qwen).unwrap();
    std::fs::write(qwen.join("config.json"), "{}").unwrap();
    std::fs::write(qwen.join("model.safetensors"), "x").unwrap();
    let solar = tmp.path().join("upstage--SOLAR-10.7B");
    std::fs::create_dir_all(&solar).unwrap();
    std::fs::write(solar.join("model.safetensors"), "x").unwrap();
    std::fs::write(tmp.path().join("ignore.txt"), "x").unwrap();
    let mut models = list_downloaded_models(tmp.path());
    models.sort();
    assert_eq!(models.len(), 2);
    assert!(models[0].contains("Qwen") || models[1].contains("Qwen"));
}

#[test]
fn list_models_are_sorted() {
    let tmp = tempfile::TempDir::new().unwrap();
    let zulu = tmp.path().join("zulu-model");
    let alpha = tmp.path().join("alpha-model");
    std::fs::create_dir_all(&zulu).unwrap();
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(zulu.join("model.safetensors"), "x").unwrap();
    std::fs::write(alpha.join("model.safetensors"), "x").unwrap();

    let models = list_downloaded_models(tmp.path());
    assert_eq!(
        models,
        vec!["alpha-model".to_string(), "zulu-model".to_string()]
    );
}

#[test]
fn list_models_skips_empty_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("empty")).unwrap();
    assert!(list_downloaded_models(tmp.path()).is_empty());
}

#[test]
fn list_models_skips_metadata_only_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("README.md"), "x").unwrap();
    std::fs::write(model.join(".gitattributes"), "x").unwrap();

    assert!(list_downloaded_models(tmp.path()).is_empty());
    assert!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .is_none()
    );
}

#[test]
fn list_models_empty_path_returns_empty() {
    assert!(list_downloaded_models(std::path::Path::new("")).is_empty());
}

#[test]
fn downloaded_exl2_preset_folder_accepts_same_model_bpw_variant() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-4.0bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_accepts_nested_model_layout() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    let nested = root.join("weights");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("config.json"), "{}").unwrap();
    std::fs::write(nested.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-4.0bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_accepts_root_with_multiple_nested_variants() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
    let child_35 = root.join("3.5bpw");
    let child_40 = root.join("4.0bpw");
    std::fs::create_dir_all(&child_35).unwrap();
    std::fs::create_dir_all(&child_40).unwrap();
    std::fs::write(child_35.join("config.json"), "{}").unwrap();
    std::fs::write(child_35.join("model.safetensors"), "x").unwrap();
    std::fs::write(child_40.join("config.json"), "{}").unwrap();
    std::fs::write(child_40.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-3.5bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_exact_match_wins() {
    let tmp = tempfile::TempDir::new().unwrap();
    let exact = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
    let other = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    std::fs::create_dir_all(&exact).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(exact.join("config.json"), "{}").unwrap();
    std::fs::write(exact.join("model.safetensors"), "x").unwrap();
    std::fs::write(other.join("config.json"), "{}").unwrap();
    std::fs::write(other.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-3.5bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_avoids_ambiguous_variants() {
    let tmp = tempfile::TempDir::new().unwrap();
    let a = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    let b = tmp.path().join("Llama-3.2-3B-Instruct-5.0bpw");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("config.json"), "{}").unwrap();
    std::fs::write(a.join("model.safetensors"), "x").unwrap();
    std::fs::write(b.join("config.json"), "{}").unwrap();
    std::fs::write(b.join("model.safetensors"), "x").unwrap();

    assert!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .is_none()
    );
}

#[test]
fn resolve_tabbyapi_model_dir_accepts_single_nested_child() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Qwen2.5-Coder-7B-Instruct-exl2-4.0bpw");
    let nested = root.join("model");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("config.json"), "{}").unwrap();
    std::fs::write(nested.join("model.safetensors"), "x").unwrap();

    let resolved = resolve_tabbyapi_model_dir(&root).expect("expected nested model dir");
    assert_eq!(resolved, nested);
}

#[test]
fn resolve_tabbyapi_model_dir_for_folder_prefers_matching_bpw_child() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
    let child_35 = root.join("3.5bpw");
    let child_40 = root.join("4.0bpw");
    std::fs::create_dir_all(&child_35).unwrap();
    std::fs::create_dir_all(&child_40).unwrap();
    std::fs::write(child_35.join("config.json"), "{}").unwrap();
    std::fs::write(child_35.join("model.safetensors"), "x").unwrap();
    std::fs::write(child_40.join("config.json"), "{}").unwrap();
    std::fs::write(child_40.join("model.safetensors"), "x").unwrap();

    let resolved = resolve_tabbyapi_model_dir_for_folder(&root, "Llama-3.2-3B-Instruct-3.5bpw")
        .expect("expected bpw-matched nested model dir");
    assert_eq!(resolved, child_35);
}
