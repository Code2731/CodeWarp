// update_chat_stream.rs — Chat stream update methods (main.rs child module)
use super::*;
use iced::widget::text_editor;
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
                    Arc::make_mut(&mut self.conversation)
                        .push(ChatMessage::assistant(final_text.clone()));
                } else {
                    self.status =
                        "[WARN] 모델이 빈 응답을 반환했습니다. Provider/Runtime 로그를 확인해 주세요.".into();
                    if let Some(idx) = self.streaming_block_idx {
                        if idx < self.blocks.len() && self.blocks[idx].id == ai_id {
                            if let BlockBody::Assistant(content) = &mut self.blocks[idx].body {
                                if content.text().trim().is_empty() {
                                    *content =
                                        text_editor::Content::with_text("[WARN] empty response");
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
            }
            ChatEvent::Error(e) => {
                // Mid-stream retry: if tokens were emitted, retry silently
                if !self.streaming_raw.is_empty()
                    && self.mid_stream_retries < MAX_MID_STREAM_RETRIES
                    && !e.contains("OpenRouter 401")
                    && !e.contains("OpenRouter 402")
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
                            let full = format!("{}{}[ERROR] {}", final_text, prefix, e);
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
                let humanized = openrouter::humanize_error(&e);
                if e.contains("OpenRouter 401") || e.contains("OpenRouter 402") {
                    self.status = format!(
                        "[WARN] {} | Open Settings and check API key / credits",
                        humanized
                    );
                } else {
                    self.status = format!("[ERROR] {}", humanized);
                }
            }
        }
        if self.follow_bottom {
            snap_to_end(self.stream_id.clone())
        } else {
            Task::none()
        }
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
                    *content = text_editor::Content::with_text(&text);
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
