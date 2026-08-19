// update_chat_tools_writes.rs — Tool write-approval methods (main.rs child module)
use super::{
    App, Arc, Block, BlockBody, ChatMessage, MAX_TOOL_ROUNDS, Message, ToolExecutionResult,
    ViewMode, summarize_tool_result, tools,
};
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
    pub(crate) fn discard_write_call(&mut self, idx: usize) -> Task<Message> {
        if idx >= self.pending_write_calls.len() {
            return Task::none();
        }
        let tc = self.pending_write_calls.remove(idx);
        self.push_tool_result_block(&tc.name, "discarded", false);
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
    pub(crate) fn push_tool_result_block(&mut self, name: &str, summary: &str, success: bool) {
        let id = self.next_id();
        self.blocks.push(Block {
            id,
            body: BlockBody::ToolResult {
                name: name.to_owned(),
                summary: summary.to_owned(),
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
        self.ui.show_write_confirm = false;

        if approved {
            self.tool_execution_pending = true;
            self.status = "도구 실행 중…".into();
            return self.execute_approved_tools(calls);
        } else {
            self.tool_execution_pending = false;
            for tc in &calls {
                self.push_tool_result_block(&tc.name, "denied", false);
                Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 파일 쓰기를 거부했습니다.",
                ));
            }
            self.status = "사용자가 파일 쓰기를 거부했습니다".into();
        }

        if self.mcp_pending_results > 0 {
            self.status = "MCP tool 실행 중…".into();
            return Task::none();
        }
        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }

    fn execute_approved_tools(&mut self, calls: Vec<super::PendingToolCall>) -> Task<Message> {
        let cwd = self.cwd.clone();
        let generation = self.stream_generation;
        let task = Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    calls
                        .into_iter()
                        .map(|call| ToolExecutionResult {
                            id: call.id.clone(),
                            name: call.name.clone(),
                            arguments: call.arguments.clone(),
                            result: tools::dispatch(&call.name, &call.arguments, &cwd),
                        })
                        .collect::<Vec<_>>()
                })
                .await
                .map_err(|error| format!("도구 실행 작업 실패: {error}"))
            },
            move |result| Message::ApprovedToolsFinished { generation, result },
        );
        let (task, handle) = task.abortable();
        self.abort_handle = Some(handle);
        task
    }

    pub(crate) fn on_approved_tools_finished(
        &mut self,
        generation: u64,
        result: Result<Vec<ToolExecutionResult>, String>,
    ) -> Task<Message> {
        if generation != self.stream_generation || self.streaming_block_id.is_none() {
            return Task::none();
        }
        self.tool_execution_pending = false;
        self.abort_handle = None;

        let results = match result {
            Ok(results) => results,
            Err(error) => {
                let Some(block_id) = self.streaming_block_id else {
                    return Task::none();
                };
                return self.handle_chat_error(block_id, &error);
            }
        };
        let mut names = Vec::with_capacity(results.len());
        for tool in results {
            names.push(tool.name.clone());
            let (summary, success) =
                summarize_tool_result(&tool.name, &tool.arguments, &tool.result);
            self.push_tool_result_block(&tool.name, &summary, success);
            Arc::make_mut(&mut self.conversation)
                .push(ChatMessage::tool_result(&tool.id, tool.result));
        }
        self.status = format!("실행 완료: {}", names.join(", "));

        if self.mcp_pending_results > 0 {
            self.status = "MCP tool 실행 중…".into();
            return Task::none();
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
        if success
            && let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id)
            && let Some((_, applied)) = b.apply_candidates.get_mut(idx)
        {
            *applied = true;
        }
        let summary = if success {
            format!("{path} ({} bytes)", content.len())
        } else {
            format!("실패: {path}")
        };
        self.push_tool_result_block("apply", &summary, success);
        self.status = if success {
            format!("적용됨: {path}")
        } else {
            result
        };
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PendingToolCall;

    fn app_with_streaming_block() -> App {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.stream_generation = 4;
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app
    }

    #[test]
    fn approved_tool_execution_is_scheduled_off_the_update_path() {
        let mut app = app_with_streaming_block();
        app.pending_write_calls = vec![PendingToolCall {
            id: "call-1".into(),
            name: "run_command".into(),
            arguments: r#"{"command":"echo ready"}"#.into(),
        }];

        let _task = app.continue_after_writes(true);

        assert!(app.tool_execution_pending);
        assert!(app.conversation.is_empty());
        assert!(app.blocks[0].body.to_text().is_empty());
    }

    #[test]
    fn approved_tool_results_wait_for_mcp_before_starting_next_stream() {
        let mut app = app_with_streaming_block();
        app.tool_execution_pending = true;
        app.mcp_pending_results = 1;

        let _task = app.on_approved_tools_finished(
            4,
            Ok(vec![ToolExecutionResult {
                id: "call-1".into(),
                name: "run_command".into(),
                arguments: r#"{"command":"echo ready"}"#.into(),
                result: "ready".into(),
            }]),
        );

        assert!(!app.tool_execution_pending);
        assert_eq!(app.mcp_pending_results, 1);
        assert_eq!(app.tool_round, 0);
        assert_eq!(app.conversation.len(), 1);
        assert!(app.blocks.iter().any(|block| matches!(
            block.body,
            BlockBody::ToolResult { ref name, .. } if name == "run_command"
        )));
    }

    #[test]
    fn stale_approved_tool_results_are_ignored_after_stream_switch() {
        let mut app = app_with_streaming_block();
        app.tool_execution_pending = true;

        let _task = app.on_approved_tools_finished(
            3,
            Ok(vec![ToolExecutionResult {
                id: "late".into(),
                name: "run_command".into(),
                arguments: "{}".into(),
                result: "late result".into(),
            }]),
        );

        assert!(app.tool_execution_pending);
        assert!(app.conversation.is_empty());
        assert_eq!(app.blocks.len(), 1);
    }

    #[test]
    fn failed_approved_tool_execution_invalidates_mcp_results() {
        let mut app = app_with_streaming_block();
        app.tool_execution_pending = true;
        app.mcp_request_generation = 2;
        app.mcp_pending_results = 1;
        app.mcp_pending_call_ids.insert("mcp-call".into());

        let _task = app.on_approved_tools_finished(4, Err("worker failed".into()));

        assert!(!app.tool_execution_pending);
        assert_eq!(app.mcp_pending_results, 0);
        assert!(app.mcp_pending_call_ids.is_empty());
        assert!(app.blocks[0].body.to_text().contains("[ERROR]"));
    }
}
