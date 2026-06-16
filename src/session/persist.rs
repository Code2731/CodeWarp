use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::openrouter::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedBlock {
    pub id: u64,
    pub role: String,
    pub content: String,
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
    #[serde(default)]
    pub scroll_y: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PersistedAllSessions {
    pub sessions: Vec<PersistedSessionData>,
    pub active_idx: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OldPersistedSession {
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

pub fn load_all_at(dir: Option<&std::path::Path>) -> PersistedAllSessions {
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

    #[test]
    fn json_roundtrip_preserves_full_state() {
        let data = PersistedAllSessions {
            sessions: vec![
                PersistedSessionData {
                    id: 1,
                    title: "chat one".into(),
                    conversation: vec![
                        ChatMessage::user("hello"),
                        ChatMessage::assistant("hi there"),
                    ],
                    blocks: vec![
                        PersistedBlock {
                            id: 10,
                            role: "user".into(),
                            content: "hello".into(),
                            model: String::new(),
                        },
                        PersistedBlock {
                            id: 11,
                            role: "assistant".into(),
                            content: "hi there".into(),
                            model: "gpt-4o".into(),
                        },
                    ],
                    next_block_id: 12,
                    scroll_y: 42.5,
                },
                PersistedSessionData {
                    id: 2,
                    title: "chat two".into(),
                    conversation: Vec::new(),
                    blocks: Vec::new(),
                    next_block_id: 0,
                    scroll_y: 0.0,
                },
            ],
            active_idx: 1,
        };
        let json = serde_json::to_string_pretty(&data).unwrap();
        let loaded: PersistedAllSessions = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.sessions.len(), 2);
        assert_eq!(loaded.active_idx, 1);
        assert_eq!(loaded.sessions[0].title, "chat one");
        assert_eq!(loaded.sessions[0].blocks.len(), 2);
        assert_eq!(loaded.sessions[0].blocks[1].model, "gpt-4o");
        assert_eq!(loaded.sessions[0].scroll_y, 42.5);
        assert_eq!(loaded.sessions[1].title, "chat two");
    }

    #[test]
    fn load_all_at_non_existent_dir_returns_default() {
        let tmp = TempDir::new().unwrap();
        let phantom = tmp.path().join("does-not-exist");
        let loaded = load_all_at(Some(&phantom));
        assert_eq!(loaded.sessions.len(), 1);
        assert_eq!(loaded.sessions[0].title, "새 채팅");
    }
}
