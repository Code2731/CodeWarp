// update_chat_send.rs — Send/input update methods (main.rs child module)
use super::*;
use iced::widget::text_editor;
use iced::Task;

impl App {
    pub(crate) fn on_file_read_done(
        &mut self,
        path: std::path::PathBuf,
        content: String,
    ) -> Task<Message> {
        if !self.is_already_attached(&path) {
            self.attached_files.push((path, content));
            let current_total = self.total_attached_bytes();
            self.status = format!(
                "Attached ({} files): {}/{}",
                self.attached_files.len(),
                fmt_bytes(current_total),
                fmt_bytes(MAX_ATTACH_BYTES)
            );
        } else {
            self.status = format!("Already attached: {}", path.display());
        }
        Task::none()
    }
    pub(crate) fn on_input_changed(&mut self, value: String) -> Task<Message> {
        self.input = value;
        match extract_mention_query(&self.input) {
            Some(q) => {
                self.mention_query = q.to_string();
                self.mention_selected = 0;
                if !self.show_mention {
                    self.show_mention = true;
                    let cwd = self.cwd.clone();
                    return Task::perform(
                        collect_mention_candidates(cwd),
                        Message::MentionCandidatesLoaded,
                    );
                }
            }
            None => {
                if self.show_mention {
                    self.close_mention();
                }
            }
        }
        Task::none()
    }
    pub(crate) fn edit_last_user(&mut self) -> Task<Message> {
        if self.streaming_block_id.is_some() {
            return Task::none();
        }
        let Some(idx) = last_user_block_idx(&self.blocks) else {
            return Task::none();
        };
        let user_text = match &self.blocks[idx].body {
            BlockBody::User(s) => s.clone(),
            _ => return Task::none(),
        };
        self.blocks.truncate(idx);
        truncate_after_last_user(Arc::make_mut(&mut self.conversation));
        Arc::make_mut(&mut self.conversation).pop();
        self.tool_round = 0;
        self.pending_tool_calls.clear();
        self.input = user_text;
        self.status = "편집 모드 — 수정 후 Enter".into();
        Task::none()
    }
    pub(crate) fn regenerate_last(&mut self) -> Task<Message> {
        if self.streaming_block_id.is_some() {
            return Task::none();
        }
        if !self.conversation.iter().any(|m| m.role == "user") {
            return Task::none();
        }
        truncate_after_last_user(Arc::make_mut(&mut self.conversation));
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
        let model = self.selected_model.clone().unwrap_or_default();
        let messages = self.conversation.clone();

        let ai_id = self.next_id();
        self.blocks.push(Block {
            id: ai_id,
            body: BlockBody::Assistant(text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: self.selected_model.clone(),
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
                self.status = format!("{} 모드", AgentMode::Plan.label());
                return Task::none();
            }
            "/build" => {
                self.agent_mode = AgentMode::Build;
                self.input.clear();
                self.status = format!("{} 모드", AgentMode::Build.label());
                return Task::none();
            }
            s if s.starts_with('/') => {
                self.status = format!("알 수 없는 슬래시 명령: {}", s);
                return Task::none();
            }
            _ => {}
        }
        if self.streaming_block_id.is_some() || self.compare_pending {
            return Task::none();
        }
        if self.compare_both {
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
            Arc::make_mut(&mut self.conversation).push(ChatMessage::user(user_msg));
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
                body: BlockBody::Assistant(text_editor::Content::with_text(
                    "OpenRouter 응답 대기 중…",
                )),
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
            return Task::batch(vec![snap_to_end(self.stream_id.clone()), task]);
        }
        let (base_url, api_key) = match self.resolve_provider() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };
        let Some(model) = self.selected_model.clone() else {
            self.status = "모델을 먼저 선택해주세요.".into();
            return Task::none();
        };

        self.ensure_system_message();
        let user_msg = if !self.attached_files.is_empty() {
            let ctx = build_file_context(&self.attached_files);
            format!("{ctx}\n\n{text}")
        } else {
            text.clone()
        };
        Arc::make_mut(&mut self.conversation).push(ChatMessage::user(user_msg));
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
            model: self.selected_model.clone(),
            apply_candidates: Vec::new(),
        });
        self.streaming_block_id = Some(ai_id);
        self.streaming_block_idx = Some(self.blocks.len() - 1);
        self.input.clear();
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
