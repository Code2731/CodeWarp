// update_chat_stream.rs — Chat stream event dispatcher (main.rs child module)
use super::{snap_to_end, App, ChatEvent, Message, PendingToolCall};
use iced::Task;

impl App {
    pub(crate) fn on_chat_chunk(&mut self, event: ChatEvent) -> Task<Message> {
        let Some(ai_id) = self.streaming_block_id else {
            return Task::none();
        };
        match event {
            ChatEvent::Token(t) => {
                self.append_assistant_block_text(ai_id, &t);
            }
            ChatEvent::ToolCallDelta {
                index,
                id,
                name,
                arguments,
            } => {
                let i = index as usize;
                while self.pending_tool_calls.len() <= i {
                    self.pending_tool_calls.push(PendingToolCall::default());
                }
                let tc = &mut self.pending_tool_calls[i];
                if let Some(id) = id {
                    tc.id = id;
                }
                if let Some(name) = name {
                    tc.name = name;
                }
                if let Some(args) = arguments {
                    tc.arguments.push_str(&args);
                }
            }
            ChatEvent::Done {
                finish_reason,
                generation_id,
            } => {
                return self.handle_chat_done(ai_id, finish_reason.as_ref(), generation_id);
            }
            ChatEvent::Error(e) => {
                return self.handle_chat_error(ai_id, &e);
            }
        }
        if self.follow_bottom {
            snap_to_end(self.stream_id.clone())
        } else {
            Task::none()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Block, BlockBody, ViewMode};
    use std::sync::Arc;

    #[test]
    fn chat_chunk_tokens_append_to_assistant_block_without_editor_focus() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ChatChunk(ChatEvent::Token("hel".into())));
        let _ = app.update(Message::ChatChunk(ChatEvent::Token("lo".into())));

        assert_eq!(app.streaming_raw, "hello");
    }

    #[test]
    fn chat_chunk_does_not_reparse_markdown_during_streaming() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
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

        let _ = app.update(Message::ChatChunk(ChatEvent::Token("**hello**".into())));
        assert!(
            app.blocks[0].md_items.is_empty(),
            "md_items should stay empty during streaming"
        );
        assert_eq!(app.streaming_raw, "**hello**");
    }
}
