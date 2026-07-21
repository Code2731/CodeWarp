// MCP (Model Context Protocol) stdio client.
// Spawns a server process and communicates over JSON-RPC 2.0 via stdin/stdout.

mod mcp_types;
mod process;
mod rpc;

pub(crate) use mcp_types::*;

use tokio::process::Command;

use process::PRODUCTION_DEADLINES;
use rpc::rpc_call_command;
#[cfg(test)]
use rpc::send_json_bounded;

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
    let mut process_command = Command::new(program);
    process_command.args(args);
    rpc_call_command(process_command, method, params, PRODUCTION_DEADLINES)
        .await
        .map(|(result, _receipt)| result)
        .map_err(|failure| failure.message)
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
mod mcp_lifecycle_tests;
#[cfg(test)]
mod mcp_tests;
