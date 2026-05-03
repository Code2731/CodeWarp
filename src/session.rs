// 채팅 세션 영구 저장 (단일 세션, 자동 복원)
// Windows: %LOCALAPPDATA%\codewarp\session.json
// macOS:   ~/Library/Application Support/codewarp/session.json
// Linux:   ~/.local/share/codewarp/session.json

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::openrouter::ChatMessage;

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedSession {
    pub conversation: Vec<ChatMessage>,
    pub blocks: Vec<PersistedBlock>,
    pub next_block_id: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedBlock {
    pub id: u64,
    pub role: String, // "user" | "assistant"
    pub content: String,
}

pub fn session_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("session.json"))
}

pub fn save(session: &PersistedSession) -> Result<(), String> {
    let path = session_path().ok_or_else(|| "data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

pub fn load() -> Option<PersistedSession> {
    let path = session_path()?;
    let json = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

#[allow(dead_code)]
pub fn clear() -> Result<(), String> {
    if let Some(path) = session_path() {
        if path.exists() {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

// ── 즐겨찾기 모델 ID 리스트 (favorites.json) ─────────────────────────

fn favorites_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("favorites.json"))
}

pub fn read_favorites() -> Vec<String> {
    let Some(path) = favorites_path() else {
        return Vec::new();
    };
    let Ok(json) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub fn write_favorites(favs: &[String]) -> Result<(), String> {
    let path = favorites_path().ok_or_else(|| "data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(favs).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
