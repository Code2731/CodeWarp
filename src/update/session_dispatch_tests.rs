use super::*;
use crate::pty::{PtyDeadlines, spawn_pty_command};
use crate::test_support::process_fixture::{ProcessFixtureMode, pty_command_with_pid};
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn failed_window_close_save_does_not_create_clean_marker() {
    // Given
    let storage = TempDir::new().unwrap();
    std::fs::create_dir(storage.path().join("sessions.json.tmp")).unwrap();
    let marker = TempDir::new().unwrap();
    let (mut app, _) = App::new();

    // When
    app.window_close_at(storage.path(), marker.path());

    // Then
    assert!(app.status.contains("세션 저장 실패"), "status: {}", app.status);
    assert!(!marker.path().join(".clean_shutdown").exists());
}

#[test]
fn marker_write_failure_is_exposed_in_app_status_with_injected_roots() {
    // Given
    let storage = TempDir::new().unwrap();
    let marker_parent = TempDir::new().unwrap();
    let marker_root = marker_parent.path().join("not-a-directory");
    std::fs::write(&marker_root, "blocking file").unwrap();
    let (mut app, _) = App::new();

    // When
    app.window_close_at(storage.path(), &marker_root);

    // Then
    assert!(app.status.contains("정상 종료 표시 실패"), "status: {}", app.status);
    assert!(
        app.status.contains(&marker_root.display().to_string()),
        "status: {}",
        app.status
    );
}

#[test]
fn coordinated_close_records_cancel_reap_persist_marker_then_close() {
    // Given
    let storage = TempDir::new().unwrap();
    let marker = TempDir::new().unwrap();
    let (mut app, _) = App::new();

    // When
    let _task = app.begin_window_close();
    let close_permitted = app.complete_window_close_at(Ok(()), storage.path(), marker.path());

    // Then
    assert!(close_permitted);
    eprintln!("LIFECYCLE_TRACE {:?}", app.close_lifecycle_events);
    assert_eq!(
        app.close_lifecycle_events,
        vec![
            CloseLifecycleEvent::CancelStreamAndMcp,
            CloseLifecycleEvent::ReapInferenceAndPty,
            CloseLifecycleEvent::Persist,
            CloseLifecycleEvent::MarkClean,
            CloseLifecycleEvent::CloseWindow,
        ]
    );
    assert!(marker.path().join(".clean_shutdown").exists());
}

#[test]
fn persistence_failure_prevents_marker_and_window_close() {
    // Given
    let storage = TempDir::new().unwrap();
    std::fs::create_dir(storage.path().join("sessions.json.tmp")).unwrap();
    let marker = TempDir::new().unwrap();
    let (mut app, _) = App::new();

    // When
    let _task = app.begin_window_close();
    let close_permitted = app.complete_window_close_at(Ok(()), storage.path(), marker.path());

    // Then
    assert!(!close_permitted);
    eprintln!("PERSISTENCE_FAILURE_TRACE {:?}", app.close_lifecycle_events);
    assert_eq!(
        app.close_lifecycle_events,
        vec![
            CloseLifecycleEvent::CancelStreamAndMcp,
            CloseLifecycleEvent::ReapInferenceAndPty,
            CloseLifecycleEvent::Persist,
        ]
    );
    assert!(!marker.path().join(".clean_shutdown").exists());
}

#[test]
fn marker_failure_preserves_unclean_status_but_permits_window_close() {
    // Given
    let storage = TempDir::new().unwrap();
    let marker_parent = TempDir::new().unwrap();
    let marker_root = marker_parent.path().join("not-a-directory");
    std::fs::write(&marker_root, "blocking file").unwrap();
    let (mut app, _) = App::new();

    // When
    let _task = app.begin_window_close();
    let close_permitted = app.complete_window_close_at(Ok(()), storage.path(), &marker_root);

    // Then
    assert!(close_permitted);
    eprintln!(
        "MARKER_FAILURE_TRACE events={:?} status={}",
        app.close_lifecycle_events, app.status
    );
    assert_eq!(
        app.close_lifecycle_events,
        vec![
            CloseLifecycleEvent::CancelStreamAndMcp,
            CloseLifecycleEvent::ReapInferenceAndPty,
            CloseLifecycleEvent::Persist,
            CloseLifecycleEvent::MarkClean,
            CloseLifecycleEvent::CloseWindow,
        ]
    );
    assert!(app.status.contains("정상 종료 표시 실패"));
    assert!(storage.path().join("sessions.json").exists());
    assert!(!marker_root.join(".clean_shutdown").exists());
    let reloaded = session::load_all_at(Some(storage.path()));
    assert!(!reloaded.sessions.is_empty());
    assert!(!session::was_clean_shutdown_at(&marker_root).unwrap_or(false));
}

#[test]
fn partial_marker_failure_removes_temporary_marker_and_stays_unclean() {
    // Given
    let storage = TempDir::new().unwrap();
    let marker = TempDir::new().unwrap();
    std::fs::create_dir(marker.path().join(".clean_shutdown")).unwrap();
    let (mut app, _) = App::new();

    // When
    let _task = app.begin_window_close();
    let close_permitted = app.complete_window_close_at(Ok(()), storage.path(), marker.path());

    // Then
    assert!(close_permitted);
    assert!(app.status.contains("정상 종료 표시 실패"));
    assert!(!marker.path().join(".clean_shutdown.tmp").exists());
    assert!(!session::was_clean_shutdown_at(marker.path()).unwrap_or(false));
}

#[test]
fn reap_failure_prevents_persistence_marker_and_window_close() {
    // Given
    let storage = TempDir::new().unwrap();
    let marker = TempDir::new().unwrap();
    let (mut app, _) = App::new();

    // When
    let _task = app.begin_window_close();
    let close_permitted = app.complete_window_close_at(
        Err("fixture reap failed".to_string()),
        storage.path(),
        marker.path(),
    );

    // Then
    assert!(!close_permitted);
    assert_eq!(
        app.close_lifecycle_events,
        vec![
            CloseLifecycleEvent::CancelStreamAndMcp,
            CloseLifecycleEvent::ReapInferenceAndPty,
        ]
    );
    assert!(app.status.contains("fixture reap failed"));
    assert!(!storage.path().join("sessions.json").exists());
    assert!(!marker.path().join(".clean_shutdown").exists());
}

#[test]
fn repeated_close_request_does_not_duplicate_cancel_or_reap_stages() {
    // Given
    let (mut app, _) = App::new();
    let (_mcp_task, mcp_handle) = Task::<Message>::none().abortable();
    app.mcp_abort_handle = Some(mcp_handle);

    // When
    let _first = app.begin_window_close();
    let _second = app.begin_window_close();

    // Then
    assert!(app.close_in_progress);
    assert!(app.mcp_abort_handle.is_none());
    assert_eq!(
        app.close_lifecycle_events,
        vec![
            CloseLifecycleEvent::CancelStreamAndMcp,
            CloseLifecycleEvent::ReapInferenceAndPty,
        ]
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reap_failure_restores_pty_ownership_for_retry() {
    // Given
    let root = TempDir::new().unwrap();
    let command = pty_command_with_pid(
        ProcessFixtureMode::PtyInteractiveShell,
        &root.path().join("restore.pid"),
    );
    let deadlines = PtyDeadlines::new(Duration::from_millis(300), Duration::from_millis(300));
    let (session, _stream) = spawn_pty_command(root.path(), command, deadlines).unwrap();
    let pid = session.pid();
    let (mut app, _) = App::new();
    app.close_in_progress = true;

    // When
    let _task = app.complete_window_close(CloseReapOutcome {
        result: Err("fixture reap failed".to_string()),
        inference: None,
        pty: Some(session),
    });

    // Then
    assert_eq!(app.pty_session.as_ref().and_then(crate::pty::PtySession::pid), pid);
    let restored = app.pty_session.take().unwrap();
    let receipt = restored.shutdown().await.unwrap();
    assert_eq!(Some(receipt.pid), pid);
}
