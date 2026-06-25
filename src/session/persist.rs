use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::openrouter::ChatMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedBlock {
    pub id: u64,
    pub role: String,
    pub content: String,
    #[serde(default)]
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedSessionData {
    pub id: u64,
    pub title: String,
    pub conversation: Vec<ChatMessage>,
    pub blocks: Vec<PersistedBlock>,
    pub next_block_id: u64,
    #[serde(default)]
    pub scroll_y: f32,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub(crate) struct PersistedAllSessions {
    pub sessions: Vec<PersistedSessionData>,
    pub active_idx: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OldPersistedSession {
    pub conversation: Vec<ChatMessage>,
    pub blocks: Vec<PersistedBlock>,
    pub next_block_id: u64,
}

fn sessions_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("sessions.json"))
}

pub(crate) fn load_all() -> PersistedAllSessions {
    let dir = dirs::data_local_dir().map(|d| d.join("codewarp"));
    load_all_at(dir.as_deref())
}

pub(crate) fn load_all_at(dir: Option<&std::path::Path>) -> PersistedAllSessions {
    if let Some(d) = dir {
        let path = d.join("sessions.json");
        if let Ok(json) = std::fs::read_to_string(&path)
            && let Ok(p) = serde_json::from_str::<PersistedAllSessions>(&json)
            && !p.sessions.is_empty()
        {
            let mut p = p;
            if p.active_idx >= p.sessions.len() {
                p.active_idx = 0;
            }
            return p;
        }
        let old_path = d.join("session.json");
        if let Ok(json) = std::fs::read_to_string(&old_path)
            && let Ok(old) = serde_json::from_str::<OldPersistedSession>(&json)
        {
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

pub(crate) fn save_all(p: &PersistedAllSessions) -> Result<(), String> {
    let path = sessions_path().ok_or("data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}
