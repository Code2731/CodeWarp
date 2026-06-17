// update_chat_compare — Compare mode update methods (main.rs child module)
use super::*;
use iced::widget::text_editor;
use iced::Task;

mod collect;
mod routes;
#[cfg(test)]
mod tests;

use collect::collect_chat_text;

impl App {
    pub(crate) fn on_compare_responses_loaded(
        &mut self,
        openrouter_block_id: u64,
        tabby_block_id: u64,
        openrouter_result: Result<String, String>,
        tabby_result: Result<String, String>,
    ) -> Task<Message> {
        if !self.compare_pending {
            return Task::none();
        }
        let openrouter_text = match openrouter_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[OpenRouter] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", openrouter::humanize_error(&e)),
        };
        let tabby_text = match tabby_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[Tabby] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", tabby::humanize_error(&e)),
        };
        self.fill_assistant_block(openrouter_block_id, openrouter_text.clone());
        self.fill_assistant_block(tabby_block_id, tabby_text.clone());
        std::sync::Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(format!(
            "[OpenRouter]\n{}\n\n[Tabby]\n{}",
            openrouter_text, tabby_text
        )));
        self.compare_pending = false;
        self.status = "Compare 응답 완료".into();
        self.maybe_update_title();
        self.save_session();
        if self.follow_bottom {
            snap_to_end(self.stream_id.clone())
        } else {
            Task::none()
        }
    }
    pub(crate) fn compare_send_message(&mut self, text: String) -> Task<Message> {
        let (openrouter_route, tabby_route) = match self.compare_routes() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };

        self.ensure_system_message();
        let user_msg = if !self.attached_files.is_empty() {
            let ctx = build_file_context(&self.attached_files);
            format!("{ctx}\n\n{text}")
        } else {
            text.clone()
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
        let openrouter_block_id = self.next_id();
        self.blocks.push(Block {
            id: openrouter_block_id,
            body: BlockBody::Assistant(text_editor::Content::with_text("OpenRouter 응답 대기 중…")),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: Some(format!(
                "{}: {}",
                openrouter_route.label, openrouter_route.model
            )),
            apply_candidates: Vec::new(),
        });
        let tabby_block_id = self.next_id();
        self.blocks.push(Block {
            id: tabby_block_id,
            body: BlockBody::Assistant(text_editor::Content::with_text("Tabby 응답 대기 중…")),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: Some(format!("{}: {}", tabby_route.label, tabby_route.model)),
            apply_candidates: Vec::new(),
        });

        self.input.clear();
        self.compare_pending = true;
        self.status = "Compare 응답 생성 중…".into();
        self.follow_bottom = true;

        let openrouter_messages = messages.clone();
        let tabby_messages = messages;
        let task = Task::perform(
            async move {
                let openrouter = collect_chat_text(
                    openrouter_route.base_url,
                    openrouter_route.api_key,
                    openrouter_route.model,
                    openrouter_messages,
                );
                let tabby = collect_chat_text(
                    tabby_route.base_url,
                    tabby_route.api_key,
                    tabby_route.model,
                    tabby_messages,
                );
                tokio::join!(openrouter, tabby)
            },
            move |(openrouter_result, tabby_result)| Message::CompareResponsesLoaded {
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            },
        );
        Task::batch(vec![snap_to_end(self.stream_id.clone()), task])
    }
}
