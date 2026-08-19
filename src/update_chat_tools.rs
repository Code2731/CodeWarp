// update_chat_tools.rs — Tool execution update methods (main.rs child module)
use super::{
    App, Arc, ChatMessage, MAX_TOOL_ROUNDS, Message, PendingToolCall, mcp, openrouter, session,
    tools,
};
use iced::Task;

fn normalize_tool_call_ids(calls: &mut [PendingToolCall], stream_generation: u64, tool_round: u32) {
    let mut seen = std::collections::HashSet::new();
    for (index, call) in calls.iter_mut().enumerate() {
        let provided = call.id.trim();
        if !provided.is_empty() {
            let normalized = provided.to_string();
            if seen.insert(normalized.clone()) {
                call.id = normalized;
                continue;
            }
        }

        let base = format!("codewarp-call-{stream_generation}-{tool_round}-{index}");
        let mut generated = base.clone();
        let mut suffix = 1_u32;
        while seen.contains(&generated) {
            generated = format!("{base}-{suffix}");
            suffix = suffix.saturating_add(1);
        }
        call.id = generated.clone();
        seen.insert(generated);
    }
}

impl App {
    pub(crate) fn on_mcp_tool_result(
        &mut self,
        generation: u64,
        tool_call_id: &str,
        result: String,
    ) -> Task<Message> {
        if self.close_in_progress
            || generation != self.mcp_request_generation
            || self.mcp_pending_results == 0
            || !self.mcp_pending_call_ids.remove(tool_call_id)
        {
            return Task::none();
        }
        Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(tool_call_id, result));
        self.mcp_pending_results = self.mcp_pending_call_ids.len();
        if self.mcp_pending_results > 0 {
            return Task::none();
        }
        self.mcp_abort_handle.take();
        if !self.pending_write_calls.is_empty() || self.tool_execution_pending {
            return Task::none();
        }
        self.tool_round += 1;
        self.kick_chat_stream()
    }
    pub(crate) fn on_generation_loaded(
        &mut self,
        generation: u64,
        result: Result<openrouter::GenerationData, String>,
    ) -> Task<Message> {
        if generation != self.generation_lookup_generation {
            return Task::none();
        }
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
            self.try_persist(session::save_usage(&self.usage), "사용량 저장");
            return Task::done(Message::FetchAccount);
        }
        Task::none()
    }
    pub(crate) fn run_tool_round(&mut self, assistant_partial: String) -> Task<Message> {
        let mut calls = std::mem::take(&mut self.pending_tool_calls);
        normalize_tool_call_ids(&mut calls, self.stream_generation, self.tool_round);

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

        let mcp_tool_names: std::collections::HashSet<&str> =
            self.mcp_tools.iter().map(|t| t.name.as_str()).collect();

        let (mcp_calls, local_calls): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|tc| mcp_tool_names.contains(tc.name.as_str()));

        if !mcp_calls.is_empty() {
            self.mcp_request_generation = self.mcp_request_generation.saturating_add(1);
            let generation = self.mcp_request_generation;
            self.mcp_pending_call_ids = mcp_calls.iter().map(|tc| tc.id.clone()).collect();
            self.mcp_pending_results = self.mcp_pending_call_ids.len();
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
                self.ui.show_write_confirm = true;
            }

            let mut tasks = Vec::with_capacity(mcp_calls.len());
            for tc in mcp_calls {
                let server = self
                    .mcp_tools
                    .iter()
                    .find(|t| t.name == tc.name)
                    .and_then(|t| self.mcp_servers.iter().find(|s| s.name == t.server_name))
                    .cloned();
                let tool_name = tc.name;
                let call_id = tc.id;
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
                    move |result| Message::McpToolResult {
                        generation,
                        tool_call_id: call_id,
                        result,
                    },
                ));
            }
            self.status = "MCP tool 실행 중…".into();
            if let Some(handle) = self.mcp_abort_handle.take() {
                handle.abort();
            }
            let (task, handle) = Task::batch(tasks).abortable();
            self.mcp_abort_handle = Some(handle);
            return task;
        }

        let (read_calls, write_calls): (Vec<_>, Vec<_>) = local_calls
            .into_iter()
            .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);

        let mut names: Vec<String> = Vec::with_capacity(read_calls.len());
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
            self.ui.show_write_confirm = true;
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

#[cfg(test)]
mod mcp_close_tests {
    use super::*;

    #[test]
    fn duplicate_or_empty_tool_call_ids_are_normalized() {
        let mut calls = vec![
            PendingToolCall {
                id: String::new(),
                name: "read_file".into(),
                arguments: "{}".into(),
            },
            PendingToolCall {
                id: "provider-id".into(),
                name: "glob".into(),
                arguments: "{}".into(),
            },
            PendingToolCall {
                id: "provider-id".into(),
                name: "grep".into(),
                arguments: "{}".into(),
            },
        ];

        normalize_tool_call_ids(&mut calls, 4, 2);

        assert_eq!(calls[1].id, "provider-id");
        assert_eq!(calls[0].id, "codewarp-call-4-2-0");
        assert_eq!(calls[2].id, "codewarp-call-4-2-2");
        assert_eq!(
            calls
                .iter()
                .map(|call| call.id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len(),
            calls.len()
        );
    }

    #[test]
    fn late_mcp_result_is_ignored_after_close_starts() {
        // Given
        let (mut app, _) = App::new();
        app.close_in_progress = true;
        let conversation_len = app.conversation.len();

        // When
        let _task = app.on_mcp_tool_result(0, "late-call", "late-result".to_string());

        // Then
        assert_eq!(app.conversation.len(), conversation_len);
    }

    #[test]
    fn stale_mcp_result_is_ignored_after_stream_stop() {
        let (mut app, _) = App::new();
        app.mcp_request_generation = 2;
        app.streaming_block_id = Some(42);
        let conversation_len = app.conversation.len();

        let _task = app.on_mcp_tool_result(1, "late-call", "late-result".to_string());

        assert_eq!(app.conversation.len(), conversation_len);
        assert!(app.streaming_block_id.is_some());
    }

    #[test]
    fn mcp_results_wait_for_all_calls_before_starting_next_stream() {
        let (mut app, _) = App::new();
        app.mcp_request_generation = 1;
        app.mcp_pending_call_ids = ["first", "second"].into_iter().map(String::from).collect();
        app.mcp_pending_results = 2;

        let _ = app.on_mcp_tool_result(1, "first", "one".into());

        assert_eq!(app.mcp_pending_results, 1);
        assert_eq!(app.tool_round, 0);
        assert_eq!(app.conversation.len(), 1);

        let _ = app.on_mcp_tool_result(1, "second", "two".into());

        assert_eq!(app.mcp_pending_results, 0);
        assert_eq!(app.tool_round, 1);
        assert_eq!(app.conversation.len(), 2);
    }

    #[test]
    fn unknown_mcp_result_is_ignored_without_consuming_pending_work() {
        let (mut app, _) = App::new();
        app.mcp_request_generation = 1;
        app.mcp_pending_call_ids = ["expected".to_string()].into_iter().collect();
        app.mcp_pending_results = 1;
        let conversation_len = app.conversation.len();

        let _ = app.on_mcp_tool_result(1, "unexpected", "late-result".into());

        assert_eq!(app.conversation.len(), conversation_len);
        assert_eq!(app.mcp_pending_results, 1);
        assert_eq!(app.mcp_pending_call_ids.len(), 1);
    }

    #[test]
    fn duplicate_mcp_result_is_ignored_after_expected_result() {
        let (mut app, _) = App::new();
        app.mcp_request_generation = 1;
        app.mcp_pending_call_ids = ["expected".to_string()].into_iter().collect();
        app.mcp_pending_results = 1;

        let _ = app.on_mcp_tool_result(1, "expected", "first".into());
        let conversation_len = app.conversation.len();
        let _ = app.on_mcp_tool_result(1, "expected", "duplicate".into());

        assert_eq!(app.conversation.len(), conversation_len);
        assert_eq!(app.mcp_pending_results, 0);
        assert!(app.mcp_pending_call_ids.is_empty());
    }

    #[test]
    fn stale_generation_lookup_is_ignored() {
        let (mut app, _) = App::new();
        app.generation_lookup_generation = 2;
        app.last_response_cost = Some(1.25);

        let _ = app.on_generation_loaded(
            1,
            Ok(openrouter::GenerationData {
                model: Some("stale-model".into()),
                total_cost: Some(99.0),
                native_tokens_prompt: Some(100),
                native_tokens_completion: Some(200),
            }),
        );

        assert_eq!(app.last_response_cost, Some(1.25));
        assert!(!app.usage.by_model.contains_key("stale-model"));
    }
}
