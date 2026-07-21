use super::process_fixture::{ProcessFixtureMode, command, process_is_running};
use std::io::{BufRead, BufReader as SyncBufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdout, Command};

const IO_DEADLINE: Duration = Duration::from_secs(2);
const KILL_REAP_DEADLINE: Duration = Duration::from_secs(2);

async fn read_until(lines: &mut tokio::io::Lines<BufReader<ChildStdout>>, prefix: &str) -> String {
    tokio::time::timeout(IO_DEADLINE, async {
        loop {
            let line = lines
                .next_line()
                .await
                .expect("read fixture output")
                .expect("fixture exited before readiness");
            if line.starts_with(prefix) {
                return line;
            }
        }
    })
    .await
    .expect("fixture readiness deadline exceeded")
}

async fn close_and_wait(mut child: Child) -> ExitStatus {
    drop(child.stdin.take());
    match tokio::time::timeout(IO_DEADLINE, child.wait()).await {
        Ok(result) => result.expect("wait for fixture exit"),
        Err(_) => {
            tokio::time::timeout(KILL_REAP_DEADLINE, child.kill())
                .await
                .expect("fixture kill/reap deadline exceeded")
                .expect("kill fixture");
            tokio::time::timeout(KILL_REAP_DEADLINE, child.wait())
                .await
                .expect("fixture post-kill reap deadline exceeded")
                .expect("reap killed fixture")
        }
    }
}

fn request_health(address: &str) -> String {
    let address: SocketAddr = address.parse().expect("fixture health address");
    let mut stream = TcpStream::connect_timeout(&address, IO_DEADLINE).expect("connect health");
    stream
        .set_read_timeout(Some(IO_DEADLINE))
        .expect("set health read timeout");
    stream
        .set_write_timeout(Some(IO_DEADLINE))
        .expect("set health write timeout");
    stream
        .write_all(b"GET /health HTTP/1.1\r\nHost: fixture\r\n\r\n")
        .expect("write health request");
    let mut response = String::new();
    SyncBufReader::new(stream)
        .read_line(&mut response)
        .expect("read health response");
    response
}

#[tokio::test]
async fn runtime_http_health_mode_serves_health_and_is_reaped() {
    // Given: the runtime health fixture with bounded captured output.
    let mut fixture_command = Command::from(command(ProcessFixtureMode::RuntimeHttpHealth));
    fixture_command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    let mut child = fixture_command.spawn().unwrap();
    let pid = child.id().expect("runtime fixture PID");
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let ready = read_until(&mut lines, "FIXTURE_HTTP ").await;
    let address = ready
        .trim()
        .strip_prefix("FIXTURE_HTTP ")
        .expect("health ready prefix")
        .to_string();
    // When: a bounded real loopback health request is sent.
    let response = tokio::time::timeout(
        IO_DEADLINE,
        tokio::task::spawn_blocking(move || request_health(&address)),
    )
    .await
    .expect("health task deadline exceeded")
    .expect("health task panicked");
    // Then: health succeeds and stdin closure gracefully reaps the process.
    assert!(response.contains("200 OK"));
    let status = close_and_wait(child).await;
    eprintln!("mode=runtime-http-health pid={pid} status={status}");
    assert!(status.success());
    assert!(!process_is_running(pid));
}

#[tokio::test]
async fn pty_interactive_shell_mode_echoes_input_and_is_reaped() {
    // Given: the interactive fixture with bounded piped stdin and stdout.
    let mut fixture_command = Command::from(command(ProcessFixtureMode::PtyInteractiveShell));
    fixture_command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .kill_on_drop(true);
    let mut child = fixture_command.spawn().unwrap();
    let pid = child.id().expect("PTY fixture PID");
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    let _ready = read_until(&mut lines, "fixture-shell-ready").await;
    // When: one shell line is written and flushed within the I/O deadline.
    let mut stdin = child.stdin.take().unwrap();
    tokio::time::timeout(IO_DEADLINE, async {
        stdin.write_all(b"fixture probe\n").await?;
        stdin.flush().await
    })
    .await
    .expect("PTY write deadline exceeded")
    .expect("write PTY fixture input");
    let echoed = read_until(&mut lines, "fixture-shell> ").await;
    drop(stdin);
    // Then: the fixture echoes the line and exits inside the shutdown deadline.
    assert_eq!(echoed.trim(), "fixture-shell> fixture probe");
    let status = close_and_wait(child).await;
    eprintln!("mode=pty-interactive-shell pid={pid} status={status}");
    assert!(status.success());
    assert!(!process_is_running(pid));
}
