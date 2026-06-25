use std::path::PathBuf;

fn favorites_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("favorites.json"))
}

pub(crate) fn read_favorites() -> Vec<String> {
    let Some(path) = favorites_path() else {
        return Vec::new();
    };
    let Ok(json) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub(crate) fn write_favorites(favs: &[String]) -> Result<(), String> {
    let path = favorites_path().ok_or("data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(favs).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn favorites_serde_roundtrip() {
        let favs = vec!["gpt-4o".to_string(), "claude-3.5".to_string()];
        let json = serde_json::to_string(&favs).unwrap();
        let loaded: Vec<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, favs);
    }
}
