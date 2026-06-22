// update_chat_tools.rs — Tool execution update methods (main.rs child module)
use super::{mcp, openrouter, session, tools, App, Arc, ChatMessage, Message, MAX_TOOL_ROUNDS};
use iced::Task;

impl App {
    pub(crate) fn on_mcp_tool_result(
        &mut self,
        tool_call_id: &str,
        result: String,
    ) -> Task<Message> {
        Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(tool_call_id, result));
        self.tool_round += 1;
        self.kick_chat_stream()
    }
    pub(crate) fn on_generation_loaded(
        &mut self,
        result: Result<openrouter::GenerationData, String>,
    ) -> Task<Message> {
        if let Ok(data) = result {
            let cost = data.total_cost.unwrap_or(0.0);
            self.last_response_cost = Some(cost);
            let model_id = data.model.clone().unwrap_or_default();
            if !model_id.is_empty() {
                let entry = self.usage.by_model.entry(model_id).or_default();
                entry.total_cost += cost;
                entry.prompt_tokens += data.native_tokens_prompt.unwrap_or(0);
                entry.completion_tokens += data.native_tokens_completion.unwrap_or(0);
                entry.call_count += 1;
            }
            let _ = session::save_usage(&self.usage);
            return Task::done(Message::FetchAccount);
        }
        Task::none()
    }
    pub(crate) fn run_tool_round(&mut self, assistant_partial: String) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_tool_calls);

        let tool_calls_json = serde_json::Value::Array(
            calls
                .iter()
                .enumerate()
                .map(|(i, tc)| {
                    serde_json::json!({
                        "id": tc.id,
                        "type": "function",
                        "index": i,
                        "function": {
                            "name": tc.name,
                            "arguments": tc.arguments,
                        }
                    })
                })
                .collect(),
        );
        let mut assistant_msg = ChatMessage::assistant_tool_calls(tool_calls_json);
        if !assistant_partial.is_empty() {
            assistant_msg.content = Some(assistant_partial);
        }
        Arc::make_mut(&mut self.conversation).push(assistant_msg);

        let mcp_tool_names: std::collections::HashSet<String> =
            self.mcp_tools.iter().map(|t| t.name.clone()).collect();

        let (mcp_calls, local_calls): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|tc| mcp_tool_names.contains(&tc.name));

        if !mcp_calls.is_empty() {
            let (local_read, local_write): (Vec<_>, Vec<_>) = local_calls
                .into_iter()
                .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);
            for tc in &local_read {
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                Arc::make_mut(&mut self.conversation)
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            if !local_write.is_empty() {
                self.pending_write_calls = local_write;
                self.show_write_confirm = true;
            }

            let servers = self.mcp_servers.clone();
            let mcp_tools = self.mcp_tools.clone();
            let mut tasks = Vec::new();
            for tc in mcp_calls {
                let server = mcp_tools
                    .iter()
                    .find(|t| t.name == tc.name)
                    .and_then(|t| servers.iter().find(|s| s.name == t.server_name))
                    .cloned();
                let tool_name = tc.name.clone();
                let call_id = tc.id.clone();
                let args: serde_json::Value =
                    serde_json::from_str(&tc.arguments).unwrap_or_default();
                tasks.push(Task::perform(
                    async move {
                        match server {
                            Some(s) => mcp::call_tool(&s, &tool_name, args)
                                .await
                                .unwrap_or_else(|e| format!("[MCP 오류] {e}")),
                            None => "[MCP 오류] 서버 찾을 수 없음".into(),
                        }
                    },
                    move |result| Message::McpToolResult(call_id, result),
                ));
            }
            self.status = "MCP tool 실행 중…".into();
            return Task::batch(tasks);
        }

        let (read_calls, write_calls): (Vec<_>, Vec<_>) = local_calls
            .into_iter()
            .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);

        let mut names: Vec<String> = Vec::new();
        for tc in &read_calls {
            names.push(tc.name.clone());
            let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
            Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(&tc.id, result));
        }
        if !names.is_empty() {
            self.status = format!("도구 호출: {}", names.join(", "));
        }

        if !write_calls.is_empty() {
            self.pending_write_calls = write_calls;
            self.show_write_confirm = true;
            self.status = "파일 쓰기 승인 대기".into();
            return Task::none();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }
}
