use super::{RuntimeDeadlines, RuntimeProcess};
use crate::test_support::process_fixture::{ProcessFixtureMode, command, process_is_running};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{ChildStdout, Command};

const TEST_DEADLINES: RuntimeDeadlines = RuntimeDeadlines::new(
    Duration::from_millis(250),
    Duration::from_millis(150),
    Duration::from_secs(1),
);
const OBSERVATION_DEADLINE: Duration = Duration::from_secs(2);

fn fixture_command(mode: ProcessFixtureMode) -> Command {
    let mut fixture = Command::from(command(mode));
    fixture
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    fixture
}

async fn spawn_fixture(
    mode: ProcessFixtureMode,
) -> (
    RuntimeProcess,
    tokio::io::Lines<BufReader<ChildStdout>>,
    u32,
) {
    spawn_fixture_with_deadlines(mode, TEST_DEADLINES).await
}

async fn spawn_fixture_with_deadlines(
    mode: ProcessFixtureMode,
    deadlines: RuntimeDeadlines,
) -> (
    RuntimeProcess,
    tokio::io::Lines<BufReader<ChildStdout>>,
    u32,
) {
    let (process, stdout, _stderr) = RuntimeProcess::spawn(fixture_command(mode), deadlines)
        .expect("spawn managed runtime fixture");
    let pid = process.pid().expect("runtime fixture PID");
    (process, BufReader::new(stdout).lines(), pid)
}

async fn read_until(lines: &mut tokio::io::Lines<BufReader<ChildStdout>>, prefix: &str) -> String {
    tokio::time::timeout(OBSERVATION_DEADLINE, async {
        loop {
            let line = lines
                .next_line()
                .await
                .expect("read fixture output")
                .expect("fixture exited before expected output");
            if line.starts_with(prefix) {
                return line;
            }
        }
    })
    .await
    .expect("fixture output deadline exceeded")
}

async fn start_health_fixture() -> (RuntimeProcess, u32) {
    let (mut process, mut lines, pid) = spawn_fixture(ProcessFixtureMode::RuntimeHttpHealth).await;
    let address = read_until(&mut lines, "FIXTURE_HTTP ").await;
    let health_url = format!(
        "http://{}/health",
        address
            .strip_prefix("FIXTURE_HTTP ")
            .expect("fixture health address")
    );
    process
        .wait_until_healthy(&health_url)
        .await
        .expect("fixture health response");
    (process, pid)
}

async fn wait_until_reaped(pid: u32) {
    tokio::time::timeout(OBSERVATION_DEADLINE, async {
        while process_is_running(pid) {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("runtime fixture was not reaped");
}

#[tokio::test]
async fn runtime_start_requires_successful_health_response() {
    // Given: a managed runtime fixture that exposes a real loopback health endpoint.
    let started_at = Instant::now();

    // When: startup waits for the endpoint's successful response.
    let (mut process, pid) = start_health_fixture().await;

    // Then: readiness is bounded and the owned child remains live until shutdown.
    assert!(started_at.elapsed() < TEST_DEADLINES.startup);
    assert!(process_is_running(pid));
    let receipt = process.shutdown().await.expect("graceful runtime shutdown");
    assert!(!receipt.forced);
    assert_eq!(receipt.pid, Some(pid));
    assert!(!process_is_running(pid));
}

#[tokio::test]
async fn runtime_can_restart_after_owned_child_is_reaped() {
    // Given: one healthy managed runtime that has completed shutdown and reap.
    let (mut first, first_pid) = start_health_fixture().await;
    first.shutdown().await.expect("stop first runtime");
    assert!(!process_is_running(first_pid));

    // When: the lifecycle starts the fixture again.
    let (mut second, second_pid) = start_health_fixture().await;

    // Then: a distinct live child is owned and can also be cleanly reaped.
    assert_ne!(first_pid, second_pid);
    assert!(process_is_running(second_pid));
    second.shutdown().await.expect("stop restarted runtime");
    assert!(!process_is_running(second_pid));
}

#[tokio::test]
async fn unexpected_child_exit_is_observed_and_reaped() {
    // Given: a healthy fixture whose stdin lifetime controls its process lifetime.
    let (mut process, pid) = start_health_fixture().await;

    // When: the child exits without an explicit lifecycle stop request.
    process.close_stdin();
    let receipt = process.wait_for_exit().await.expect("observe child exit");

    // Then: the exit is reaped and cannot leave stale live-process state.
    assert_eq!(receipt.pid, Some(pid));
    assert!(receipt.status.code().is_some());
    assert!(!receipt.forced);
    assert!(!process_is_running(pid));
}

#[tokio::test]
async fn health_timeout_cleans_and_reaps_owned_child() {
    // Given: a child that stays alive but has no endpoint at the requested address.
    let (mut process, _lines, pid) = spawn_fixture(ProcessFixtureMode::IgnoreShutdown).await;
    assert!(process_is_running(pid));

    // When: the startup health deadline expires.
    let failure = process
        .wait_until_healthy("http://127.0.0.1:9/health")
        .await
        .expect_err("health timeout must fail startup");

    // Then: timeout owns escalation through force-kill and reap.
    assert!(failure.message.contains("startup deadline"));
    let receipt = failure.receipt.expect("timeout cleanup receipt");
    assert_eq!(receipt.pid, Some(pid));
    assert!(receipt.forced);
    assert!(!process_is_running(pid));
}

#[tokio::test]
async fn cancelling_startup_reaps_owned_child() {
    // Given: an in-flight startup probe that owns an unresponsive child.
    let (mut process, _lines, pid) = spawn_fixture(ProcessFixtureMode::IgnoreShutdown).await;
    assert!(process_is_running(pid));
    let startup = tokio::spawn(async move {
        process
            .wait_until_healthy("http://127.0.0.1:9/health")
            .await
    });

    // When: the startup task is cancelled before its health deadline.
    startup.abort();
    let cancelled = startup.await.expect_err("startup task must be cancelled");
    assert!(cancelled.is_cancelled());

    // Then: drop cleanup still escalates and reaps the owned child.
    wait_until_reaped(pid).await;
    assert!(!process_is_running(pid));
}

#[tokio::test]
async fn ignored_graceful_shutdown_is_force_killed_and_reaped() {
    // Given: a managed fixture that intentionally ignores stdin closure.
    let (mut process, _lines, pid) = spawn_fixture(ProcessFixtureMode::IgnoreShutdown).await;
    assert!(process_is_running(pid));
    let started_at = Instant::now();

    // When: owned shutdown reaches its graceful deadline.
    let receipt = process.shutdown().await.expect("forced runtime shutdown");

    // Then: force-kill/reap follows grace without exceeding the bounded envelope.
    let elapsed = started_at.elapsed();
    assert!(elapsed >= TEST_DEADLINES.graceful);
    assert!(
        elapsed < TEST_DEADLINES.graceful + TEST_DEADLINES.kill_reap + Duration::from_millis(500)
    );
    assert_eq!(receipt.pid, Some(pid));
    assert!(receipt.forced);
    assert!(!process_is_running(pid));
}

#[tokio::test]
#[ignore = "manual QA uses production 10/5/2 second deadlines"]
async fn production_health_fixture_reports_ready_and_reaps() {
    // Given: the repository health fixture under production lifecycle deadlines.
    let (mut process, mut lines, pid) = spawn_fixture_with_deadlines(
        ProcessFixtureMode::RuntimeHttpHealth,
        super::PRODUCTION_DEADLINES,
    )
    .await;
    let address = read_until(&mut lines, "FIXTURE_HTTP ").await;
    let health_url = format!(
        "http://{}/health",
        address
            .strip_prefix("FIXTURE_HTTP ")
            .expect("fixture health address")
    );
    let started_at = Instant::now();

    // When: the owned lifecycle waits for HTTP success and then stops the child.
    process
        .wait_until_healthy(&health_url)
        .await
        .expect("production health startup");
    let ready_elapsed = started_at.elapsed();
    let receipt = process.shutdown().await.expect("production graceful stop");

    // Then: readiness precedes 10 seconds and the child is reaped.
    eprintln!(
        "mode=runtime-http-health pid={pid} ready_elapsed_ms={} forced={} status={} reaped={}",
        ready_elapsed.as_millis(),
        receipt.forced,
        receipt.status,
        !process_is_running(pid),
    );
    assert!(ready_elapsed < super::PRODUCTION_DEADLINES.startup);
    assert!(!receipt.forced);
    assert!(!process_is_running(pid));
}

#[tokio::test]
#[ignore = "manual QA waits for the production 5-second graceful deadline"]
async fn production_ignore_shutdown_waits_then_force_reaps() {
    // Given: the ignore-shutdown fixture under production lifecycle deadlines.
    let (mut process, _lines, pid) = spawn_fixture_with_deadlines(
        ProcessFixtureMode::IgnoreShutdown,
        super::PRODUCTION_DEADLINES,
    )
    .await;
    assert!(process_is_running(pid));
    let started_at = Instant::now();

    // When: graceful shutdown expires and force-kill/reap runs.
    let receipt = process.shutdown().await.expect("production forced stop");
    let elapsed = started_at.elapsed();

    // Then: elapsed time and the OS process table prove 5/2 escalation cleanup.
    eprintln!(
        "mode=ignore-shutdown pid={pid} elapsed_ms={} forced={} status={} reaped={}",
        elapsed.as_millis(),
        receipt.forced,
        receipt.status,
        !process_is_running(pid),
    );
    assert!(elapsed >= super::PRODUCTION_DEADLINES.graceful);
    assert!(elapsed < super::PRODUCTION_DEADLINES.graceful + super::PRODUCTION_DEADLINES.kill_reap);
    assert!(receipt.forced);
    assert!(!process_is_running(pid));
}
