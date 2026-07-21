use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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
    pub conversation: Arc<Vec<ChatMessage>>,
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

const PRIMARY_NAME: &str = "sessions.json";
const TMP_NAME: &str = "sessions.json.tmp";
const BACKUP_NAME: &str = "sessions.json.bak";
const BACKUP_TMP_NAME: &str = "sessions.json.bak.tmp";

#[derive(Debug)]
pub(crate) enum SessionLoadNotice {
    BackupFallback { primary: PathBuf, backup: PathBuf },
    LegacyMigration,
}

#[cfg(not(test))]
fn sessions_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp"))
}

fn default_sessions() -> PersistedAllSessions {
    PersistedAllSessions {
        sessions: vec![PersistedSessionData {
            id: 1,
            title: "새 채팅".into(),
            conversation: Arc::new(Vec::new()),
            blocks: Vec::new(),
            next_block_id: 0,
            scroll_y: 0.0,
        }],
        active_idx: 0,
    }
}

fn read_valid(path: &Path) -> Result<PersistedAllSessions, String> {
    let json =
        std::fs::read_to_string(path).map_err(|e| format!("{} 읽기 실패: {e}", path.display()))?;
    let mut persisted: PersistedAllSessions = serde_json::from_str(&json)
        .map_err(|e| format!("{} JSON 검증 실패: {e}", path.display()))?;
    if persisted.sessions.is_empty() {
        return Err(format!("{} 세션 목록이 비어 있습니다", path.display()));
    }
    if persisted.active_idx >= persisted.sessions.len() {
        persisted.active_idx = 0;
    }
    Ok(persisted)
}

fn read_legacy(path: &Path) -> Result<PersistedAllSessions, String> {
    let json =
        std::fs::read_to_string(path).map_err(|e| format!("{} 읽기 실패: {e}", path.display()))?;
    let old: OldPersistedSession = serde_json::from_str(&json)
        .map_err(|e| format!("{} JSON 검증 실패: {e}", path.display()))?;
    Ok(PersistedAllSessions {
        sessions: vec![PersistedSessionData {
            id: 1,
            title: "이전 채팅".into(),
            conversation: Arc::new(old.conversation),
            blocks: old.blocks,
            next_block_id: old.next_block_id,
            scroll_y: 0.0,
        }],
        active_idx: 0,
    })
}

#[cfg(not(test))]
pub(crate) fn load_all_with_notice() -> (PersistedAllSessions, Option<SessionLoadNotice>) {
    load_all_with_notice_at(sessions_dir().as_deref())
}

#[cfg(test)]
pub(crate) fn load_all_with_notice() -> (PersistedAllSessions, Option<SessionLoadNotice>) {
    (default_sessions(), None)
}

#[cfg(test)]
pub(crate) fn load_all_at(dir: Option<&Path>) -> PersistedAllSessions {
    load_all_with_notice_at(dir).0
}

pub(crate) fn load_all_with_notice_at(
    dir: Option<&Path>,
) -> (PersistedAllSessions, Option<SessionLoadNotice>) {
    let Some(dir) = dir else {
        return (default_sessions(), None);
    };
    let primary = dir.join(PRIMARY_NAME);
    if let Ok(persisted) = read_valid(&primary) {
        return (persisted, None);
    }
    let backup = dir.join(BACKUP_NAME);
    if let Ok(persisted) = read_valid(&backup) {
        return (
            persisted,
            Some(SessionLoadNotice::BackupFallback { primary, backup }),
        );
    }
    if let Ok(persisted) = read_legacy(&dir.join("session.json")) {
        return (persisted, Some(SessionLoadNotice::LegacyMigration));
    }
    (default_sessions(), None)
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("{} 열기 실패: {e}", path.display()))?;
    file.write_all(bytes)
        .map_err(|e| format!("{} 쓰기 실패: {e}", path.display()))?;
    file.sync_all()
        .map_err(|e| format!("{} 동기화 실패: {e}", path.display()))
}

fn copy_synced_valid(source: &Path, destination: &Path) -> Result<(), String> {
    let bytes =
        std::fs::read(source).map_err(|e| format!("{} 읽기 실패: {e}", source.display()))?;
    write_synced(destination, &bytes)?;
    read_valid(destination).map(|_| ())
}

fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::rename(source, destination).map_err(|e| {
        format!(
            "{} -> {} 원자적 교체 실패: {e}",
            source.display(),
            destination.display()
        )
    })
}

#[cfg(all(test, windows))]
mod replace_file_tests {
    use super::replace_file;
    use tempfile::TempDir;

    #[test]
    fn replace_error_preserves_existing_destination() {
        // Given
        let tmp = TempDir::new().unwrap();
        let source = tmp.path().join("missing-source");
        let destination = tmp.path().join("destination");
        std::fs::write(&destination, b"known destination").unwrap();

        // When
        let result = replace_file(&source, &destination);

        // Then
        assert!(result.is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"known destination");
    }
}

fn quarantine_path(dir: &Path) -> PathBuf {
    let mut monotonic = 0_u64;
    loop {
        let candidate = dir.join(format!("sessions.json.corrupt-{monotonic}.json"));
        if !candidate.exists() {
            return candidate;
        }
        monotonic = monotonic.saturating_add(1);
    }
}

#[cfg(not(test))]
pub(crate) fn save_all(persisted: &PersistedAllSessions) -> Result<(), String> {
    let dir = sessions_dir().ok_or_else(|| "data_local_dir 없음".to_string())?;
    save_all_at(&dir, persisted)
}

#[cfg(test)]
pub(crate) fn save_all(_persisted: &PersistedAllSessions) -> Result<(), String> {
    Ok(())
}

pub(crate) fn save_all_at(dir: &Path, persisted: &PersistedAllSessions) -> Result<(), String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("{} 세션 디렉터리 생성 실패: {e}", dir.display()))?;

    let primary = dir.join(PRIMARY_NAME);
    let temporary = dir.join(TMP_NAME);
    let backup = dir.join(BACKUP_NAME);
    let backup_temporary = dir.join(BACKUP_TMP_NAME);
    let json =
        serde_json::to_vec_pretty(persisted).map_err(|e| format!("세션 JSON 직렬화 실패: {e}"))?;
    write_synced(&temporary, &json)?;
    read_valid(&temporary)?;

    let primary_is_valid = read_valid(&primary).is_ok();
    let backup_is_valid = read_valid(&backup).is_ok();
    if primary.exists() {
        if primary_is_valid {
            copy_synced_valid(&primary, &backup_temporary)?;
            replace_file(&backup_temporary, &backup)?;
        } else {
            let quarantine = quarantine_path(dir);
            std::fs::rename(&primary, &quarantine).map_err(|e| {
                format!(
                    "손상된 {} -> {} 격리 실패: {e}",
                    primary.display(),
                    quarantine.display()
                )
            })?;
        }
    }

    replace_file(&temporary, &primary)?;
    if !primary_is_valid && !backup_is_valid {
        copy_synced_valid(&primary, &backup_temporary)?;
        replace_file(&backup_temporary, &backup)?;
    }
    Ok(())
}
