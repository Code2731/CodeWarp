use std::io::Read;
use std::path::Path;

const MAX_MCP_CONFIG_BYTES: u64 = 1024 * 1024;

/// User-configured MCP server entry.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct McpServer {
    pub name: String,
    pub command: String,
}

/// Tool metadata discovered from an MCP server.
#[derive(Debug, Clone)]
pub(crate) struct McpTool {
    pub server_name: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

impl McpTool {
    pub(crate) fn to_openai_tool(&self) -> serde_json::Value {
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

/// Extract text from `{content:[{type:"text",text:"..."}]}` response shape.
pub(crate) fn extract_text_content(result: &serde_json::Value) -> String {
    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    if item.get("type")?.as_str()? == "text" {
                        item.get("text")?
                            .as_str()
                            .map(std::string::ToString::to_string)
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

pub(crate) fn save_servers(servers: &[McpServer]) -> Result<(), String> {
    let path = servers_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(servers).map_err(|e| e.to_string())?;
    if json.len() as u64 > MAX_MCP_CONFIG_BYTES {
        return Err(format!(
            "MCP 설정이 {} bytes 제한을 초과했습니다",
            MAX_MCP_CONFIG_BYTES
        ));
    }
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

pub(crate) fn load_servers() -> Vec<McpServer> {
    load_servers_at(&servers_path())
}

fn load_servers_at(path: &Path) -> Vec<McpServer> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Vec::new();
    };
    if metadata.len() > MAX_MCP_CONFIG_BYTES {
        return Vec::new();
    }
    let Ok(file) = std::fs::File::open(path) else {
        return Vec::new();
    };
    let capacity = usize::try_from(metadata.len()).unwrap_or(0);
    let mut bytes = Vec::with_capacity(capacity);
    if file
        .take(MAX_MCP_CONFIG_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_MCP_CONFIG_BYTES
    {
        return Vec::new();
    }
    let Ok(data) = String::from_utf8(bytes) else {
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
        let result = std::panic::catch_unwind(load_servers);
        assert!(result.is_ok());
    }

    #[test]
    fn oversized_server_config_is_ignored_without_reading_contents() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("mcp_servers.json");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_MCP_CONFIG_BYTES + 1).unwrap();

        assert!(load_servers_at(&path).is_empty());
    }
}
