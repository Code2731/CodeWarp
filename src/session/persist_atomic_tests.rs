use super::persist::{
    PersistedAllSessions, PersistedSessionData, SessionLoadNotice, load_all_at,
    load_all_with_notice_at, save_all_at,
};
use std::sync::Arc;
use tempfile::TempDir;

fn sessions(title: &str) -> PersistedAllSessions {
    PersistedAllSessions {
        sessions: vec![PersistedSessionData {
            id: 7,
            title: title.into(),
            conversation: Arc::new(Vec::new()),
            blocks: Vec::new(),
            next_block_id: 0,
            scroll_y: 0.0,
        }],
        active_idx: 0,
    }
}

fn title_at(path: &std::path::Path) -> String {
    let json = std::fs::read_to_string(path).unwrap();
    let persisted: PersistedAllSessions = serde_json::from_str(&json).unwrap();
    persisted.sessions[0].title.clone()
}

#[test]
fn first_save_promotes_validated_tmp_and_establishes_backup() {
    // Given
    let tmp = TempDir::new().unwrap();

    // When
    save_all_at(tmp.path(), &sessions("first")).unwrap();

    // Then
    assert_eq!(title_at(&tmp.path().join("sessions.json")), "first");
    assert_eq!(title_at(&tmp.path().join("sessions.json.bak")), "first");
    assert!(!tmp.path().join("sessions.json.tmp").exists());
}

#[test]
fn later_save_preserves_previous_valid_primary_as_backup() {
    // Given
    let tmp = TempDir::new().unwrap();
    save_all_at(tmp.path(), &sessions("previous")).unwrap();

    // When
    save_all_at(tmp.path(), &sessions("current")).unwrap();

    // Then
    assert_eq!(title_at(&tmp.path().join("sessions.json")), "current");
    assert_eq!(title_at(&tmp.path().join("sessions.json.bak")), "previous");
}

#[test]
fn corrupt_primary_is_quarantined_without_overwriting_valid_backup() {
    // Given
    let tmp = TempDir::new().unwrap();
    save_all_at(tmp.path(), &sessions("recoverable")).unwrap();
    std::fs::write(tmp.path().join("sessions.json"), b"{corrupt").unwrap();

    // When
    save_all_at(tmp.path(), &sessions("replacement")).unwrap();

    // Then
    assert_eq!(title_at(&tmp.path().join("sessions.json")), "replacement");
    assert_eq!(
        title_at(&tmp.path().join("sessions.json.bak")),
        "recoverable"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("sessions.json.corrupt-0.json")).unwrap(),
        b"{corrupt"
    );
}

#[test]
fn interrupted_corrupt_primary_loads_backup_and_preserves_evidence() {
    // Given
    let tmp = TempDir::new().unwrap();
    save_all_at(tmp.path(), &sessions("last known good")).unwrap();
    std::fs::write(tmp.path().join("sessions.json"), b"{interrupted").unwrap();
    std::fs::write(tmp.path().join("sessions.json.tmp"), b"{partial").unwrap();

    // When
    let loaded = load_all_at(Some(tmp.path()));

    // Then
    assert_eq!(loaded.sessions[0].title, "last known good");
    assert_eq!(
        std::fs::read(tmp.path().join("sessions.json")).unwrap(),
        b"{interrupted"
    );
    assert_eq!(
        std::fs::read(tmp.path().join("sessions.json.tmp")).unwrap(),
        b"{partial"
    );
}

#[test]
fn missing_primary_loads_valid_backup_before_legacy() {
    // Given
    let tmp = TempDir::new().unwrap();
    let backup = serde_json::to_vec(&sessions("backup")).unwrap();
    std::fs::write(tmp.path().join("sessions.json.bak"), backup).unwrap();
    std::fs::write(tmp.path().join("session.json"), "{}").unwrap();

    // When
    let loaded = load_all_at(Some(tmp.path()));

    // Then
    assert_eq!(loaded.sessions[0].title, "backup");
}

#[test]
fn backup_fallback_reports_actionable_recovery_notice() {
    // Given
    let tmp = TempDir::new().unwrap();
    let backup = serde_json::to_vec(&sessions("backup")).unwrap();
    std::fs::write(tmp.path().join("sessions.json"), b"{corrupt").unwrap();
    std::fs::write(tmp.path().join("sessions.json.bak"), backup).unwrap();

    // When
    let (loaded, notice) = load_all_with_notice_at(Some(tmp.path()));

    // Then
    assert_eq!(loaded.sessions[0].title, "backup");
    match notice {
        Some(SessionLoadNotice::BackupFallback { backup, .. }) => {
            assert!(backup.ends_with("sessions.json.bak"));
        }
        Some(SessionLoadNotice::LegacyMigration) => panic!("unexpected legacy migration"),
        None => panic!("missing backup fallback notice"),
    }
}
