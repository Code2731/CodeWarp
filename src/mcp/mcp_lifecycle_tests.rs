use super::*;
use crate::test_support::process_fixture::{
    ProcessFixtureMode, command, command_with_pid, process_is_running,
};
use std::time::Duration;
use tokio::process::Command;

const TEST_DEADLINES: process::McpDeadlines = process::McpDeadlines::new(
    Duration::from_millis(200),
    Duration::from_millis(200),
    Duration::from_millis(500),
);

fn fixture_command(mode: ProcessFixtureMode) -> Command {
    Command::from(command(mode))
}

async fn fixture_call(
    mode: ProcessFixtureMode,
    method: &str,
) -> Result<(serde_json::Value, process::ProcessReceipt), process::RpcFailure> {
    rpc_call_command(
        fixture_command(mode),
        method,
        serde_json::json!({}),
        TEST_DEADLINES,
    )
    .await
}

#[test]
fn production_deadlines_are_ten_five_and_two_seconds() {
    assert_eq!(
        process::PRODUCTION_DEADLINES.response,
        Duration::from_secs(10)
    );
    assert_eq!(
        process::PRODUCTION_DEADLINES.graceful,
        Duration::from_secs(5)
    );
    assert_eq!(
        process::PRODUCTION_DEADLINES.kill_reap,
        Duration::from_secs(2)
    );
}

#[tokio::test]
async fn tools_list_success_flushes_each_request_and_reaps_child() {
    // Given: a fixture that responds to initialize and tools/list.
    // When: the complete RPC exchange finishes.
    let (result, receipt) = fixture_call(ProcessFixtureMode::McpSuccess, "tools/list")
        .await
        .unwrap();
    // Then: the tool result is returned after a graceful reap.
    assert_eq!(result["tools"][0]["name"], "fixture_echo");
    eprintln!("mode=mcp-success receipt={receipt:?}");
    assert!(receipt.pid.is_some());
    assert!(!receipt.forced);
    assert!(receipt.status.success());
}

#[tokio::test]
async fn tools_call_success_flushes_each_request_and_reaps_child() {
    // Given: a fixture that responds to initialize and tools/call.
    // When: the complete RPC exchange finishes.
    let (result, receipt) = fixture_call(ProcessFixtureMode::McpSuccess, "tools/call")
        .await
        .unwrap();
    // Then: textual content is returned after the child is reaped.
    assert_eq!(result["content"][0]["text"], "fixture tool result");
    eprintln!("mode=mcp-success receipt={receipt:?}");
    assert!(receipt.pid.is_some());
    assert!(receipt.status.success());
}

#[tokio::test]
async fn protocol_failures_return_bounded_errors_and_reap_children() {
    let cases = [
        (ProcessFixtureMode::McpSilent, "deadline"),
        (ProcessFixtureMode::McpMalformed, "deadline"),
        (ProcessFixtureMode::McpWrongId, "deadline"),
        (ProcessFixtureMode::McpRpcError, "fixture RPC error"),
        (ProcessFixtureMode::McpEarlyExit, "응답 없이 종료"),
    ];

    for (mode, expected) in cases {
        let started = tokio::time::Instant::now();
        // Given: an adversarial MCP protocol fixture.
        // When: initialize or response parsing fails.
        let failure = fixture_call(mode, "tools/list").await.unwrap_err();
        // Then: the expected bounded error includes a completed reap receipt.
        assert!(failure.message.contains(expected), "{mode:?}: {failure:?}");
        let receipt = failure.receipt.expect("cleanup receipt");
        eprintln!(
            "mode={} elapsed_ms={} receipt={receipt:?}",
            mode.as_str(),
            started.elapsed().as_millis()
        );
        assert!(receipt.pid.is_some());
        assert!(!receipt.forced, "{mode:?} should exit after stdin closes");
    }
}

#[tokio::test]
async fn shutdown_escalates_to_kill_and_reaps_ignoring_child() {
    // Given: a process that ignores both protocol input and stdin closure.
    let started = tokio::time::Instant::now();
    // When: response and graceful shutdown deadlines expire.
    let failure = fixture_call(ProcessFixtureMode::IgnoreShutdown, "tools/list")
        .await
        .unwrap_err();
    // Then: forced termination is reported and remains inside all three bounds.
    let receipt = failure.receipt.expect("forced cleanup receipt");
    eprintln!(
        "mode=ignore-shutdown elapsed_ms={} receipt={receipt:?}",
        started.elapsed().as_millis()
    );
    assert!(receipt.pid.is_some());
    assert!(receipt.forced);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn cancellation_closes_and_reaps_silent_child() {
    let temp = tempfile::TempDir::new().unwrap();
    let pid_path = temp.path().join("cancel.pid");
    let command = Command::from(command_with_pid(ProcessFixtureMode::McpSilent, &pid_path));

    // Given: an in-flight silent MCP request with a recorded child PID.
    let task = tokio::spawn(rpc_call_command(
        command,
        "tools/list",
        serde_json::json!({}),
        TEST_DEADLINES,
    ));
    let pid = wait_for_pid(&pid_path).await;
    // When: the owning RPC future is cancelled.
    task.abort();
    let _ = task.await;
    // Then: the drop cleanup path reaps the recorded child.
    wait_for_process_exit(pid).await;
    eprintln!("mode=mcp-silent cancellation_pid={pid} reaped=true");
    assert!(!process_is_running(pid));
}

#[tokio::test]
async fn blocked_request_flush_returns_inside_deadline() {
    // Given: a pipe too small for a request whose reader never consumes bytes.
    let (mut writer, _reader) = tokio::io::duplex(8);
    let deadline = Duration::from_millis(50);
    let request = serde_json::json!({"payload": "x".repeat(1024)});
    // When: the bounded request writer attempts to flush the payload.
    let started = tokio::time::Instant::now();
    let error = send_json_bounded(&mut writer, &request, deadline)
        .await
        .unwrap_err();
    // Then: the flush phase reports its own deadline without waiting forever.
    assert!(error.contains("flush deadline"));
    assert!(started.elapsed() < Duration::from_millis(250));
}

async fn wait_for_pid(path: &std::path::Path) -> u32 {
    for _ in 0..200 {
        if let Ok(value) = std::fs::read_to_string(path) {
            return value.parse().expect("fixture PID number");
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("fixture did not publish PID");
}

async fn wait_for_process_exit(pid: u32) {
    for _ in 0..200 {
        if !process_is_running(pid) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("fixture PID {pid} remained alive");
}
