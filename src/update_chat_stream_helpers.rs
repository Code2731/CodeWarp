// update_chat_stream_helpers.rs — Chat stream helper utilities (main.rs child module)
use super::*;
use iced::Task;

impl App {
    pub(crate) fn stop_stream(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.compare_pending = false;
        self.status = "중지됨".into();
        self.maybe_update_title();
        self.save_session();
        Task::none()
    }
    pub(crate) fn abort_active_chat_stream(&mut self, keep_partial_assistant: bool) {
        if let Some(h) = self.abort_handle.take() {
            h.abort();
        }
        self.compare_pending = false;
        if keep_partial_assistant {
            if let Some(ai_id) = self.streaming_block_id {
                let txt = if !self.streaming_raw.is_empty() {
                    std::mem::take(&mut self.streaming_raw)
                } else if let Some(idx) = self.streaming_block_idx {
                    if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                        self.blocks[idx].body.to_text()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                if !txt.is_empty() {
                    Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(txt));
                }
            }
        }
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.streaming_raw.clear();
        self.pending_tool_calls.clear();
        self.tool_round = 0;
        self.mid_stream_retries = 0;
    }
    pub(crate) fn fill_assistant_block(&mut self, block_id: u64, text: String) {
        if let Some(idx) = self.streaming_block_idx {
            if idx < self.blocks.len() && self.blocks[idx].id == block_id {
                if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                    *content = iced::widget::text_editor::Content::with_text(&text);
                    self.blocks[idx].md_items = markdown::parse(&text).collect();
                }
            }
        }
        if self.streaming_block_id == Some(block_id) {
            self.streaming_raw.clear();
        }
    }
    pub(crate) fn append_assistant_block_text(&mut self, block_id: u64, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(idx) = self.streaming_block_idx {
            if idx < self.blocks.len() && self.blocks[idx].id == block_id {
                if let BlockBody::Assistant(_) = &self.blocks[idx].body {
                    self.streaming_raw.push_str(text);
                }
            }
        }
    }
    pub(crate) fn kick_chat_stream(&mut self) -> Task<Message> {
        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                self.streaming_block_id = None;
                self.streaming_block_idx = None;
                self.streaming_raw.clear();
                return Task::none();
            }
        };
        let model = self.selected_model.clone().unwrap_or_default();
        let messages = self.conversation.clone();
        // 기본 tool + MCP tool 합산
        let mut tool_defs = self.tool_definitions_for_selected_model();
        if !self.mcp_tools.is_empty() {
            if let Some(arr) = tool_defs.as_mut().and_then(|v| v.as_array_mut()) {
                for t in &self.mcp_tools {
                    arr.push(t.to_openai_tool());
                }
            }
        }
        let (task, handle) = Task::run(
            openrouter::chat_stream(base_url, api_key, model, messages, tool_defs),
            Message::ChatChunk,
        )
        .abortable();
        self.abort_handle = Some(handle);
        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant_block_with_text(id: u64, text: &str) -> Block {
        Block {
            id,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(text)),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }

    #[test]
    fn abort_stream_keeps_partial_assistant_when_requested() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.tool_round = 3;
        app.pending_tool_calls = vec![PendingToolCall {
            id: "tc-1".into(),
            name: "read_file".into(),
            arguments: "{}".into(),
        }];
        app.blocks
            .push(assistant_block_with_text(42, "partial response"));

        app.abort_active_chat_stream(true);

        assert!(app.streaming_block_id.is_none());
        assert!(app.streaming_block_idx.is_none());
        assert!(app.pending_tool_calls.is_empty());
        assert_eq!(app.tool_round, 0);
        assert_eq!(app.conversation.len(), 1);
        assert_eq!(app.conversation[0].role, "assistant");
        assert_eq!(
            app.conversation[0].content.as_deref(),
            Some("partial response")
        );
    }

    #[test]
    fn abort_stream_drops_partial_assistant_when_not_requested() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(7);
        app.streaming_block_idx = Some(0);
        app.tool_round = 2;
        app.pending_tool_calls = vec![PendingToolCall {
            id: "tc-2".into(),
            name: "glob".into(),
            arguments: "{}".into(),
        }];
        app.blocks
            .push(assistant_block_with_text(7, "to be discarded"));

        app.abort_active_chat_stream(false);

        assert!(app.streaming_block_id.is_none());
        assert!(app.streaming_block_idx.is_none());
        assert!(app.pending_tool_calls.is_empty());
        assert_eq!(app.tool_round, 0);
        assert!(app.conversation.is_empty());
    }

    #[test]
    fn abort_stream_handles_missing_assistant_block() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(999);
        app.tool_round = 1;
        app.pending_tool_calls = vec![PendingToolCall {
            id: "tc-3".into(),
            name: "grep".into(),
            arguments: "{}".into(),
        }];

        app.abort_active_chat_stream(true);

        assert!(app.streaming_block_id.is_none());
        assert!(app.pending_tool_calls.is_empty());
        assert_eq!(app.tool_round, 0);
        assert!(app.conversation.is_empty());
    }
}
