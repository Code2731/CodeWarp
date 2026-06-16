// update_chat_stream_done.rs — Chat stream Done/Error handlers (main.rs child module)
use super::*;
use iced::widget::text_editor;
use iced::Task;

impl App {
    pub(crate) fn handle_chat_done(
        &mut self,
        ai_id: u64,
        finish_reason: Option<String>,
        generation_id: Option<String>,
    ) -> Task<Message> {
        let assistant_text = self.streaming_raw.clone();

        let has_tools = !self.pending_tool_calls.is_empty()
            && (finish_reason.as_deref() == Some("tool_calls") || finish_reason.is_none());

        if has_tools && self.tool_round < MAX_TOOL_ROUNDS {
            return self.run_tool_round(assistant_text);
        }

        if self.tool_round >= MAX_TOOL_ROUNDS && !self.pending_tool_calls.is_empty() {
            self.status = format!("최대 도구 라운드 {} 초과", MAX_TOOL_ROUNDS);
        } else {
            self.status = "준비됨".into();
        }

        let final_text = std::mem::take(&mut self.streaming_raw);
        if let Some(idx) = self.streaming_block_idx {
            if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                    *content = text_editor::Content::with_text(&final_text);
                    if !final_text.is_empty() {
                        self.blocks[idx].md_items = markdown::parse(&final_text).collect();
                    }
                }
            }
        }

        if !final_text.is_empty() {
            Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(final_text.clone()));
        } else {
            self.status =
                "[WARN] 모델이 빈 응답을 반환했습니다. Provider/Runtime 로그를 확인해 주세요."
                    .into();
            if let Some(idx) = self.streaming_block_idx {
                if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                    if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                        if content.text().trim().is_empty() {
                            *content = text_editor::Content::with_text("[WARN] empty response");
                        }
                    }
                }
            }
        }

        let candidates = parse_apply_candidates(&final_text);
        if !candidates.is_empty() {
            if let Some(idx) = self.streaming_block_idx {
                if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                    self.blocks[idx].apply_candidates =
                        candidates.into_iter().map(|c| (c, false)).collect();
                }
            }
        }
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.streaming_raw.clear();
        self.abort_handle = None;
        self.pending_tool_calls.clear();
        self.maybe_update_title();
        self.save_session();
        if let Some(id) = generation_id {
            if let Ok(api_key) = keystore::read_api_key() {
                return Task::perform(
                    openrouter::get_generation(api_key, id),
                    Message::GenerationLoaded,
                );
            }
        }
        Task::none()
    }

    pub(crate) fn handle_chat_error(&mut self, ai_id: u64, error: String) -> Task<Message> {
        if !self.streaming_raw.is_empty()
            && self.mid_stream_retries < MAX_MID_STREAM_RETRIES
            && !error.contains("OpenRouter 401")
            && !error.contains("OpenRouter 402")
        {
            self.mid_stream_retries += 1;
            self.streaming_raw.clear();
            if let Some(idx) = self.streaming_block_idx {
                if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                    if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                        *content = text_editor::Content::new();
                    }
                    self.blocks[idx].md_items.clear();
                }
            }
            self.pending_tool_calls.clear();
            self.status = format!(
                "재시도 중… ({}/{})",
                self.mid_stream_retries, MAX_MID_STREAM_RETRIES
            );
            return self.kick_chat_stream();
        }

        if let Some(idx) = self.streaming_block_idx {
            if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                    let prefix = if self.streaming_raw.is_empty() {
                        ""
                    } else {
                        "\n\n"
                    };
                    let final_text = std::mem::take(&mut self.streaming_raw);
                    let full = format!("{}{}[ERROR] {}", final_text, prefix, error);
                    *content = text_editor::Content::with_text(&full);
                    self.blocks[idx].md_items = markdown::parse(&full).collect();
                }
            }
        }
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.streaming_raw.clear();
        self.abort_handle = None;
        self.pending_tool_calls.clear();
        let humanized = openrouter::humanize_error(&error);
        if error.contains("OpenRouter 401") || error.contains("OpenRouter 402") {
            self.status = format!(
                "[WARN] {} | Open Settings and check API key / credits",
                humanized
            );
        } else {
            self.status = format!("[ERROR] {}", humanized);
        }
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_chunk_done_builds_content_from_streaming_raw() {
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
        app.streaming_raw = "hello world".into();

        let _ = app.update(Message::ChatChunk(ChatEvent::Done {
            finish_reason: Some("stop".into()),
            generation_id: None,
        }));

        assert_eq!(app.blocks[0].body.to_text(), "hello world");
        assert!(!app.blocks[0].md_items.is_empty());
        assert!(app.streaming_raw.is_empty());
        assert!(app.streaming_block_id.is_none());
        assert_eq!(app.conversation.len(), 1);
        assert_eq!(app.conversation[0].content.as_deref(), Some("hello world"));
    }

    #[test]
    fn chat_chunk_done_empty_streaming_raw_shows_warning() {
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
        app.streaming_raw = String::new();

        let _ = app.update(Message::ChatChunk(ChatEvent::Done {
            finish_reason: Some("stop".into()),
            generation_id: None,
        }));

        assert_eq!(app.blocks[0].body.to_text(), "[WARN] empty response");
        assert!(app.conversation.is_empty());
    }

    #[test]
    fn chat_chunk_error_appends_to_streaming_raw() {
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
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = MAX_MID_STREAM_RETRIES;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error("server error".into())));

        assert!(app.blocks[0].body.to_text().contains("partial text"));
        assert!(app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] server error"));
        assert!(app.streaming_block_id.is_none());
    }

    #[test]
    fn chat_chunk_error_empty_streaming_raw() {
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

        let _ = app.update(Message::ChatChunk(ChatEvent::Error("server error".into())));

        assert!(app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] server error"));
        assert!(app.streaming_block_id.is_none());
    }

    #[test]
    fn mid_stream_error_triggers_retry() {
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
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = 0;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error(
            "connection dropped".into(),
        )));

        assert_eq!(app.mid_stream_retries, 1);
        assert!(app.blocks[0].body.to_text().is_empty());
        assert!(app.streaming_raw.is_empty());
    }

    #[test]
    fn mid_stream_error_retries_exhausted() {
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
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = MAX_MID_STREAM_RETRIES;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error(
            "connection dropped".into(),
        )));

        assert!(app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] connection dropped"));
        assert_eq!(app.mid_stream_retries, MAX_MID_STREAM_RETRIES);
        assert!(app.streaming_block_id.is_none());
    }

    #[test]
    fn mid_stream_error_401_not_retried() {
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
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = 0;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error(
            "OpenRouter 401 unauthorized".into(),
        )));

        assert!(app.blocks[0].body.to_text().contains("[ERROR]"));
        assert_eq!(app.mid_stream_retries, 0);
    }
}
