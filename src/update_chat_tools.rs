// update_chat_tools.rs — Tool execution & write-approval update methods (main.rs child module)
use super::*;
use iced::Task;

impl App {
    pub(crate) fn approve_pending_writes(&mut self) -> Task<Message> {
        self.ui.expanded_confirm_idx = None;
        self.continue_after_writes(true)
    }
    pub(crate) fn deny_pending_writes(&mut self) -> Task<Message> {
        self.ui.expanded_confirm_idx = None;
        self.continue_after_writes(false)
    }
    pub(crate) fn on_mcp_tool_result(
        &mut self,
        tool_call_id: String,
        result: String,
    ) -> Task<Message> {
        Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(&tool_call_id, result));
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
    pub(crate) fn discard_write_call(&mut self, idx: usize) -> Task<Message> {
        if idx >= self.pending_write_calls.len() {
            return Task::none();
        }
        let tc = self.pending_write_calls.remove(idx);
        self.push_tool_result_block(tc.name.clone(), "discarded".into(), false);
        Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(
            &tc.id,
            "[denied] 사용자가 이 도구 호출을 제외했습니다.",
        ));
        self.ui.expanded_confirm_idx = match self.ui.expanded_confirm_idx {
            Some(e) if e == idx => None,
            Some(e) if e > idx => Some(e - 1),
            other => other,
        };
        if self.pending_write_calls.is_empty() {
            return self.continue_after_writes(true);
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
            // 로컬 read-only는 MCP와 함께 즉시 처리, mutating은 승인 대기
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
    pub(crate) fn push_tool_result_block(&mut self, name: String, summary: String, success: bool) {
        let id = self.next_id();
        self.blocks.push(Block {
            id,
            body: BlockBody::ToolResult {
                name,
                summary,
                success,
            },
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
    }
    pub(crate) fn continue_after_writes(&mut self, approved: bool) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_write_calls);
        self.show_write_confirm = false;

        if approved {
            let mut names: Vec<String> = Vec::new();
            for tc in &calls {
                names.push(tc.name.clone());
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                let (summary, success) = summarize_tool_result(&tc.name, &tc.arguments, &result);
                self.push_tool_result_block(tc.name.clone(), summary, success);
                Arc::make_mut(&mut self.conversation)
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            self.status = format!("실행 완료: {}", names.join(", "));
        } else {
            for tc in &calls {
                self.push_tool_result_block(tc.name.clone(), "denied".into(), false);
                Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 파일 쓰기를 거부했습니다.",
                ));
            }
            self.status = "사용자가 파일 쓰기를 거부했습니다".into();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }
    pub(crate) fn apply_change(&mut self, block_id: u64, idx: usize) -> Task<Message> {
        let snapshot = self
            .blocks
            .iter()
            .find(|b| b.id == block_id)
            .and_then(|b| b.apply_candidates.get(idx))
            .filter(|(_, applied)| !*applied)
            .map(|(c, _)| (c.path.clone(), c.content.clone()));
        let Some((path, content)) = snapshot else {
            return Task::none();
        };
        let args_json = serde_json::json!({
            "path": path,
            "content": content,
        })
        .to_string();
        let result = tools::dispatch("write_file", &args_json, &self.cwd);
        let success = !result.contains("[error]");
        if success {
            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id) {
                if let Some((_, applied)) = b.apply_candidates.get_mut(idx) {
                    *applied = true;
                }
            }
        }
        let summary = if success {
            format!("{} ({} bytes)", path, content.len())
        } else {
            format!("실패: {}", path)
        };
        self.push_tool_result_block("apply".into(), summary, success);
        self.status = if success {
            format!("적용됨: {}", path)
        } else {
            result
        };
        Task::none()
    }
}
