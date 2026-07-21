use super::process::{McpDeadlines, McpProcess, ProcessReceipt, RpcFailure};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

pub(super) async fn rpc_call_command(
    command: Command,
    method: &str,
    params: serde_json::Value,
    deadlines: McpDeadlines,
) -> Result<(serde_json::Value, ProcessReceipt), RpcFailure> {
    let (mut process, stdout) =
        McpProcess::spawn(command, deadlines).map_err(|message| RpcFailure::new(message, None))?;
    let mut lines = BufReader::new(stdout).lines();
    let operation = {
        let stdin = process
            .stdin_mut()
            .map_err(|message| RpcFailure::new(message, None))?;
        let mut rpc = RpcIo {
            stdin,
            lines: &mut lines,
            response_deadline: deadlines.response,
        };
        perform_rpc(&mut rpc, method, params).await
    };
    let cleanup = process.shutdown().await;

    match (operation, cleanup) {
        (Ok(result), Ok(receipt)) => Ok((result, receipt)),
        (Err(message), Ok(receipt)) => Err(RpcFailure::new(message, Some(receipt))),
        (Ok(_), Err(cleanup_error)) => Err(RpcFailure::new(cleanup_error, None)),
        (Err(message), Err(cleanup_error)) => Err(RpcFailure::new(
            format!("{message}; cleanup 실패: {cleanup_error}"),
            None,
        )),
    }
}

struct RpcIo<'a> {
    stdin: &'a mut tokio::process::ChildStdin,
    lines: &'a mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    response_deadline: std::time::Duration,
}

impl RpcIo<'_> {
    async fn request(
        &mut self,
        value: &serde_json::Value,
        expected_id: u64,
    ) -> Result<serde_json::Value, String> {
        send_json_bounded(self.stdin, value, self.response_deadline).await?;
        tokio::time::timeout(
            self.response_deadline,
            read_response(self.lines, expected_id),
        )
        .await
        .map_err(|_| {
            format!(
                "MCP response deadline exceeded after {}s (id={expected_id})",
                self.response_deadline.as_secs()
            )
        })?
    }

    async fn notify(&mut self, value: &serde_json::Value) -> Result<(), String> {
        send_json_bounded(self.stdin, value, self.response_deadline).await
    }
}

async fn perform_rpc(
    rpc: &mut RpcIo<'_>,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    rpc.request(
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "CodeWarp", "version": env!("CARGO_PKG_VERSION")}
            }
        }),
        0,
    )
    .await?;
    rpc.notify(
        &serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}),
    )
    .await?;
    rpc.request(
        &serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}),
        1,
    )
    .await
}

pub(super) async fn send_json_bounded<W>(
    writer: &mut W,
    val: &serde_json::Value,
    deadline: std::time::Duration,
) -> Result<(), String>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    tokio::time::timeout(deadline, send_json(writer, val))
        .await
        .map_err(|_| {
            format!(
                "MCP request flush deadline exceeded after {}s",
                deadline.as_secs()
            )
        })?
}

async fn send_json<W>(writer: &mut W, val: &serde_json::Value) -> Result<(), String>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut line = serde_json::to_string(val).map_err(|e| format!("JSON 직렬화 실패: {e}"))?;
    line.push('\n');
    writer
        .write_all(line.as_bytes())
        .await
        .map_err(|e| format!("stdin 쓰기 실패: {e}"))?;
    writer
        .flush()
        .await
        .map_err(|e| format!("stdin flush 실패: {e}"))
}

async fn read_response(
    lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    expected_id: u64,
) -> Result<serde_json::Value, String> {
    loop {
        let line = lines
            .next_line()
            .await
            .map_err(|e| format!("stdout 읽기 실패: {e}"))?
            .ok_or("서버가 응답 없이 종료됨")?;

        let val: serde_json::Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };

        if val.get("id").and_then(serde_json::Value::as_u64) != Some(expected_id) {
            continue;
        }
        if let Some(error) = val.get("error") {
            return Err(format!("MCP 오류: {error}"));
        }
        return val
            .get("result")
            .cloned()
            .ok_or("result 필드 없음".to_string());
    }
}
