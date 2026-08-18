use std::io::{BufRead, BufReader, Write};
use std::process::Command;
use std::time::Duration;

use super::process_fixture_modes::{run_http_health, run_interactive_shell};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProcessFixtureMode {
    McpSuccess,
    McpSilent,
    McpMalformed,
    McpWrongId,
    McpRpcError,
    McpEarlyExit,
    IgnoreShutdown,
    RuntimeHttpHealth,
    PtyInteractiveShell,
}

impl ProcessFixtureMode {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::McpSuccess => "mcp-success",
            Self::McpSilent => "mcp-silent",
            Self::McpMalformed => "mcp-malformed",
            Self::McpWrongId => "mcp-wrong-id",
            Self::McpRpcError => "mcp-rpc-error",
            Self::McpEarlyExit => "mcp-early-exit",
            Self::IgnoreShutdown => "ignore-shutdown",
            Self::RuntimeHttpHealth => "runtime-http-health",
            Self::PtyInteractiveShell => "pty-interactive-shell",
        }
    }

    const fn test_name(self) -> &'static str {
        match self {
            Self::McpSuccess => "test_support::process_fixture::fixture_mcp_success",
            Self::McpSilent => "test_support::process_fixture::fixture_mcp_silent",
            Self::McpMalformed => "test_support::process_fixture::fixture_mcp_malformed",
            Self::McpWrongId => "test_support::process_fixture::fixture_mcp_wrong_id",
            Self::McpRpcError => "test_support::process_fixture::fixture_mcp_rpc_error",
            Self::McpEarlyExit => "test_support::process_fixture::fixture_mcp_early_exit",
            Self::IgnoreShutdown => "test_support::process_fixture::fixture_ignore_shutdown",
            Self::RuntimeHttpHealth => "test_support::process_fixture::fixture_runtime_http_health",
            Self::PtyInteractiveShell => {
                "test_support::process_fixture::fixture_pty_interactive_shell"
            }
        }
    }
}

pub(crate) fn command(mode: ProcessFixtureMode) -> Command {
    let executable = std::env::current_exe().expect("test executable path");
    let mut command = Command::new(executable);
    command.args([
        "--ignored",
        "--exact",
        mode.test_name(),
        "--nocapture",
        "--test-threads=1",
    ]);
    command
}

pub(crate) fn command_with_pid(mode: ProcessFixtureMode, pid_path: &std::path::Path) -> Command {
    let mut command = command(mode);
    command.env("CODEWARP_FIXTURE_PID_PATH", pid_path);
    command
}

pub(crate) fn pty_command_with_pid(
    mode: ProcessFixtureMode,
    pid_path: &std::path::Path,
) -> portable_pty::CommandBuilder {
    #[cfg(windows)]
    {
        if mode == ProcessFixtureMode::IgnoreShutdown {
            let mut command = portable_pty::CommandBuilder::new("ping.exe");
            command.args(["-t", "127.0.0.1"]);
            return command;
        }
        let launcher = pid_path.with_extension("cmd");
        let script = match mode {
            ProcessFixtureMode::PtyInteractiveShell => {
                "@echo off\r\necho FIXTURE_READY mode=pty-interactive-shell\r\necho fixture-shell-ready\r\n"
            }
            ProcessFixtureMode::IgnoreShutdown => unreachable!("handled above"),
            _ => panic!("unsupported Windows PTY fixture mode: {mode:?}"),
        };
        std::fs::write(&launcher, script).expect("write Windows PTY fixture launcher");
        let mut command = portable_pty::CommandBuilder::new("cmd.exe");
        command.args(["/D", "/Q", "/K"]);
        command.arg(launcher);
        command
    }
    #[cfg(not(windows))]
    if mode == ProcessFixtureMode::PtyInteractiveShell {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
        let mut command = portable_pty::CommandBuilder::new(&shell);
        command.args([
            "-c",
            "printf 'FIXTURE_READY mode=pty-interactive-shell\\nfixture-shell-ready\\n'; exec \"$0\" -i",
            &shell,
        ]);
        return command;
    }
    #[cfg(not(windows))]
    let executable = std::env::current_exe().expect("test executable path");
    #[cfg(not(windows))]
    let mut command = portable_pty::CommandBuilder::new(executable);
    #[cfg(not(windows))]
    command.args([
        "--ignored",
        "--exact",
        mode.test_name(),
        "--nocapture",
        "--test-threads=1",
    ]);
    #[cfg(not(windows))]
    command.env("CODEWARP_FIXTURE_PID_PATH", pid_path);
    #[cfg(not(windows))]
    command
}

pub(crate) fn command_line(mode: ProcessFixtureMode) -> String {
    let executable = std::env::current_exe().expect("test executable path");
    format!(
        "\"{}\" --ignored --exact {} --nocapture --test-threads=1",
        executable.display(),
        mode.test_name()
    )
}

fn run(mode: ProcessFixtureMode) {
    if let Some(path) = std::env::var_os("CODEWARP_FIXTURE_PID_PATH") {
        std::fs::write(path, std::process::id().to_string()).expect("write fixture PID");
    }
    println!(
        "FIXTURE_READY mode={} pid={}",
        mode.as_str(),
        std::process::id()
    );
    std::io::stdout().flush().expect("flush fixture ready line");

    match mode {
        ProcessFixtureMode::McpSuccess
        | ProcessFixtureMode::McpSilent
        | ProcessFixtureMode::McpMalformed
        | ProcessFixtureMode::McpWrongId
        | ProcessFixtureMode::McpRpcError
        | ProcessFixtureMode::McpEarlyExit => run_mcp(mode),
        ProcessFixtureMode::IgnoreShutdown => loop {
            std::thread::sleep(Duration::from_secs(60));
        },
        ProcessFixtureMode::RuntimeHttpHealth => run_http_health(),
        ProcessFixtureMode::PtyInteractiveShell => run_interactive_shell(),
    }
}

pub(crate) fn process_is_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
        use windows_sys::Win32::System::Threading::{
            GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        };

        // `tasklist` can be denied in constrained Windows runners. Query the
        // owned process handle instead so lifecycle assertions remain stable.
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle.is_null() {
            return false;
        }
        let mut exit_code = 0;
        let result = unsafe { GetExitCodeProcess(handle, &mut exit_code) } != 0;
        unsafe {
            CloseHandle(handle);
        }
        result && exit_code == STILL_ACTIVE as u32
    }
    #[cfg(target_os = "linux")]
    {
        std::path::Path::new("/proc").join(pid.to_string()).exists()
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        Command::new("ps")
            .args(["-p", &pid.to_string()])
            .status()
            .is_ok_and(|status| status.success())
    }
}

fn run_mcp(mode: ProcessFixtureMode) {
    for line in BufReader::new(std::io::stdin()).lines() {
        let line = line.expect("read MCP request");
        let request: serde_json::Value = serde_json::from_str(&line).expect("parse MCP request");
        let Some(id) = request.get("id").and_then(serde_json::Value::as_u64) else {
            continue;
        };

        match mode {
            ProcessFixtureMode::McpSuccess => write_mcp_success(&request, id),
            ProcessFixtureMode::McpSilent => {}
            ProcessFixtureMode::McpMalformed => write_line("not-json"),
            ProcessFixtureMode::McpWrongId => write_line(
                &serde_json::json!({"jsonrpc": "2.0", "id": id + 100, "result": {}}).to_string(),
            ),
            ProcessFixtureMode::McpRpcError => write_line(
                &serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {"code": -32603, "message": "fixture RPC error"}
                })
                .to_string(),
            ),
            ProcessFixtureMode::McpEarlyExit => return,
            ProcessFixtureMode::IgnoreShutdown
            | ProcessFixtureMode::RuntimeHttpHealth
            | ProcessFixtureMode::PtyInteractiveShell => unreachable!("non-MCP fixture mode"),
        }
    }
}

fn write_mcp_success(request: &serde_json::Value, id: u64) {
    let method = request
        .get("method")
        .and_then(serde_json::Value::as_str)
        .expect("MCP method");
    let result = match method {
        "initialize" => serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "serverInfo": {"name": "codewarp-fixture", "version": "1"}
        }),
        "tools/list" => serde_json::json!({"tools": [{
            "name": "fixture_echo",
            "description": "Echo fixture input",
            "inputSchema": {"type": "object"}
        }]}),
        "tools/call" => serde_json::json!({
            "content": [{"type": "text", "text": "fixture tool result"}]
        }),
        other => panic!("unexpected MCP method: {other}"),
    };
    write_line(&serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string());
}

fn write_line(line: &str) {
    let mut stdout = std::io::stdout().lock();
    writeln!(stdout, "{line}").expect("write fixture output");
    stdout.flush().expect("flush fixture output");
}

macro_rules! fixture_test {
    ($name:ident, $mode:ident) => {
        #[test]
        #[ignore]
        fn $name() {
            run(ProcessFixtureMode::$mode);
        }
    };
}

fixture_test!(fixture_mcp_success, McpSuccess);
fixture_test!(fixture_mcp_silent, McpSilent);
fixture_test!(fixture_mcp_malformed, McpMalformed);
fixture_test!(fixture_mcp_wrong_id, McpWrongId);
fixture_test!(fixture_mcp_rpc_error, McpRpcError);
fixture_test!(fixture_mcp_early_exit, McpEarlyExit);
fixture_test!(fixture_ignore_shutdown, IgnoreShutdown);
fixture_test!(fixture_runtime_http_health, RuntimeHttpHealth);
fixture_test!(fixture_pty_interactive_shell, PtyInteractiveShell);
