use std::path::PathBuf;

fn codewarp_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp"))
}

fn clean_shutdown_path() -> Option<PathBuf> {
    codewarp_dir().map(|d| d.join(".clean_shutdown"))
}

pub fn mark_clean_shutdown() {
    if let Some(path) = clean_shutdown_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, "");
    }
}

pub fn was_clean_shutdown() -> bool {
    let Some(path) = clean_shutdown_path() else {
        return true;
    };
    let exists = path.exists();
    if exists {
        let _ = std::fs::remove_file(&path);
    }
    exists
}
