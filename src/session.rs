// 멀티 세션 영구 저장
// Windows: %LOCALAPPDATA%\codewarp\sessions.json (sessions list)
// macOS:   ~/Library/Application Support/codewarp/sessions.json
// Linux:   ~/.local/share/codewarp/sessions.json
//
// 호환: 옛 단일 세션 파일(session.json)이 있으면 첫 회 load 시 자동 마이그레이션.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::openrouter::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedBlock {
    pub id: u64,
    pub role: String, // "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSessionData {
    pub id: u64,
    pub title: String,
    pub conversation: Vec<ChatMessage>,
    pub blocks: Vec<PersistedBlock>,
    pub next_block_id: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PersistedAllSessions {
    pub sessions: Vec<PersistedSessionData>,
    pub active_idx: usize,
}

// ── 옛 단일 세션 호환 ──────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct OldPersistedSession {
    pub conversation: Vec<ChatMessage>,
    pub blocks: Vec<PersistedBlock>,
    pub next_block_id: u64,
}

fn old_session_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("session.json"))
}

fn sessions_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("sessions.json"))
}

pub fn load_all() -> PersistedAllSessions {
    // 1) 새 형식 우선
    if let Some(path) = sessions_path() {
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(p) = serde_json::from_str::<PersistedAllSessions>(&json) {
                if !p.sessions.is_empty() {
                    let mut p = p;
                    if p.active_idx >= p.sessions.len() {
                        p.active_idx = 0;
                    }
                    return p;
                }
            }
        }
    }
    // 2) 옛 단일 세션 마이그레이션
    if let Some(path) = old_session_path() {
        if let Ok(json) = std::fs::read_to_string(&path) {
            if let Ok(old) = serde_json::from_str::<OldPersistedSession>(&json) {
                return PersistedAllSessions {
                    sessions: vec![PersistedSessionData {
                        id: 1,
                        title: "이전 채팅".into(),
                        conversation: old.conversation,
                        blocks: old.blocks,
                        next_block_id: old.next_block_id,
                    }],
                    active_idx: 0,
                };
            }
        }
    }
    // 3) 빈 시작
    PersistedAllSessions {
        sessions: vec![PersistedSessionData {
            id: 1,
            title: "새 채팅".into(),
            conversation: Vec::new(),
            blocks: Vec::new(),
            next_block_id: 0,
        }],
        active_idx: 0,
    }
}

pub fn save_all(p: &PersistedAllSessions) -> Result<(), String> {
    let path = sessions_path().ok_or_else(|| "data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
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
