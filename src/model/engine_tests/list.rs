use super::super::*;

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
fn list_models_empty_path_returns_empty() {
    assert!(list_downloaded_models(std::path::Path::new("")).is_empty());
}
