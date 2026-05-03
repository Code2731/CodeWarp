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
    /// assistant 블록을 생성한 모델 ID (옛 데이터는 빈 문자열).
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSessionData {
    pub id: u64,
    pub title: String,
    pub conversation: Vec<ChatMessage>,
    pub blocks: Vec<PersistedBlock>,
    pub next_block_id: u64,
    /// stream 영역 absolute scroll y (마지막 위치). 세션별로 별도 보존.
    #[serde(default)]
    pub scroll_y: f32,
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

fn sessions_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("sessions.json"))
}

pub fn load_all() -> PersistedAllSessions {
    let dir = dirs::data_local_dir().map(|d| d.join("codewarp"));
    load_all_at(dir.as_deref())
}

/// 디렉토리 path를 인자로 받아 마이그레이션을 수행 — 테스트 가능 형태.
fn load_all_at(dir: Option<&std::path::Path>) -> PersistedAllSessions {
    // 1) 새 형식 우선
    if let Some(d) = dir {
        let path = d.join("sessions.json");
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
        // 2) 옛 단일 세션 마이그레이션
        let old_path = d.join("session.json");
        if let Ok(json) = std::fs::read_to_string(&old_path) {
            if let Ok(old) = serde_json::from_str::<OldPersistedSession>(&json) {
                return PersistedAllSessions {
                    sessions: vec![PersistedSessionData {
                        id: 1,
                        title: "이전 채팅".into(),
                        conversation: old.conversation,
                        blocks: old.blocks,
                        next_block_id: old.next_block_id,
                        scroll_y: 0.0,
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
            scroll_y: 0.0,
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

// ── 모델별 누적 사용량 (usage.json) ─────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelUsage {
    pub total_cost: f64,
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub call_count: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UsageStore {
    /// model id → 누적 사용량
    pub by_model: std::collections::BTreeMap<String, ModelUsage>,
}

fn usage_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("usage.json"))
}

pub fn load_usage() -> UsageStore {
    let Some(path) = usage_path() else {
        return UsageStore::default();
    };
    let Ok(json) = std::fs::read_to_string(&path) else {
        return UsageStore::default();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub fn save_usage(usage: &UsageStore) -> Result<(), String> {
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
    use tempfile::TempDir;

    #[test]
    fn empty_dir_returns_default() {
        let tmp = TempDir::new().unwrap();
        let p = load_all_at(Some(tmp.path()));
        assert_eq!(p.sessions.len(), 1);
        assert_eq!(p.sessions[0].title, "새 채팅");
        assert_eq!(p.active_idx, 0);
    }

    #[test]
    fn none_dir_returns_default() {
        let p = load_all_at(None);
        assert_eq!(p.sessions.len(), 1);
        assert_eq!(p.sessions[0].title, "새 채팅");
    }

    #[test]
    fn loads_new_format() {
        let tmp = TempDir::new().unwrap();
        let data = PersistedAllSessions {
            sessions: vec![
                PersistedSessionData {
                    id: 7,
                    title: "기존 세션".into(),
                    conversation: Vec::new(),
                    blocks: Vec::new(),
                    next_block_id: 0,
                    scroll_y: 100.0,
                },
                PersistedSessionData {
                    id: 8,
                    title: "두 번째".into(),
                    conversation: Vec::new(),
                    blocks: Vec::new(),
                    next_block_id: 0,
                    scroll_y: 0.0,
                },
            ],
            active_idx: 1,
        };
        std::fs::write(
            tmp.path().join("sessions.json"),
            serde_json::to_string(&data).unwrap(),
        )
        .unwrap();
        let p = load_all_at(Some(tmp.path()));
        assert_eq!(p.sessions.len(), 2);
        assert_eq!(p.active_idx, 1);
        assert_eq!(p.sessions[0].title, "기존 세션");
    }

    #[test]
    fn active_idx_out_of_bounds_clamped() {
        let tmp = TempDir::new().unwrap();
        let data = PersistedAllSessions {
            sessions: vec![PersistedSessionData {
                id: 1,
                title: "only".into(),
                conversation: Vec::new(),
                blocks: Vec::new(),
                next_block_id: 0,
                scroll_y: 0.0,
            }],
            active_idx: 99,
        };
        std::fs::write(
            tmp.path().join("sessions.json"),
            serde_json::to_string(&data).unwrap(),
        )
        .unwrap();
        let p = load_all_at(Some(tmp.path()));
        assert_eq!(p.active_idx, 0);
    }

    #[test]
    fn corrupt_json_falls_back_to_default() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("sessions.json"), "{ not valid json").unwrap();
        let p = load_all_at(Some(tmp.path()));
        // 손상 → default 빈 세션
        assert_eq!(p.sessions.len(), 1);
        assert_eq!(p.sessions[0].title, "새 채팅");
    }

    #[test]
    fn empty_sessions_array_falls_back() {
        let tmp = TempDir::new().unwrap();
        let data = PersistedAllSessions {
            sessions: Vec::new(),
            active_idx: 0,
        };
        std::fs::write(
            tmp.path().join("sessions.json"),
            serde_json::to_string(&data).unwrap(),
        )
        .unwrap();
        let p = load_all_at(Some(tmp.path()));
        // 빈 sessions → 옛 형식 시도 → 없음 → default
        assert_eq!(p.sessions.len(), 1);
        assert_eq!(p.sessions[0].title, "새 채팅");
    }

    #[test]
    fn migrates_old_single_session() {
        let tmp = TempDir::new().unwrap();
        let old = OldPersistedSession {
            conversation: vec![crate::openrouter::ChatMessage::user("hello")],
            blocks: vec![PersistedBlock {
                id: 0,
                role: "user".into(),
                content: "hello".into(),
                model: String::new(),
            }],
            next_block_id: 1,
        };
        std::fs::write(
            tmp.path().join("session.json"),
            serde_json::to_string(&old).unwrap(),
        )
        .unwrap();
        let p = load_all_at(Some(tmp.path()));
        assert_eq!(p.sessions.len(), 1);
        assert_eq!(p.sessions[0].title, "이전 채팅");
        assert_eq!(p.sessions[0].blocks.len(), 1);
        assert_eq!(p.sessions[0].blocks[0].content, "hello");
        assert_eq!(p.sessions[0].next_block_id, 1);
    }

    #[test]
    fn new_format_takes_precedence_over_old() {
        let tmp = TempDir::new().unwrap();
        // 옛 형식
        let old = OldPersistedSession {
            conversation: Vec::new(),
            blocks: Vec::new(),
            next_block_id: 0,
        };
        std::fs::write(
            tmp.path().join("session.json"),
            serde_json::to_string(&old).unwrap(),
        )
        .unwrap();
        // 새 형식 (다른 title)
        let new = PersistedAllSessions {
            sessions: vec![PersistedSessionData {
                id: 42,
                title: "new format".into(),
                conversation: Vec::new(),
                blocks: Vec::new(),
                next_block_id: 0,
                scroll_y: 0.0,
            }],
            active_idx: 0,
        };
        std::fs::write(
            tmp.path().join("sessions.json"),
            serde_json::to_string(&new).unwrap(),
        )
        .unwrap();
        let p = load_all_at(Some(tmp.path()));
        assert_eq!(p.sessions[0].title, "new format");
    }
}
