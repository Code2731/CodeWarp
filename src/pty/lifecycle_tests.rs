use std::time::{Duration, Instant};

use futures_util::{Stream, StreamExt};
use tempfile::TempDir;

use super::{PRODUCTION_DEADLINES, PtyDeadlines, PtyEvent, spawn_pty_command};
use crate::test_support::process_fixture::{
    ProcessFixtureMode, process_is_running, pty_command_with_pid,
};

const TEST_DEADLINES: PtyDeadlines =
    PtyDeadlines::new(Duration::from_millis(300), Duration::from_millis(300));

async fn next_line_containing(
    stream: &mut (impl Stream<Item = PtyEvent> + Unpin),
    needle: &str,
) -> String {
    let mut seen = Vec::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    loop {
        tokio::select! {
            event = stream.next() => match event {
                Some(PtyEvent::Line(line)) if line.contains(needle) => return line,
                Some(PtyEvent::Line(line)) => seen.push(line),
                Some(PtyEvent::Exited) => {
                    panic!("PTY exited before output contained {needle}; output: {seen:?}")
                }
                None => {
                    panic!("PTY stream ended before output contained {needle}; output: {seen:?}")
                }
            },
            () = tokio::time::sleep_until(deadline) => {
                panic!("PTY output deadline waiting for {needle}; output: {seen:?}");
            }
        }
    }
}

async fn wait_for_output_before_exit(
    stream: &mut (impl Stream<Item = PtyEvent> + Unpin),
    expected_output: &str,
) {
    tokio::time::timeout(Duration::from_secs(3), async {
        let mut output_seen = false;
        loop {
            match stream.next().await {
                Some(PtyEvent::Line(line)) => output_seen |= line.contains(expected_output),
                Some(PtyEvent::Exited) | None => {
                    assert!(output_seen, "PTY exited before final output was drained");
                    return;
                }
            }
        }
    })
    .await
    .expect("PTY final-output drain deadline");
}

fn final_output_and_exit_command() -> (&'static str, &'static str) {
    if cfg!(windows) {
        ("set /A 246800+11 & exit", "246811")
    } else {
        (
            "printf '\\146\\151\\156\\141\\154\\055\\157\\165\\164\\160\\165\\164\\012'; exit",
            "final-output",
        )
    }
}

fn fixture_command(
    mode: ProcessFixtureMode,
    pid_path: &std::path::Path,
) -> portable_pty::CommandBuilder {
    pty_command_with_pid(mode, pid_path)
}

fn foreground_command() -> &'static str {
    if cfg!(windows) {
        "ping -t 127.0.0.1 >NUL"
    } else {
        "while :; do sleep 1; done"
    }
}

fn usability_checks() -> [(&'static str, &'static str); 2] {
    if cfg!(windows) {
        [("set /A 123450+7", "123457"), ("set /A 765430+8", "765438")]
    } else {
        [
            (
                "printf '\\146\\151\\170\\164\\165\\162\\055\\157\\156\\145\\012'",
                "fixture-one",
            ),
            (
                "printf '\\146\\151\\170\\164\\165\\162\\055\\164\\167\\157\\012'",
                "fixture-two",
            ),
        ]
    }
}

async fn wait_for_fixture_pid(pid_path: &std::path::Path, owned_pid: u32) {
    #[cfg(windows)]
    {
        let _ = pid_path;
        assert!(process_is_running(owned_pid));
    }
    #[cfg(not(windows))]
    {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
        while !pid_path.exists() && tokio::time::Instant::now() < deadline {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert!(
            pid_path.exists(),
            "fixture PID file missing; owned pid {owned_pid} running={}",
            process_is_running(owned_pid)
        );
    }
}

fn fixture_pid(mode: ProcessFixtureMode, pid_path: &std::path::Path, owned_pid: u32) -> u32 {
    #[cfg(windows)]
    {
        let _ = mode;
        let _ = pid_path;
        owned_pid
    }
    #[cfg(not(windows))]
    {
        if mode == ProcessFixtureMode::PtyInteractiveShell {
            owned_pid
        } else {
            std::fs::read_to_string(pid_path)
                .unwrap()
                .trim()
                .parse::<u32>()
                .unwrap()
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ctrl_c_interrupts_foreground_command_and_keeps_fixture_shell_usable_repeatedly() {
    // Given
    let root = TempDir::new().expect("temporary PTY fixture root");
    let pid_path = root.path().join("interactive.pid");
    let command = fixture_command(ProcessFixtureMode::PtyInteractiveShell, &pid_path);
    let (session, stream) = spawn_pty_command(root.path(), command, TEST_DEADLINES)
        .expect("spawn interactive PTY fixture");
    let owned_pid = session.pid().expect("owned PTY PID");
    futures_util::pin_mut!(stream);
    assert!(process_is_running(owned_pid));
    next_line_containing(&mut stream, "fixture-shell-ready").await;

    // When
    for (command, expected_output) in usability_checks() {
        session.write_line(foreground_command());
        tokio::time::sleep(Duration::from_millis(150)).await;
        session.ctrl_c();
        tokio::time::sleep(Duration::from_millis(300)).await;
        session.write_line(command);

        // Then
        next_line_containing(&mut stream, expected_output).await;
    }

    let receipt = session.shutdown().await.expect("close interactive PTY");
    let fixture_pid = fixture_pid(
        ProcessFixtureMode::PtyInteractiveShell,
        &pid_path,
        owned_pid,
    );
    eprintln!(
        "CTRL_C_RECEIPT pid={} forced={} exit={} elapsed_ms={}",
        receipt.pid,
        receipt.forced,
        receipt.status.exit_code(),
        receipt.elapsed.as_millis()
    );
    assert_eq!(receipt.pid, owned_pid);
    assert!(!process_is_running(receipt.pid));
    assert!(!process_is_running(fixture_pid));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ignored_graceful_shutdown_is_force_reaped_after_production_deadline() {
    // Given
    let root = TempDir::new().expect("temporary PTY fixture root");
    let pid_path = root.path().join("ignored.pid");
    let command = fixture_command(ProcessFixtureMode::IgnoreShutdown, &pid_path);
    let (session, stream) = spawn_pty_command(root.path(), command, PRODUCTION_DEADLINES)
        .expect("spawn ignore-shutdown PTY fixture");
    let owned_pid = session.pid().expect("owned PTY PID");
    #[cfg(windows)]
    let _ = &stream;
    #[cfg(not(windows))]
    futures_util::pin_mut!(stream);
    wait_for_fixture_pid(&pid_path, owned_pid).await;
    #[cfg(not(windows))]
    next_line_containing(&mut stream, "FIXTURE_READY mode=ignore-shutdown").await;
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert!(
        process_is_running(owned_pid),
        "ignore fixture exited before shutdown"
    );
    let started = Instant::now();

    // When
    let receipt = session.shutdown().await.expect("force-reap PTY fixture");
    eprintln!(
        "FORCE_REAP_RECEIPT pid={} forced={} exit={} elapsed_ms={}",
        receipt.pid,
        receipt.forced,
        receipt.status.exit_code(),
        receipt.elapsed.as_millis()
    );

    // Then
    assert!(receipt.forced, "receipt: {receipt:?}");
    assert!(
        receipt.elapsed >= Duration::from_secs(5),
        "elapsed: {:?}",
        receipt.elapsed
    );
    assert!(
        receipt.elapsed < Duration::from_secs(7),
        "elapsed: {:?}",
        receipt.elapsed
    );
    let fixture_pid = fixture_pid(ProcessFixtureMode::IgnoreShutdown, &pid_path, owned_pid);
    assert_eq!(receipt.pid, owned_pid);
    assert!(!process_is_running(receipt.pid));
    assert!(!process_is_running(fixture_pid));
    assert!(started.elapsed() < Duration::from_secs(7));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn normal_exit_is_reaped_and_a_new_fixture_can_restart() {
    // Given
    let root = TempDir::new().expect("temporary PTY fixture root");
    let first_pid_path = root.path().join("first.pid");
    let first_command = fixture_command(ProcessFixtureMode::PtyInteractiveShell, &first_pid_path);
    let (first, first_stream) = spawn_pty_command(root.path(), first_command, TEST_DEADLINES)
        .expect("spawn first PTY fixture");
    futures_util::pin_mut!(first_stream);
    next_line_containing(&mut first_stream, "fixture-shell-ready").await;

    // When
    let (exit_command, final_output) = final_output_and_exit_command();
    first.write_line(exit_command);
    wait_for_output_before_exit(&mut first_stream, final_output).await;
    let first_receipt = first.wait_for_exit().await.expect("reap normal PTY exit");
    let second_pid_path = root.path().join("second.pid");
    let second_command = fixture_command(ProcessFixtureMode::PtyInteractiveShell, &second_pid_path);
    let (second, second_stream) = spawn_pty_command(root.path(), second_command, TEST_DEADLINES)
        .expect("restart PTY fixture");
    futures_util::pin_mut!(second_stream);
    next_line_containing(&mut second_stream, "fixture-shell-ready").await;
    let second_receipt = second
        .shutdown()
        .await
        .expect("close restarted PTY fixture");
    eprintln!(
        "RESTART_RECEIPTS first_pid={} first_forced={} second_pid={} second_forced={}",
        first_receipt.pid, first_receipt.forced, second_receipt.pid, second_receipt.forced
    );

    // Then
    assert!(!first_receipt.forced);
    assert_ne!(first_receipt.pid, second_receipt.pid);
    assert!(!process_is_running(first_receipt.pid));
    assert!(!process_is_running(second_receipt.pid));
}
