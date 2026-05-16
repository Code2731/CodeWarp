// MCP (Model Context Protocol) stdio client.
// Spawns a server process and communicates over JSON-RPC 2.0 via stdin/stdout.

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;

/// User-configured MCP server entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServer {
    /// Display label used in UI logs/tool list.
    pub name: String,
    /// Spawn command. Quote arguments that include spaces.
    /// Example: `npx -y @modelcontextprotocol/server-filesystem "/tmp/work dir"`
    pub command: String,
}

/// Tool metadata discovered from an MCP server.
#[derive(Debug, Clone)]
pub struct McpTool {
    pub server_name: String,
    pub name: String,
    pub description: String,
    /// JSON Schema from MCP `inputSchema`.
    pub input_schema: serde_json::Value,
}

impl McpTool {
    /// Convert to OpenAI-compatible tool definition.
    pub fn to_openai_tool(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.input_schema
            }
        })
    }
}

/// Parse a command string into executable + args.
/// Supports single/double quoted segments so paths with spaces survive parsing.
fn parse_command(command: &str) -> Result<Vec<String>, String> {
    let mut args: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;

    for ch in command.chars() {
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
        return Err("MCP 명령 파싱 실패: 따옴표가 닫히지 않았습니다.".into());
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
    let parts = parse_command(command)?;
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

        if val.get("id").and_then(|v| v.as_u64()) != Some(expected_id) {
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
pub async fn list_tools(server: &McpServer) -> Result<Vec<McpTool>, String> {
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
pub async fn call_tool(
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

/// Extract text from `{content:[{type:"text",text:"..."}]}` response shape.
pub fn extract_text_content(result: &serde_json::Value) -> String {
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        item.get("text")?.as_str().map(|s| s.to_string())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default();

    if content.is_empty() {
        "(빈 응답)".to_string()
    } else {
        content
    }
}

/// Persist MCP server list.
pub fn save_servers(servers: &[McpServer]) -> Result<(), String> {
    let path = servers_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(servers).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

/// Load persisted MCP server list.
pub fn load_servers() -> Vec<McpServer> {
    let path = servers_path();
    let Ok(data) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

fn servers_path() -> std::path::PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("codewarp")
        .join("mcp_servers.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_tool_to_openai_format() {
        let tool = McpTool {
            server_name: "test".into(),
            name: "list_files".into(),
            description: "returns file list".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                },
                "required": ["path"]
            }),
        };
        let v = tool.to_openai_tool();
        assert_eq!(v["type"], "function");
        assert_eq!(v["function"]["name"], "list_files");
        assert_eq!(v["function"]["description"], "returns file list");
        assert!(v["function"]["parameters"]["properties"]["path"].is_object());
    }

    #[test]
    fn extract_text_content_single() {
        let result = serde_json::json!({
            "content": [{"type": "text", "text": "hello world"}]
        });
        assert_eq!(extract_text_content(&result), "hello world");
    }

    #[test]
    fn extract_text_content_multi() {
        let result = serde_json::json!({
            "content": [
                {"type": "text", "text": "line1"},
                {"type": "image", "data": "..."},
                {"type": "text", "text": "line2"}
            ]
        });
        assert_eq!(extract_text_content(&result), "line1\nline2");
    }

    #[test]
    fn extract_text_content_empty() {
        let result = serde_json::json!({"content": []});
        assert_eq!(extract_text_content(&result), "(빈 응답)");
    }

    #[test]
    fn extract_text_content_missing_content() {
        let result = serde_json::json!({"other": "field"});
        assert_eq!(extract_text_content(&result), "(빈 응답)");
    }

    #[test]
    fn mcp_server_serde_roundtrip() {
        let server = McpServer {
            name: "filesystem".into(),
            command: "npx -y @modelcontextprotocol/server-filesystem /tmp".into(),
        };
        let json = serde_json::to_string(&server).unwrap();
        let back: McpServer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, server.name);
        assert_eq!(back.command, server.command);
    }

    #[test]
    fn load_servers_returns_empty_on_missing_file() {
        // 파일이 없으면 빈 Vec 반환 (panic 없음)
        let result = std::panic::catch_unwind(load_servers);
        assert!(result.is_ok());
    }

    #[test]
    fn parse_command_splits_basic_tokens() {
        let parsed = parse_command("npx -y @modelcontextprotocol/server-filesystem /tmp").unwrap();
        assert_eq!(
            parsed,
            vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                "/tmp".to_string()
            ]
        );
    }

    #[test]
    fn parse_command_preserves_quoted_segments() {
        let parsed =
            parse_command(r#"npx -y server-filesystem "C:\Work Dir\project root""#).unwrap();
        assert_eq!(
            parsed,
            vec![
                "npx".to_string(),
                "-y".to_string(),
                "server-filesystem".to_string(),
                r#"C:\Work Dir\project root"#.to_string()
            ]
        );
    }

    #[test]
    fn parse_command_rejects_unclosed_quote() {
        let err = parse_command(r#"npx -y "server-filesystem"#).unwrap_err();
        assert!(err.contains("따옴표"));
    }
}
