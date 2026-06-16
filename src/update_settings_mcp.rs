// update_settings_mcp.rs — MCP server management methods (main.rs child module)
use super::*;
use iced::Task;

impl App {
    pub(crate) fn add_mcp_server(&mut self) -> Task<Message> {
        let name = self.mcp_input.name_input.trim().to_string();
        let command = self.mcp_input.command_input.trim().to_string();
        if name.is_empty() || command.is_empty() {
            self.status = "MCP 서버 이름과 명령을 모두 입력하세요.".into();
            return Task::none();
        }
        let server = mcp::McpServer {
            name: name.clone(),
            command,
        };
        self.mcp_servers.push(server.clone());
        self.mcp_input.name_input.clear();
        self.mcp_input.command_input.clear();
        if let Err(e) = mcp::save_servers(&self.mcp_servers) {
            self.status = format!("MCP 저장 실패: {e}");
            return Task::none();
        }
        self.status = format!("MCP 서버 추가됨: {name} — tool 목록 로드 중…");
        Task::perform(
            async move {
                mcp::list_tools(&server)
                    .await
                    .map(|tools| (name.clone(), tools))
                    .map_err(|e| format!("[{name}] {e}"))
            },
            |r| match r {
                Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                Err(msg) => Message::McpToolsFailed(msg),
            },
        )
    }
    pub(crate) fn remove_mcp_server(&mut self, idx: usize) -> Task<Message> {
        if idx < self.mcp_servers.len() {
            let removed = self.mcp_servers.remove(idx);
            self.mcp_tools.retain(|t| t.server_name != removed.name);
            let _ = mcp::save_servers(&self.mcp_servers);
            self.status = format!("MCP 서버 제거됨: {}", removed.name);
        }
        Task::none()
    }
    pub(crate) fn on_mcp_tools_loaded(
        &mut self,
        server_name: String,
        tools: Vec<mcp::McpTool>,
    ) -> Task<Message> {
        self.mcp_tools.retain(|t| t.server_name != server_name);
        let count = tools.len();
        self.mcp_tools.extend(tools);
        self.status = format!("MCP [{server_name}] tool {count}개 로드 완료");
        Task::none()
    }
    pub(crate) fn on_mcp_tools_failed(&mut self, msg: String) -> Task<Message> {
        self.status = format!("MCP tool 로드 실패: {msg}");
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_mcp_tools_loaded_removes_old_tools_and_updates_status() {
        let (mut app, _) = App::new();
        app.mcp_tools.push(mcp::McpTool {
            server_name: "fs".into(),
            name: "read".into(),
            description: "".into(),
            input_schema: serde_json::json!({}),
        });
        app.mcp_tools.push(mcp::McpTool {
            server_name: "old-server".into(),
            name: "list".into(),
            description: "".into(),
            input_schema: serde_json::json!({}),
        });

        let new_tools = vec![
            mcp::McpTool {
                server_name: "fs".into(),
                name: "read".into(),
                description: "read file".into(),
                input_schema: serde_json::json!({}),
            },
            mcp::McpTool {
                server_name: "fs".into(),
                name: "write".into(),
                description: "write file".into(),
                input_schema: serde_json::json!({}),
            },
        ];

        let _ = app.on_mcp_tools_loaded("fs".into(), new_tools);
        assert_eq!(app.mcp_tools.len(), 3);
        assert!(app.mcp_tools.iter().any(|t| t.server_name == "old-server"));
        assert!(app.mcp_tools.iter().any(|t| t.name == "write"));
        assert!(app.status.contains("MCP"));
    }

    #[test]
    fn on_mcp_tools_failed_shows_error_in_status() {
        let (mut app, _) = App::new();
        let _ = app.on_mcp_tools_failed("connection refused".into());
        assert!(app.status.contains("MCP tool 로드 실패"));
        assert!(app.status.contains("connection refused"));
    }
}
