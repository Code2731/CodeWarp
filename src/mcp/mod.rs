// MCP (Model Context Protocol) stdio client.
// Spawns a server process and communicates over JSON-RPC 2.0 via stdin/stdout.

mod mcp_types;

pub(crate) use mcp_types::*;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// Parse a command string into executable + args.
/// Supports single/double quoted segments so paths with spaces survive parsing.
pub(crate) fn parse_command(command: &str) -> Result<Vec<String>, String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut chars = command.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\\' && !in_single {
            if let Some(next) = chars.peek().copied() {
                let escapable = next == '"' || next == '\'' || next == '\\' || next.is_whitespace();
                if escapable {
                    current.push(next);
                    let _ = chars.next();
                    continue;
                }
            }
            current.push(ch);
            continue;
        }

        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if in_single || in_double {
        return Err("명령 파싱 실패: 따옴표가 닫히지 않았습니다.".into());
    }

    if !current.is_empty() {
        args.push(current);
    }

    if args.is_empty() {
        return Err("빈 명령".into());
    }

    Ok(args)
}

/// Spawn server, initialize session, call method, return result, and then exit.
async fn rpc_call(
    command: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let parts = parse_command(command).map_err(|e| format!("MCP {e}"))?;
    let (program, args) = parts.split_first().ok_or("빈 명령")?;

    let mut child = Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("MCP 서버 시작 실패: {e}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| "MCP stdin pipe 열기 실패".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "MCP stdout pipe 열기 실패".to_string())?;
    let mut lines = BufReader::new(stdout).lines();

    // initialize
    send_json(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0", "id": 0,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "CodeWarp", "version": "0.2.0"}
            }
        }),
    )
    .await?;

    // wait initialize response (id=0)
    read_response(&mut lines, 0).await?;

    // initialized notification
    send_json(
        &mut stdin,
        &serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}}),
    )
    .await?;

    // request (id=1)
    send_json(
        &mut stdin,
        &serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}),
    )
    .await?;

    let result = read_response(&mut lines, 1).await?;

    // Close stdin and wait for child shutdown.
    drop(stdin);
    let _ = child.wait().await;

    Ok(result)
}

async fn send_json(
    stdin: &mut tokio::process::ChildStdin,
    val: &serde_json::Value,
) -> Result<(), String> {
    let mut line = serde_json::to_string(val).map_err(|e| format!("JSON 직렬화 실패: {e}"))?;
    line.push('\n');
    stdin
        .write_all(line.as_bytes())
        .await
        .map_err(|e| format!("stdin 쓰기 실패: {e}"))
}

async fn read_response(
    lines: &mut tokio::io::Lines<BufReader<tokio::process::ChildStdout>>,
    expected_id: u64,
) -> Result<serde_json::Value, String> {
    // Read up to 50 lines while skipping notifications/log lines.
    for _ in 0..50 {
        let line = lines
            .next_line()
            .await
            .map_err(|e| format!("stdout 읽기 실패: {e}"))?
            .ok_or("서버가 응답 없이 종료됨")?;

        let val: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue, // Ignore non-JSON lines.
        };

        if val.get("id").and_then(serde_json::Value::as_u64) != Some(expected_id) {
            continue;
        }

        if let Some(err) = val.get("error") {
            return Err(format!("MCP 오류: {err}"));
        }

        return val
            .get("result")
            .cloned()
            .ok_or_else(|| "result 필드 없음".to_string());
    }
    Err("MCP 응답 타임아웃 (50줄 초과)".into())
}

/// Spawn MCP server, call `tools/list`, return tool metadata.
pub(crate) async fn list_tools(server: &McpServer) -> Result<Vec<McpTool>, String> {
    let result = rpc_call(&server.command, "tools/list", serde_json::json!({})).await?;

    let arr = result
        .get("tools")
        .and_then(|t| t.as_array())
        .ok_or("tools 배열 없음")?;

    Ok(arr
        .iter()
        .filter_map(|t| {
            let name = t.get("name")?.as_str()?.to_string();
            let description = t
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("")
                .to_string();
            let input_schema = t
                .get("inputSchema")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
            Some(McpTool {
                server_name: server.name.clone(),
                name,
                description,
                input_schema,
            })
        })
        .collect())
}

/// Spawn MCP server, call `tools/call`, return textual content.
pub(crate) async fn call_tool(
    server: &McpServer,
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<String, String> {
    let result = rpc_call(
        &server.command,
        "tools/call",
        serde_json::json!({"name": tool_name, "arguments": arguments}),
    )
    .await?;

    Ok(extract_text_content(&result))
}

#[cfg(test)]
mod mcp_tests;
