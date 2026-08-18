// update_chat_stream_helpers.rs — Chat stream helper utilities (main.rs child module)
use super::{App, Arc, BlockBody, ChatMessage, Message, markdown, openrouter};
use iced::Task;

impl App {
    pub(crate) fn stop_stream(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.ui.compare_pending = false;
        self.status = "중지됨".into();
        self.maybe_update_title();
        self.save_session();
        Task::none()
    }
    pub(crate) fn abort_active_chat_stream(&mut self, keep_partial_assistant: bool) {
        let streaming_block_id = self.streaming_block_id;
        if let Some(h) = self.abort_handle.take() {
            h.abort();
        }
        if let Some(h) = self.mcp_abort_handle.take() {
            h.abort();
        }
        self.mcp_request_generation = self.mcp_request_generation.saturating_add(1);
        self.mcp_pending_results = 0;
        self.mcp_pending_call_ids.clear();
        self.generation_lookup_generation = self.generation_lookup_generation.saturating_add(1);
        self.compare_generation = self.compare_generation.saturating_add(1);
        if self.ui.compare_pending {
            self.discard_compare_blocks();
        }
        self.ui.compare_pending = false;
        if let Some(ai_id) = streaming_block_id {
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
            if keep_partial_assistant && !txt.is_empty() {
                self.fill_assistant_block(ai_id, &txt);
                Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(txt));
            } else {
                self.blocks.retain(|block| block.id != ai_id);
            }
        }
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.streaming_raw.clear();
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.ui.show_write_confirm = false;
        self.ui.expanded_confirm_idx = None;
        self.tool_round = 0;
        self.mid_stream_retries = 0;
    }
    pub(crate) fn next_stream_generation(&mut self) -> u64 {
        self.stream_generation = self.stream_generation.saturating_add(1);
        self.stream_generation
    }
    fn discard_compare_blocks(&mut self) {
        if let Some((openrouter_block_id, tabby_block_id)) = self.compare_block_ids.take() {
            self.blocks
                .retain(|block| block.id != openrouter_block_id && block.id != tabby_block_id);
        }
        self.clear_compare_result();
    }
    pub(crate) fn clear_compare_result(&mut self) {
        self.compare_old_text = None;
        self.compare_new_text = None;
        self.compare_block_ids = None;
    }
    pub(crate) fn fill_assistant_block(&mut self, block_id: u64, text: &str) {
        // Compare responses do not use the single-stream index, so resolve by
        // the request-owned block ID instead of relying on streaming state.
        if let Some(idx) = self.blocks.iter().position(|block| block.id == block_id)
            && let BlockBody::Assistant(content) = &mut self.blocks[idx].body
        {
            *content = iced::widget::text_editor::Content::with_text(text);
            self.blocks[idx].md_items = markdown::parse(text).collect();
        }
        if self.streaming_block_id == Some(block_id) {
            self.streaming_raw.clear();
        }
    }
    pub(crate) fn append_assistant_block_text(&mut self, block_id: u64, text: &str) {
        if text.is_empty() {
            return;
        }
        if let Some(idx) = self.streaming_block_idx
            && idx < self.blocks.len()
            && self.blocks[idx].id == block_id
            && let BlockBody::Assistant(_) = &self.blocks[idx].body
        {
            self.streaming_raw.push_str(text);
        }
    }
    pub(crate) fn kick_chat_stream(&mut self) -> Task<Message> {
        let Some(block_id) = self.streaming_block_id else {
            return Task::none();
        };
        if let Some(handle) = self.abort_handle.take() {
            handle.abort();
        }
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
        let stream_generation = self.next_stream_generation();
        // 기본 tool + MCP tool 합산
        let mut tool_defs = self.tool_definitions_for_selected_model();
        if !self.mcp_tools.is_empty()
            && let Some(arr) = tool_defs.as_mut().and_then(|v| v.as_array_mut())
        {
            for t in &self.mcp_tools {
                arr.push(t.to_openai_tool());
            }
        }
        let (task, handle) = Task::run(
            openrouter::chat_stream(base_url, api_key, model, messages, tool_defs),
            move |event| Message::ChatChunk {
                block_id,
                stream_generation,
                event,
            },
        )
        .abortable();
        self.abort_handle = Some(handle);
        task
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, PendingToolCall, ViewMode};
    use tempfile::TempDir;

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
        assert_eq!(app.blocks.len(), 1);
        assert_eq!(app.blocks[0].body.to_text(), "partial response");
        assert_eq!(app.conversation[0].role, "assistant");
        assert_eq!(
            app.conversation[0].content.as_deref(),
            Some("partial response")
        );

        let storage = TempDir::new().unwrap();
        assert!(app.save_session_at(storage.path()));
        let persisted = crate::session::load_all_at(Some(storage.path()));
        assert_eq!(persisted.sessions[0].blocks.len(), 1);
        assert_eq!(persisted.sessions[0].blocks[0].content, "partial response");
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
        assert!(app.blocks.is_empty());
    }

    #[test]
    fn abort_stream_removes_empty_assistant_placeholder() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks = vec![
            Block {
                id: 1,
                body: BlockBody::User("question".into()),
                view_mode: ViewMode::Rendered,
                md_items: Vec::new(),
                model: None,
                apply_candidates: Vec::new(),
            },
            assistant_block_with_text(2, ""),
        ];
        app.streaming_block_id = Some(2);
        app.streaming_block_idx = Some(1);

        app.abort_active_chat_stream(true);

        assert_eq!(app.blocks.len(), 1);
        assert_eq!(app.blocks[0].id, 1);
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

    #[test]
    fn abort_stream_cancels_pending_write_approval() {
        let (mut app, _) = App::new();
        app.streaming_block_id = Some(42);
        app.pending_write_calls = vec![PendingToolCall {
            id: "write-1".into(),
            name: "write_file".into(),
            arguments: "{}".into(),
        }];
        app.ui.show_write_confirm = true;
        app.ui.expanded_confirm_idx = Some(0);

        app.abort_active_chat_stream(true);

        assert!(app.pending_write_calls.is_empty());
        assert!(!app.ui.show_write_confirm);
        assert!(app.ui.expanded_confirm_idx.is_none());
    }

    #[test]
    fn abort_compare_removes_pending_blocks_without_persisting_placeholders() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks = vec![
            Block {
                id: 1,
                body: BlockBody::User("compare this".into()),
                view_mode: ViewMode::Rendered,
                md_items: Vec::new(),
                model: None,
                apply_candidates: Vec::new(),
            },
            assistant_block_with_text(2, "OpenRouter 응답 대기 중…"),
            assistant_block_with_text(3, "Tabby 응답 대기 중…"),
        ];
        app.ui.compare_pending = true;
        app.compare_block_ids = Some((2, 3));
        let (_task, handle) = iced::Task::<Message>::none().abortable();
        app.abort_handle = Some(handle);

        app.abort_active_chat_stream(true);

        assert_eq!(app.blocks.len(), 1);
        assert_eq!(app.blocks[0].id, 1);
        assert!(!app.ui.compare_pending);
        assert!(app.compare_block_ids.is_none());
        assert!(app.abort_handle.is_none());
        assert!(app.conversation.is_empty());
    }

    #[test]
    fn clear_compare_result_drops_diff_state() {
        let (mut app, _) = App::new();
        app.compare_old_text = Some("old".into());
        app.compare_new_text = Some("new".into());
        app.compare_block_ids = Some((1, 2));

        app.clear_compare_result();

        assert!(app.compare_old_text.is_none());
        assert!(app.compare_new_text.is_none());
        assert!(app.compare_block_ids.is_none());
    }
}
