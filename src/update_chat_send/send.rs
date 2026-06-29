use super::{
    AgentMode, App, Block, BlockBody, ChatMessage, Message, ViewMode, build_file_context,
    last_user_block_idx, openrouter, snap_to_end, truncate_after_last_user,
};
use iced::Task;
use iced::widget::text_editor;

impl App {
    pub(crate) fn regenerate_last(&mut self) -> Task<Message> {
        if self.streaming_block_id.is_some() {
            return Task::none();
        }
        if !self.conversation.iter().any(|m| m.role == "user") {
            return Task::none();
        }
        truncate_after_last_user(std::sync::Arc::make_mut(&mut self.conversation));
        let Some(idx) = last_user_block_idx(&self.blocks) else {
            return Task::none();
        };
        self.blocks.truncate(idx + 1);
        self.tool_round = 0;
        self.mid_stream_retries = 0;
        self.pending_tool_calls.clear();

        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };
        let model_opt = self.selected_model.clone();
        let model = model_opt.clone().unwrap_or_default();
        let messages = self.conversation.clone();

        let ai_id = self.next_id();
        self.blocks.push(Block {
            id: ai_id,
            body: BlockBody::Assistant(text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: model_opt,
            apply_candidates: Vec::new(),
        });
        self.streaming_block_id = Some(ai_id);
        self.streaming_block_idx = Some(self.blocks.len() - 1);
        self.status = "응답 다시 생성 중…".into();
        self.follow_bottom = true;

        let (chat_task, handle) = Task::run(
            openrouter::chat_stream(
                base_url,
                api_key,
                model,
                messages,
                self.tool_definitions_for_selected_model(),
            ),
            Message::ChatChunk,
        )
        .abortable();
        self.abort_handle = Some(handle);
        Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
    }
    pub(crate) fn send_message(&mut self) -> Task<Message> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return Task::none();
        }
        match text.as_str() {
            "/plan" => {
                self.agent_mode = AgentMode::Plan;
                self.input.clear();
                self.editor_content = text_editor::Content::new();
                self.status = format!("{} 모드", AgentMode::Plan.label());
                return Task::none();
            }
            "/build" => {
                self.agent_mode = AgentMode::Build;
                self.input.clear();
                self.editor_content = text_editor::Content::new();
                self.status = format!("{} 모드", AgentMode::Build.label());
                return Task::none();
            }
            s if s.starts_with('/') => {
                self.status = format!("알 수 없는 슬래시 명령: {s}");
                return Task::none();
            }
            _ => {}
        }
        if self.streaming_block_id.is_some() || self.compare_pending {
            return Task::none();
        }
        if self.compare_both {
            return self.compare_send_message(text);
        }
        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };
        let model_opt = self.selected_model.clone();
        let Some(model) = model_opt.clone() else {
            self.status = "모델을 먼저 선택해주세요.".into();
            return Task::none();
        };

        self.ensure_system_message();
        let user_msg = if self.attached_files.is_empty() {
            text.clone()
        } else {
            let ctx = build_file_context(&self.attached_files);
            format!("{ctx}\n\n{text}")
        };
        std::sync::Arc::make_mut(&mut self.conversation).push(ChatMessage::user(user_msg));
        self.attached_files.clear();
        self.close_mention();
        self.pending_tool_calls.clear();
        self.tool_round = 0;
        self.mid_stream_retries = 0;
        let messages = self.conversation.clone();

        let user_id = self.next_id();
        self.blocks.push(Block {
            id: user_id,
            body: BlockBody::User(text),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        let ai_id = self.next_id();
        self.blocks.push(Block {
            id: ai_id,
            body: BlockBody::Assistant(text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: model_opt,
            apply_candidates: Vec::new(),
        });
        self.streaming_block_id = Some(ai_id);
        self.streaming_block_idx = Some(self.blocks.len() - 1);
        self.input.clear();
        self.editor_content = text_editor::Content::new();
        self.status = "응답 생성 중…".into();
        self.follow_bottom = true;

        let (chat_task, handle) = Task::run(
            openrouter::chat_stream(
                base_url,
                api_key,
                model,
                messages,
                self.tool_definitions_for_selected_model(),
            ),
            Message::ChatChunk,
        )
        .abortable();
        self.abort_handle = Some(handle);
        Task::batch(vec![snap_to_end(self.stream_id.clone()), chat_task])
    }
}
