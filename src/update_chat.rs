// update_chat.rs — Chat-related App update methods (main.rs child module)
use super::*;
use iced::widget::text_editor;
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
    pub(crate) fn new_chat(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.snapshot_current_to_inactive();
        self.blocks.clear();
        Arc::make_mut(&mut self.conversation).clear();
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.show_write_confirm = false;
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.tool_round = 0;
        self.next_block_id = 0;
        self.input.clear();
        self.ui.pending_delete_session = None;
        self.current_session_id = self.allocate_session_id();
        self.current_session_title = "새 채팅".into();
        self.status = "새 채팅".into();
        self.save_session();
        Task::none()
    }
    pub(crate) fn delete_session(&mut self, target_id: u64) -> Task<Message> {
        self.ui.pending_delete_session = None;
        if target_id == self.current_session_id {
            self.blocks.clear();
            Arc::make_mut(&mut self.conversation).clear();
            self.next_block_id = 0;
            self.current_session_id = self.allocate_session_id();
            self.current_session_title = "새 채팅".into();
        } else {
            self.inactive_sessions.retain(|s| s.id != target_id);
        }
        self.save_session();
        Task::none()
    }
    pub(crate) fn switch_session(&mut self, target_id: u64) -> Task<Message> {
        if target_id == self.current_session_id {
            return Task::none();
        }
        let Some(idx) = self
            .inactive_sessions
            .iter()
            .position(|s| s.id == target_id)
        else {
            return Task::none();
        };
        self.abort_active_chat_stream(true);
        self.snapshot_current_to_inactive();
        let target = self.inactive_sessions.remove(idx);
        self.current_session_id = target.id;
        self.current_session_title = target.title;
        self.conversation = target.conversation;
        self.next_block_id = target.next_block_id;
        self.blocks = target.blocks.into_iter().map(persisted_to_block).collect();
        self.current_scroll_y = target.scroll_y;
        self.pending_tool_calls.clear();
        self.pending_write_calls.clear();
        self.show_write_confirm = false;
        self.streaming_block_id = None;
        self.streaming_block_idx = None;
        self.tool_round = 0;
        self.input.clear();
        self.ui.pending_delete_session = None;
        self.status = "세션 전환됨".into();
        self.save_session();
        iced::widget::operation::scroll_to(
            self.stream_id.clone(),
            iced::widget::scrollable::AbsoluteOffset {
                x: 0.0,
                y: target.scroll_y,
            },
        )
    }
    pub(crate) fn on_stream_scrolled(
        &mut self,
        viewport: &iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        let rel = viewport.relative_offset();
        self.follow_bottom = rel.y > 0.95;
        self.current_scroll_y = viewport.absolute_offset().y;
        Task::none()
    }
    pub(crate) fn on_editor_action(
        &mut self,
        id: u64,
        action: iced::widget::text_editor::Action,
    ) -> Task<Message> {
        if action.is_edit() {
            return Task::none();
        }
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            if let BlockBody::Assistant(content) = &mut b.body {
                content.perform(action);
            }
        }
        Task::none()
    }
    pub(crate) fn toggle_block_view(&mut self, id: u64) -> Task<Message> {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            b.view_mode = match b.view_mode {
                ViewMode::Rendered => ViewMode::Raw,
                ViewMode::Raw => {
                    if let BlockBody::Assistant(content) = &b.body {
                        b.md_items = markdown::parse(&content.text()).collect();
                    }
                    ViewMode::Rendered
                }
            };
        }
        Task::none()
    }
    pub(crate) fn on_link_clicked(&mut self, uri: &markdown::Uri) -> Task<Message> {
        let url = uri.to_string();
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("javascript:") {
            self.status = "차단된 링크 스킴입니다.".into();
            return Task::none();
        }
        match webbrowser::open(&url) {
            Ok(_) => {
                self.status = format!("브라우저에서 열기: {}", url);
            }
            Err(e) => {
                self.status = format!("링크 열기 실패: {}", e);
            }
        }
        Task::none()
    }
    pub(crate) fn stop_stream(&mut self) -> Task<Message> {
        self.abort_active_chat_stream(true);
        self.compare_pending = false;
        self.status = "중지됨".into();
        self.maybe_update_title();
        self.save_session();
        Task::none()
    }
    pub(crate) fn copy_block(&self, id: u64) -> Task<Message> {
        if self.streaming_block_id == Some(id) && !self.streaming_raw.is_empty() {
            return iced::clipboard::write(self.streaming_raw.clone());
        }
        if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
            return iced::clipboard::write(b.body.to_text());
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
        Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(format!(
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
        if success {
            if let Some(b) = self.blocks.iter_mut().find(|b| b.id == block_id) {
                if let Some((_, applied)) = b.apply_candidates.get_mut(idx) {
                    *applied = true;
                }
            }
        }
        let summary = if success {
            format!("{} ({} bytes)", path, content.len())
        } else {
            format!("실패: {}", path)
        };
        self.push_tool_result_block("apply".into(), summary, success);
        self.status = if success {
            format!("적용됨: {}", path)
        } else {
            result
        };
        Task::none()
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
    pub(crate) fn on_mcp_tool_result(
        &mut self,
        tool_call_id: String,
        result: String,
    ) -> Task<Message> {
        Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(&tool_call_id, result));
        self.tool_round += 1;
        self.kick_chat_stream()
    }
    pub(crate) fn on_generation_loaded(
        &mut self,
        result: Result<openrouter::GenerationData, String>,
    ) -> Task<Message> {
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
            let _ = session::save_usage(&self.usage);
            return Task::done(Message::FetchAccount);
        }
        Task::none()
    }
    pub(crate) fn discard_write_call(&mut self, idx: usize) -> Task<Message> {
        if idx >= self.pending_write_calls.len() {
            return Task::none();
        }
        let tc = self.pending_write_calls.remove(idx);
        self.push_tool_result_block(tc.name.clone(), "discarded".into(), false);
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
    pub(crate) fn snapshot_current_to_inactive(&mut self) {
        if self.conversation.is_empty() && self.blocks.is_empty() {
            return; // 빈 세션은 보관 X
        }
        let sid = self.streaming_block_id;
        let raw = &self.streaming_raw;
        let blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .filter_map(|b| {
                let content = if sid == Some(b.id) {
                    raw.clone()
                } else {
                    b.body.to_text()
                };
                match &b.body {
                    BlockBody::User(_) | BlockBody::Assistant(_) => Some(session::PersistedBlock {
                        id: b.id,
                        role: if matches!(&b.body, BlockBody::User(_)) {
                            "user".into()
                        } else {
                            "assistant".into()
                        },
                        content,
                        model: b.model.clone().unwrap_or_default(),
                    }),
                    BlockBody::ToolResult { .. } => None,
                }
            })
            .collect();
        let snap = InactiveSession {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: self.conversation.clone(),
            blocks: blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        };
        if let Some(idx) = self.inactive_sessions.iter().position(|s| s.id == snap.id) {
            self.inactive_sessions[idx] = snap;
        } else {
            self.inactive_sessions.push(snap);
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
    pub(crate) fn run_tool_round(&mut self, assistant_partial: String) -> Task<Message> {
        let calls = std::mem::take(&mut self.pending_tool_calls);

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

        let mcp_tool_names: std::collections::HashSet<String> =
            self.mcp_tools.iter().map(|t| t.name.clone()).collect();

        let (mcp_calls, local_calls): (Vec<_>, Vec<_>) = calls
            .into_iter()
            .partition(|tc| mcp_tool_names.contains(&tc.name));

        if !mcp_calls.is_empty() {
            // 로컬 read-only는 MCP와 함께 즉시 처리, mutating은 승인 대기
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
                self.show_write_confirm = true;
            }

            let servers = self.mcp_servers.clone();
            let mcp_tools = self.mcp_tools.clone();
            let mut tasks = Vec::new();
            for tc in mcp_calls {
                let server = mcp_tools
                    .iter()
                    .find(|t| t.name == tc.name)
                    .and_then(|t| servers.iter().find(|s| s.name == t.server_name))
                    .cloned();
                let tool_name = tc.name.clone();
                let call_id = tc.id.clone();
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
                    move |result| Message::McpToolResult(call_id, result),
                ));
            }
            self.status = "MCP tool 실행 중…".into();
            return Task::batch(tasks);
        }

        let (read_calls, write_calls): (Vec<_>, Vec<_>) = local_calls
            .into_iter()
            .partition(|tc| tools::tool_kind(&tc.name) == tools::ToolKind::ReadOnly);

        let mut names: Vec<String> = Vec::new();
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
            self.show_write_confirm = true;
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
    pub(crate) fn push_tool_result_block(&mut self, name: String, summary: String, success: bool) {
        let id = self.next_id();
        self.blocks.push(Block {
            id,
            body: BlockBody::ToolResult {
                name,
                summary,
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
        self.show_write_confirm = false;

        if approved {
            let mut names: Vec<String> = Vec::new();
            for tc in &calls {
                names.push(tc.name.clone());
                let result = tools::dispatch(&tc.name, &tc.arguments, &self.cwd);
                let (summary, success) = summarize_tool_result(&tc.name, &tc.arguments, &result);
                self.push_tool_result_block(tc.name.clone(), summary, success);
                Arc::make_mut(&mut self.conversation)
                    .push(ChatMessage::tool_result(&tc.id, result));
            }
            self.status = format!("실행 완료: {}", names.join(", "));
        } else {
            for tc in &calls {
                self.push_tool_result_block(tc.name.clone(), "denied".into(), false);
                Arc::make_mut(&mut self.conversation).push(ChatMessage::tool_result(
                    &tc.id,
                    "[denied] 사용자가 파일 쓰기를 거부했습니다.",
                ));
            }
            self.status = "사용자가 파일 쓰기를 거부했습니다".into();
        }

        self.tool_round += 1;
        self.status = format!(
            "응답 생성 중… (도구 라운드 {}/{})",
            self.tool_round, MAX_TOOL_ROUNDS
        );
        self.kick_chat_stream()
    }
    pub(crate) fn compare_routes(&self) -> Result<(ChatRoute, ChatRoute), String> {
        let selected = self.selected_option();
        let openrouter_model = selected
            .filter(|o| o.provider == LlmProvider::OpenRouter)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenRouter)
            })
            .ok_or_else(|| "Compare 모드: OpenRouter 모델이 없습니다. OpenRouter 키/모델 목록을 먼저 불러와 주세요.".to_string())?;
        let tabby_model = selected
            .filter(|o| o.provider == LlmProvider::OpenAICompat)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenAICompat)
            })
            .ok_or_else(|| "Compare 모드: Tabby 모델이 없습니다. Provider 연결 테스트로 Tabby 모델을 먼저 불러와 주세요.".to_string())?;

        let openrouter_key = keystore::read_api_key()?;
        let tabby_base = if self.tabby_url_input.trim().is_empty() {
            keystore::read_tabby_base_url()
        } else {
            Some(self.tabby_url_input.clone())
        }
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Compare 모드: Tabby URL 미설정".to_string())?;
        let tabby_token = if self.tabby_token_input.trim().is_empty() {
            keystore::read_tabby_token()
        } else {
            Some(self.tabby_token_input.clone())
        }
        .filter(|s| !s.trim().is_empty());

        Ok((
            ChatRoute {
                label: "OpenRouter".into(),
                base_url: openrouter::BASE_URL.to_string(),
                api_key: Some(openrouter_key),
                model: openrouter_model.id.clone(),
            },
            ChatRoute {
                label: if tabby_model.provider_label.trim().is_empty() {
                    "Local".into()
                } else {
                    tabby_model.provider_label.trim().to_string()
                },
                base_url: tabby::chat_base(&tabby_base),
                api_key: tabby_token,
                model: tabby_model.id.clone(),
            },
        ))
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
    pub(crate) fn save_session(&self) {
        let sid = self.streaming_block_id;
        let raw = &self.streaming_raw;
        let current_blocks_persisted: Vec<session::PersistedBlock> = self
            .blocks
            .iter()
            .filter_map(|b| {
                let content = if sid == Some(b.id) {
                    raw.clone()
                } else {
                    b.body.to_text()
                };
                match &b.body {
                    BlockBody::User(_) | BlockBody::Assistant(_) => Some(session::PersistedBlock {
                        id: b.id,
                        role: if matches!(&b.body, BlockBody::User(_)) {
                            "user".into()
                        } else {
                            "assistant".into()
                        },
                        content,
                        model: b.model.clone().unwrap_or_default(),
                    }),
                    BlockBody::ToolResult { .. } => None,
                }
            })
            .collect();

        let mut sessions: Vec<session::PersistedSessionData> = self
            .inactive_sessions
            .iter()
            .map(|s| session::PersistedSessionData {
                id: s.id,
                title: s.title.clone(),
                conversation: (*s.conversation).clone(),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();
        sessions.push(session::PersistedSessionData {
            id: self.current_session_id,
            title: self.current_session_title.clone(),
            conversation: (*self.conversation).clone(),
            blocks: current_blocks_persisted,
            next_block_id: self.next_block_id,
            scroll_y: self.current_scroll_y,
        });

        let active_idx = sessions
            .iter()
            .position(|s| s.id == self.current_session_id)
            .unwrap_or(sessions.len() - 1);

        let p = session::PersistedAllSessions {
            sessions,
            active_idx,
        };
        let _ = session::save_all(&p);
    }
    pub(crate) fn maybe_update_title(&mut self) {
        if self.current_session_title.is_empty()
            || self.current_session_title.starts_with("새 채팅")
        {
            if let Some(first_user) = self
                .conversation
                .iter()
                .find(|m| m.role == "user")
                .and_then(|m| m.content.as_ref())
            {
                let snippet: String = first_user.chars().take(30).collect();
                self.current_session_title = snippet;
            }
        }
    }
    pub(crate) fn allocate_session_id(&mut self) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }
    pub(crate) fn ensure_system_message(&mut self) {
        let mode_block = match self.agent_mode {
            AgentMode::Plan => {
                "현재 모드: Plan (분석/계획 전용)\n\
                Plan 모드에서는 read_file/glob/grep으로 코드를 조사하고 변경 계획만 \
                제시하세요. 실제 파일 변경이나 명령 실행은 Build 모드에서만 가능하므로, \
                계획에 '필요한 변경'을 명확히 적고 사용자가 Build로 전환하기를 기다리세요.\n\n"
            }
            AgentMode::Build => {
                "현재 모드: Build (실행 가능)\n\
                Build 모드에서는 write_file/run_command를 사용해 실제 변경을 적용할 수 \
                있습니다. 단, 두 도구 모두 사용자 승인을 거치므로 부담 없이 호출하세요.\n\n"
            }
        };
        let prompt = format!(
            "당신은 CodeWarp의 코딩 어시스턴트입니다.\n\n\
            작업 디렉토리: '{}'\n\n\
            {}\
            사용 가능한 도구 (적극적으로 호출하세요):\n\
            - read_file(path): 파일 내용 읽기 (즉시 실행)\n\
            - write_file(path, content): 파일 작성/덮어쓰기 (Build 모드 + 사용자 승인)\n\
            - run_command(command): 셸 명령 실행 (Build 모드 + 사용자 승인)\n\
            - glob(pattern): 패턴 매칭 파일 리스트 (예: '**/*.rs', 'examples/**/*')\n\
            - grep(pattern): 정규식으로 모든 파일 검색\n\n\
            규칙:\n\
            1. 파일 시스템을 살펴봐야 할 때는 '확인하겠습니다' 같은 말 없이 즉시 도구를 호출하세요.\n\
            2. 새 파일을 만들기 전에 glob으로 기존 구조를 먼저 확인하세요.\n\
            3. 모든 path 인자는 작업 디렉토리 기준 상대 경로 (절대 경로 거부).\n\
            4. 도구 결과를 받은 뒤 그것을 근거로 한국어로 답하세요.\n\
            5. **마크다운 형식 제약** (한국어 폰트 한계): italic(*text* 또는 _text_)은 \
            사용하지 마세요. 강조는 오직 **굵게**만 사용. 별표 한 개로 감싸지 말고, \
            정말 강조가 필요하면 두 개로 감싸세요.\n\
            6. **Apply 가능한 코드 블록**: 사용자가 그대로 파일에 적용할 수 있도록, \
            새 파일/덮어쓸 파일의 코드 블록은 첫 줄에 다음 주석을 포함하세요:\n\
            - Rust/JS/C 계열: `// path: 상대경로`\n\
            - Python/shell/yaml: `# path: 상대경로`\n\
            예) ```rust\\n// path: src/foo.rs\\nfn main() {{}}\\n```\n\
            그러면 코드 블록 옆에 'Apply' 버튼이 노출되어 사용자가 한 번에 적용할 수 있습니다. \
            단순 예시 코드(개념 설명용)에는 path 주석을 넣지 마세요.",
            self.cwd.display(),
            mode_block,
        );
        if let Some(first) = Arc::make_mut(&mut self.conversation).first_mut() {
            if first.role == "system" {
                first.content = Some(prompt);
                return;
            }
        }
        Arc::make_mut(&mut self.conversation).insert(
            0,
            ChatMessage {
                role: "system".into(),
                content: Some(prompt),
                ..Default::default()
            },
        );
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

    fn or_opt(id: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenRouter,
            provider_label: String::new(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    fn oai_opt(id: &str, label: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: label.into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
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
            "md_items should stay empty during streaming (F16 perf fix)"
        );
        assert_eq!(app.streaming_raw, "**hello**");
    }

    #[test]
    fn toggle_block_view_to_rendered_triggers_markdown_parse() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        let id = 42;
        app.blocks.push(Block {
            id,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(
                "**bold** text",
            )),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ToggleBlockView(id));
        assert_eq!(app.blocks[0].view_mode, ViewMode::Rendered);
        assert!(
            !app.blocks[0].md_items.is_empty(),
            "md_items should be populated after toggling to Rendered"
        );
    }

    #[test]
    fn toggle_block_view_to_raw_clears_no_md_items() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        let id = 42;
        app.blocks.push(Block {
            id,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text("hello")),
            view_mode: ViewMode::Rendered,
            md_items: vec![], // pretend it was parsed
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ToggleBlockView(id));
        assert_eq!(app.blocks[0].view_mode, ViewMode::Raw);
    }

    #[test]
    fn on_mcp_tools_loaded_removes_old_tools_and_updates_status() {
        let (mut app, _) = App::new();
        app.mcp_tools.push(mcp::McpTool {
            server_name: "fs".into(),
            name: "read".into(),
            description: "".into(),
            input_schema: serde_json::json!({}),
        });
        app.mcp_tools.push(mcp::McpTool {
            server_name: "old-server".into(),
            name: "list".into(),
            description: "".into(),
            input_schema: serde_json::json!({}),
        });

        let new_tools = vec![
            mcp::McpTool {
                server_name: "fs".into(),
                name: "read".into(),
                description: "read file".into(),
                input_schema: serde_json::json!({}),
            },
            mcp::McpTool {
                server_name: "fs".into(),
                name: "write".into(),
                description: "write file".into(),
                input_schema: serde_json::json!({}),
            },
        ];

        let _ = app.on_mcp_tools_loaded("fs".into(), new_tools);
        assert_eq!(app.mcp_tools.len(), 3);
        assert!(app.mcp_tools.iter().any(|t| t.server_name == "old-server"));
        assert!(app.mcp_tools.iter().any(|t| t.name == "write"));
        assert!(app.status.contains("MCP"));
    }

    #[test]
    fn on_mcp_tools_failed_shows_error_in_status() {
        let (mut app, _) = App::new();
        let _ = app.on_mcp_tools_failed("connection refused".into());
        assert!(app.status.contains("MCP tool 로드 실패"));
        assert!(app.status.contains("connection refused"));
    }

    #[test]
    fn compare_mode_send_requires_registered_providers() {
        let (mut app, _) = App::new();
        app.compare_both = true;
        app.input = "compare this".into();
        app.selected_model = None;
        app.model_options.clear();
        let before_blocks = app.blocks.len();

        let _ = app.update(Message::Send);

        assert!(
            app.status.contains("Compare 모드: OpenRouter 모델"),
            "got: {}",
            app.status
        );
        assert_eq!(app.blocks.len(), before_blocks);
    }

    #[test]
    fn send_message_returns_early_when_streaming() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.input = "hello".into();
        let before = app.conversation.len();

        let _ = app.update(Message::Send);

        assert_eq!(
            app.conversation.len(),
            before,
            "should not send while streaming"
        );
        assert_eq!(app.streaming_block_id, Some(42));
    }

    #[test]
    fn send_message_returns_early_when_input_empty() {
        let (mut app, _) = App::new();
        app.input.clear();

        let _ = app.update(Message::Send);

        assert!(
            app.status.is_empty() || app.status == "준비됨" || app.status.starts_with("[복구됨]"),
            "unexpected status: {}",
            app.status
        );
    }

    #[test]
    fn regenerate_last_returns_early_when_streaming() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).push(ChatMessage::user("hello"));
        app.streaming_block_id = Some(42);
        let before = app.conversation.len();

        let _ = app.update(Message::RegenerateLast);

        assert_eq!(app.conversation.len(), before);
    }

    #[test]
    fn regenerate_last_returns_early_when_no_user_message() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();

        let _ = app.update(Message::RegenerateLast);

        assert!(
            app.status.is_empty() || app.status == "준비됨" || app.status.starts_with("[복구됨]"),
            "unexpected status: {}",
            app.status
        );
    }

    #[test]
    fn toggle_compare_mode_updates_status() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::ToggleCompareBoth(true));

        assert!(app.compare_both);
        assert!(app.status.contains("Compare 모드"), "got: {}", app.status);

        let _ = app.update(Message::ToggleCompareBoth(false));

        assert!(!app.compare_both);
        assert!(app.status.contains("Single 모드"), "got: {}", app.status);
    }

    #[test]
    fn local_openai_compat_models_do_not_send_tool_definitions() {
        let (mut app, _) = App::new();
        app.model_options = vec![ModelOption {
            id: "local-model".into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: "TabbyAPI".into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: Some(0.0),
            completion_per_million: Some(0.0),
        }];
        app.selected_model = Some("local-model".into());

        assert!(app.tool_definitions_for_selected_model().is_none());
    }

    #[test]
    fn selected_model_with_same_id_uses_explicit_provider_choice() {
        let (mut app, _) = App::new();
        app.model_options = vec![or_opt("shared-model"), oai_opt("shared-model", "TabbyAPI")];
        app.tabby_url_input = "http://localhost:5000".into();

        let _ = app.update(Message::SelectModel(oai_opt("shared-model", "TabbyAPI")));
        assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenAICompat));
        assert!(app.tool_definitions_for_selected_model().is_none());

        let _ = app.update(Message::SelectModel(or_opt("shared-model")));
        assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenRouter));
        assert!(app.tool_definitions_for_selected_model().is_some());
    }

    #[test]
    fn resolve_provider_prefers_current_tabby_inputs_over_keystore() {
        let (mut app, _) = App::new();
        app.model_options = vec![oai_opt("local-model", "TabbyAPI")];
        app.selected_model = Some("local-model".into());
        app.selected_model_provider = Some(LlmProvider::OpenAICompat);
        app.tabby_url_input = "http://localhost:5001".into();
        app.tabby_token_input = "live-token".into();

        let (base_url, api_key) = app.resolve_provider().expect("provider resolves");

        assert_eq!(base_url, "http://localhost:5001/v1");
        assert_eq!(api_key.as_deref(), Some("live-token"));
    }

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
        app.mid_stream_retries = MAX_MID_STREAM_RETRIES; // prevent mid-stream retry from clearing content

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
