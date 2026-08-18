// update_settings_mcp.rs — MCP server management methods (main.rs child module)
use super::{App, Message, mcp};
use iced::Task;

impl App {
    pub(crate) fn add_mcp_server(&mut self) -> Task<Message> {
        let name = self.mcp_input.name_input.trim().to_string();
        let command = self.mcp_input.command_input.trim().to_string();
        if name.is_empty() || command.is_empty() {
            self.status = "MCP 서버 이름과 명령을 모두 입력하세요.".into();
            return Task::none();
        }
        if self.mcp_servers.iter().any(|server| server.name == name) {
            self.status = format!("MCP 서버 이름이 이미 사용 중입니다: {name}");
            return Task::none();
        }
        let server = mcp::McpServer {
            name: name.clone(),
            command,
        };
        let generation = self.next_mcp_tool_load_generation(&name);
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
                    .map(|tools| (generation, name.clone(), tools))
                    .map_err(|e| (generation, name.clone(), format!("[{name}] {e}")))
            },
            |r| match r {
                Ok((generation, server_name, tools)) => Message::McpToolsLoaded {
                    generation,
                    server_name,
                    tools,
                },
                Err((generation, server_name, message)) => Message::McpToolsFailed {
                    generation,
                    server_name,
                    message,
                },
            },
        )
    }
    pub(crate) fn remove_mcp_server(&mut self, idx: usize) -> Task<Message> {
        if idx < self.mcp_servers.len() {
            let removed = self.mcp_servers.remove(idx);
            self.next_mcp_tool_load_generation(&removed.name);
            self.mcp_tools.retain(|t| t.server_name != removed.name);
            self.try_persist(mcp::save_servers(&self.mcp_servers), "MCP 서버 저장");
            self.status = format!("MCP 서버 제거됨: {}", removed.name);
        }
        Task::none()
    }
    pub(crate) fn on_mcp_tools_loaded(
        &mut self,
        generation: u64,
        server_name: &str,
        tools: Vec<mcp::McpTool>,
    ) -> Task<Message> {
        if self.mcp_tool_load_generations.get(server_name) != Some(&generation)
            || !self
                .mcp_servers
                .iter()
                .any(|server| server.name == server_name)
        {
            return Task::none();
        }
        self.mcp_tools.retain(|t| t.server_name != server_name);
        let count = tools.len();
        self.mcp_tools.extend(tools);
        self.status = format!("MCP [{server_name}] tool {count}개 로드 완료");
        Task::none()
    }
    pub(crate) fn on_mcp_tools_failed(
        &mut self,
        generation: u64,
        server_name: &str,
        message: &str,
    ) -> Task<Message> {
        if self.mcp_tool_load_generations.get(server_name) != Some(&generation)
            || !self
                .mcp_servers
                .iter()
                .any(|server| server.name == server_name)
        {
            return Task::none();
        }
        self.status = format!("MCP tool 로드 실패: {message}");
        Task::none()
    }

    pub(crate) fn next_mcp_tool_load_generation(&mut self, server_name: &str) -> u64 {
        let generation = self
            .mcp_tool_load_generations
            .entry(server_name.to_string())
            .or_default();
        *generation = generation.saturating_add(1);
        *generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_mcp_tools_loaded_removes_old_tools_and_updates_status() {
        let (mut app, _) = App::new();
        app.mcp_servers.push(mcp::McpServer {
            name: "fs".into(),
            command: "fixture".into(),
        });
        app.mcp_tool_load_generations.insert("fs".into(), 1);
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

        let _ = app.on_mcp_tools_loaded(1, "fs", new_tools);
        assert_eq!(app.mcp_tools.len(), 3);
        assert!(app.mcp_tools.iter().any(|t| t.server_name == "old-server"));
        assert!(app.mcp_tools.iter().any(|t| t.name == "write"));
        assert!(app.status.contains("MCP"));
    }

    #[test]
    fn on_mcp_tools_failed_shows_error_in_status() {
        let (mut app, _) = App::new();
        app.mcp_servers.push(mcp::McpServer {
            name: "fs".into(),
            command: "fixture".into(),
        });
        app.mcp_tool_load_generations.insert("fs".into(), 1);
        let _ = app.on_mcp_tools_failed(1, "fs", "connection refused");
        assert!(app.status.contains("MCP tool 로드 실패"));
        assert!(app.status.contains("connection refused"));
    }

    #[test]
    fn stale_mcp_tool_list_is_ignored_after_server_removal() {
        let (mut app, _) = App::new();
        app.mcp_servers.push(mcp::McpServer {
            name: "fs".into(),
            command: "fixture".into(),
        });
        app.mcp_tool_load_generations.insert("fs".into(), 1);
        app.mcp_servers.clear();
        app.mcp_tool_load_generations.insert("fs".into(), 2);

        let _ = app.on_mcp_tools_loaded(
            1,
            "fs",
            vec![mcp::McpTool {
                server_name: "fs".into(),
                name: "read".into(),
                description: String::new(),
                input_schema: serde_json::json!({}),
            }],
        );

        assert!(app.mcp_tools.is_empty());
    }

    #[test]
    fn duplicate_mcp_server_name_is_rejected() {
        let (mut app, _) = App::new();
        app.mcp_servers.push(mcp::McpServer {
            name: "fs".into(),
            command: "fixture".into(),
        });
        app.mcp_input.name_input = "fs".into();
        app.mcp_input.command_input = "another-fixture".into();

        let _ = app.add_mcp_server();

        assert_eq!(app.mcp_servers.len(), 1);
        assert!(app.status.contains("이미 사용 중"));
    }
}
