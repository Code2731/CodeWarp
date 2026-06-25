// model/tabbyapi/dir.rs — TabbyAPI model directory scanning helpers
use std::path::{Path, PathBuf};

pub(super) fn is_model_extension(name: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|ext| matches!(ext.to_str(), Some("bin" | "gguf" | "pt" | "pth")))
}

pub(super) fn has_model_weight_file(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if has_model_weight_file(&path) {
                return true;
            }
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let file_name = file_name.to_ascii_lowercase();
        if file_name.ends_with(".safetensors") || is_model_extension(&file_name) {
            return true;
        }
    }

    false
}

pub(super) fn is_valid_tabbyapi_model_dir_direct(path: &Path) -> bool {
    path.is_dir() && path.join("config.json").is_file() && has_model_weight_file(path)
}

pub(super) fn tabbyapi_direct_model_children(path: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(path) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_valid_tabbyapi_model_dir_direct(p))
        .collect()
}
