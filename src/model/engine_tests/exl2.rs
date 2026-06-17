use super::super::*;

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
