use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ModelUsage {
    pub total_cost: f64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub call_count: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct UsageStore {
    pub by_model: BTreeMap<String, ModelUsage>,
}

fn usage_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("usage.json"))
}

pub(crate) fn load_usage() -> UsageStore {
    let Some(path) = usage_path() else {
        return UsageStore::default();
    };
    let Ok(json) = std::fs::read_to_string(&path) else {
        return UsageStore::default();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub(crate) fn save_usage(usage: &UsageStore) -> Result<(), String> {
    let path = usage_path().ok_or_else(|| "data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(usage).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_serde_roundtrip() {
        let mut usage = UsageStore::default();
        usage.by_model.insert(
            "gpt-4o".into(),
            ModelUsage {
                total_cost: 1.23,
                prompt_tokens: 500,
                completion_tokens: 300,
                call_count: 10,
            },
        );
        usage.by_model.insert(
            "claude-3.5".into(),
            ModelUsage {
                total_cost: 0.45,
                prompt_tokens: 200,
                completion_tokens: 100,
                call_count: 5,
            },
        );

        let json = serde_json::to_string(&usage).unwrap();
        let loaded: UsageStore = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.by_model.len(), 2);
        assert!((loaded.by_model["gpt-4o"].total_cost - 1.23).abs() < 1e-10);
        assert_eq!(loaded.by_model["claude-3.5"].call_count, 5);
    }

    #[test]
    fn usage_default_is_empty() {
        let usage = UsageStore::default();
        assert!(usage.by_model.is_empty());
    }
}
