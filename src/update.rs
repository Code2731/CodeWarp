// update.rs — App update 메서드 (main.rs child module)
use super::*;
use iced::{Subscription, Task};
use std::time::Duration;

impl App {
    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::OpenSettings => self.open_settings_overlay(),
            Message::CloseSettings => self.close_settings_overlay(),
            Message::SetSettingsTab(tab) => self.set_settings_tab(tab),
            Message::KeyInputChanged(v) => self.set_key_input(v),
            Message::SaveKey => self.save_api_key(),
            Message::KeySaved(r) => self.on_key_saved(r),
            Message::ClearKey => self.clear_api_key(),
            Message::KeyCleared(r) => self.on_key_cleared(r),
            Message::TabbyUrlChanged(v) => self.set_tabby_url(v),
            Message::TabbyTokenChanged(v) => self.set_tabby_token(v),
            Message::ToggleTabbyTokenVisible => self.toggle_tabby_token_visible(),
            Message::InferenceCommandChanged(v) => self.set_inference_command(v),
            Message::SelectInferenceEngine(e) => self.select_inference_engine(e),
            Message::SelectInferenceModel(m) => self.set_inference_model(m),
            Message::InferencePortChanged(v) => self.set_inference_port(v),
            Message::InferenceBinaryChanged(v) => self.set_inference_binary(v),
            Message::PickInferenceBinary => self.pick_inference_binary(),
            Message::InferenceBinaryPicked(maybe) => self.on_inference_binary_picked(maybe),
            Message::InstallTabbyApiRuntime => self.install_tabbyapi_runtime_cmd(),
            Message::TabbyApiRuntimeInstalled(result) => self.on_tabbyapi_runtime_installed(result),
            Message::StartInference => self.start_inference(),
            Message::StopInference => self.stop_inference(),
            Message::InferenceLogLine(line) => self.on_inference_log_line(line),
            Message::InferenceExited(code) => self.on_inference_exited(code),
            Message::OpenAICompatLabelChanged(v) => self.set_openai_compat_label(v),
            Message::SaveTabby => self.save_tabby_settings(),
            Message::TabbySaved(r) => self.on_tabby_saved(r),
            Message::ClearTabby => self.clear_tabby_settings(),
            Message::FetchTabbyModels => self.fetch_tabby_models(),
            Message::FetchTabbyModelsRetry(generation) => self.retry_fetch_tabby_models(generation),
            Message::TabbyModelsLoaded(r) => self.on_tabby_models_loaded(r),
            // ── HF 모델 매니저 ────────────────────────────────────
            Message::HfTokenChanged(v) => self.set_hf_token_input(v),
            Message::ToggleHfTokenVisible => self.toggle_hf_token_visible(),
            Message::SaveHfToken => self.save_hf_token(),
            Message::HfTokenSaved(r) => self.on_hf_token_saved(r),
            Message::ModelDirChanged(v) => self.set_model_dir(v),
            Message::PickModelDir => Task::perform(
                async {
                    rfd::AsyncFileDialog::new()
                        .pick_folder()
                        .await
                        .map(|h| h.path().to_path_buf())
                },
                Message::ModelDirPicked,
            ),
            Message::ModelDirPicked(maybe) => self.on_model_dir_picked(maybe),
            Message::HfRepoChanged(v) => self.set_hf_repo_input(v),
            Message::UsePreset(idx) => self.apply_model_preset(idx),
            Message::DownloadExl2Preset(idx) => self.prepare_exl2_preset_download(idx),
            Message::SelectDownloadedModel(folder_name) => {
                self.select_downloaded_model(folder_name)
            }
            Message::StartHfDownload => self.start_hf_download(),
            Message::HfDownloadEvent(ev) => self.on_hf_download_event(ev),
            Message::CancelHfDownload => self.cancel_hf_download(),
            Message::RegenerateLast => self.regenerate_last(),
            Message::ApplyChange(block_id, idx) => self.apply_change(block_id, idx),
            Message::EditLastUser => self.edit_last_user(),

            // ── 파일 컨텍스트 첨부 ────────────────────────────────
            Message::FileDropped(path) => self.on_file_dropped(path),
            Message::FileDragHover => self.file_drag_hover(),
            Message::FileReadDone(path, content) => self.on_file_read_done(path, content),
            Message::FileAttachError(msg) => self.file_attach_error(msg),

            // ── MCP ───────────────────────────────────────────────────
            Message::McpNameChanged(v) => self.update_mcp_name_input(v),
            Message::McpCommandChanged(v) => self.update_mcp_command_input(v),
            Message::AddMcpServer => self.add_mcp_server(),
            Message::RemoveMcpServer(idx) => self.remove_mcp_server(idx),
            Message::McpToolsLoaded(server_name, tools) => {
                self.on_mcp_tools_loaded(server_name, tools)
            }
            Message::McpToolsFailed(msg) => self.on_mcp_tools_failed(msg),
            Message::McpToolResult(tool_call_id, result) => {
                self.on_mcp_tool_result(tool_call_id, result)
            }

            // ── PTY 터미널 ─────────────────────────────────────────
            Message::PtyToggle => self.toggle_pty(),
            Message::PtyStart => self.pty_start(),
            Message::PtyLine(line) => self.on_pty_line(line),
            Message::PtyExited => self.on_pty_exited(),
            Message::PtyInputChanged(v) => self.set_pty_input(v),
            Message::PtySend => self.send_pty_input(),
            Message::PtyCtrlC => self.pty_ctrl_c(),
            Message::PtyClear => self.pty_clear(),
            Message::RemoveAttachment(idx) => self.remove_attachment(idx),
            Message::ClearAttachments => self.clear_attachments(),
            Message::AutoSave => {
                self.save_session();
                Task::none()
            }
            Message::WindowCloseRequested => {
                self.save_session();
                session::mark_clean_shutdown();
                Task::none()
            }

            // ── @-mention ─────────────────────────────────────────
            Message::MentionMove(delta) => self.move_mention_selection(delta),
            Message::MentionConfirm => self.confirm_mention(),
            Message::MentionCandidatesLoaded(paths) => self.load_mention_candidates(paths),

            Message::FetchModels => self.fetch_models_cmd(),
            Message::ModelsLoaded(r) => self.on_models_loaded(r),
            Message::SelectModel(opt) => self.select_model(opt),
            Message::FetchAccount => self.fetch_account_cmd(),
            Message::AccountLoaded(r) => self.on_account_loaded(r),
            Message::InputChanged(v) => self.on_input_changed(v),
            Message::Send => self.send_message(),
            Message::StopStream => self.stop_stream(),
            Message::CopyBlock(id) => self.copy_block(id),
            Message::CopyText(text) => iced::clipboard::write(text),
            Message::CompareResponsesLoaded {
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            } => self.on_compare_responses_loaded(
                openrouter_block_id,
                tabby_block_id,
                openrouter_result,
                tabby_result,
            ),
            Message::ChatChunk(event) => self.on_chat_chunk(event),
            Message::StreamScrolled(viewport) => self.on_stream_scrolled(&viewport),
            Message::EditorAction(id, action) => self.on_editor_action(id, action),
            Message::ToggleBlockView(id) => self.toggle_block_view(id),
            Message::LinkClicked(uri) => self.on_link_clicked(&uri),
            Message::PickCwd => self.pick_cwd(),
            Message::PickAttachment => self.pick_attachment(),
            Message::AttachmentPicked(maybe_path) => self.on_attachment_picked(maybe_path),
            Message::ApproveWrites => self.approve_pending_writes(),
            Message::DenyWrites => self.deny_pending_writes(),
            Message::ToggleConfirmExpand(idx) => self.toggle_write_confirm_expand(idx),
            Message::DiscardWriteCall(idx) => self.discard_write_call(idx),
            Message::ToggleFilterCoding(v) => self.set_filter_coding(v),
            Message::ToggleFilterReasoning(v) => self.set_filter_reasoning(v),
            Message::ToggleFilterGeneral(v) => self.set_filter_general(v),
            Message::ToggleFilterFavorites(v) => self.set_filter_favorites_only(v),
            Message::ToggleCompareBoth(v) => self.set_compare_both(v),
            Message::CycleSortMode => self.cycle_model_sort_mode(),
            Message::CycleSidebarWidth => self.cycle_sidebar_width(),
            Message::SetAgentMode(mode) => self.set_agent_mode(mode),
            Message::ToggleAgentMode => self.toggle_agent_mode(),
            Message::NewChat => self.new_chat(),
            Message::SwitchSession(target_id) => self.switch_session(target_id),
            Message::OpenCommandPalette => self.open_command_palette(),
            Message::CloseCommandPalette => self.close_command_palette(),
            Message::CloseAllOverlays => self.close_all_overlays(),
            Message::CommandPaletteChanged(v) => self.update_command_palette_input(v),
            Message::ExecuteCommand(idx) => self.execute_palette_command(idx),
            Message::GenerationLoaded(r) => self.on_generation_loaded(r),
            Message::AskDeleteSession(id) => self.ask_delete_session(id),
            Message::CancelDeleteSession => self.cancel_delete_session(),
            Message::DeleteSession(target_id) => self.delete_session(target_id),
            Message::ToggleFavorite => self.toggle_favorite(),
            Message::CwdPicked(maybe_path) => self.apply_picked_cwd(maybe_path),
        }
    }

    // ── Key persistence helpers ──────────────────────────────────

    // ── Tabby connection helpers ──────────────────────────────────

    // ── Inference/Model dir helpers ───────────────────────────────

    // ── PTY helpers ───────────────────────────────────────────────

    // ── Attachment helpers ────────────────────────────────────────

    // ── Mention helpers ───────────────────────────────────────────

    // ── Write confirm helpers ────────────────────────────────────

    // ── HF token helpers ─────────────────────────────────────────

    // ── HF preset helpers ────────────────────────────────────────

    // ── File attachment result helpers ────────────────────────────

    // ── Model select / account helpers ────────────────────────────

    // ── Session lifecycle helpers ──────────────────────────────────

    // ── MCP server helpers ────────────────────────────────────────

    // ── Inference lifecycle helpers ────────────────────────────────

    // ── Model dir / HF model helpers ──────────────────────────────

    // ── Usage / write confirm helpers ──────────────────────────────

    // ── Inference engine config helpers ───────────────────────────

    // ── Tabby model fetch helpers ─────────────────────────────────

    /// 현재 활성 필터/정렬을 적용해 model_options을 좁힌 결과.
    pub(crate) fn filtered_model_options(&self) -> Vec<ModelOption> {
        let mut opts: Vec<ModelOption> = self
            .model_options
            .iter()
            .filter(|opt| {
                if self.model_filter.filter_favorites_only
                    && !self.model_filter.favorites.contains(&opt.id)
                {
                    return false;
                }
                let cats = categorize_model(&opt.id);
                (self.model_filter.filter_coding && cats.contains(&ModelCategory::Coding))
                    || (self.model_filter.filter_reasoning
                        && cats.contains(&ModelCategory::Reasoning))
                    || (self.model_filter.filter_general && cats.contains(&ModelCategory::General))
            })
            .cloned()
            .collect();

        // 정렬: prompt+completion 합 기준
        let total_price = |o: &ModelOption| -> f64 {
            o.prompt_per_million.unwrap_or(0.0) + o.completion_per_million.unwrap_or(0.0)
        };
        match self.model_filter.sort_mode {
            SortMode::Default => {}
            SortMode::PriceAsc => opts.sort_by(|a, b| {
                total_price(a)
                    .partial_cmp(&total_price(b))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
            SortMode::PriceDesc => opts.sort_by(|a, b| {
                total_price(b)
                    .partial_cmp(&total_price(a))
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
        opts
    }

    pub(crate) fn sync_selected_model_provider(&mut self) {
        let Some(selected_id) = self.selected_model.as_deref() else {
            self.selected_model_provider = None;
            return;
        };

        if let Some(provider) = self.selected_model_provider {
            if self
                .model_options
                .iter()
                .any(|o| o.id == selected_id && o.provider == provider)
            {
                return;
            }
        }

        let mut matches = self
            .model_options
            .iter()
            .filter(|o| o.id == selected_id)
            .map(|o| o.provider);

        let Some(first) = matches.next() else {
            self.selected_model_provider = None;
            return;
        };

        let mut has_openrouter = first == LlmProvider::OpenRouter;
        let mut has_openai_compat = first == LlmProvider::OpenAICompat;
        for provider in matches {
            match provider {
                LlmProvider::OpenRouter => has_openrouter = true,
                LlmProvider::OpenAICompat => has_openai_compat = true,
            }
        }

        self.selected_model_provider = if has_openrouter && has_openai_compat {
            if self.tabby_url_input.trim().is_empty() {
                Some(LlmProvider::OpenRouter)
            } else {
                Some(LlmProvider::OpenAICompat)
            }
        } else if has_openrouter {
            Some(LlmProvider::OpenRouter)
        } else if has_openai_compat {
            Some(LlmProvider::OpenAICompat)
        } else {
            None
        };
    }

    /// 필터/즐겨찾기 변경 시 combo_box::State 재구성.
    pub(crate) fn refresh_model_combo(&mut self) {
        self.sync_selected_model_provider();
        // favorite 필드를 현재 favorites HashSet과 동기화 (Display에 ★ 반영)
        for opt in &mut self.model_options {
            opt.favorite = self.model_filter.favorites.contains(&opt.id);
        }
        self.model_combo_state = combo_box::State::new(self.filtered_model_options());
    }

    /// 현재 활성 세션 + 비활성 세션 모두를 디스크에 저장.
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

    /// 현재 활성 세션 제목 자동 갱신 (첫 사용자 메시지 일부).
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

    /// 현재 활성 세션을 inactive_sessions로 이동 (push 또는 update).

    pub(crate) fn allocate_session_id(&mut self) -> u64 {
        let id = self.next_session_id;
        self.next_session_id += 1;
        id
    }

    /// conversation 첫 위치에 cwd를 알려주는 system 메시지를 보장 (없으면 추가, 있으면 갱신).
    pub(crate) fn close_mention(&mut self) {
        self.show_mention = false;
        self.mention_query.clear();
        self.mention_selected = 0;
    }

    pub(crate) fn normalized_attachment_path(&self, path: &std::path::Path) -> std::path::PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.cwd.join(path)
        }
    }

    pub(crate) fn is_already_attached(&self, path: &std::path::Path) -> bool {
        let needle = self.normalized_attachment_path(path);
        self.attached_files
            .iter()
            .any(|(p, _)| self.normalized_attachment_path(p) == needle)
    }

    pub(crate) fn total_attached_bytes(&self) -> u64 {
        self.attached_files
            .iter()
            .map(|(_, content)| content.len() as u64)
            .sum()
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

    /// pending_tool_calls를 conversation에 반영, 안전한 도구는 즉시 실행하고
    /// mutating 도구가 있으면 사용자 승인 모달을 띄움. 모두 처리되면 새 chat_stream 트리거.

    /// inference 서버 로그를 ring buffer에 push (cap 20).

    pub(crate) fn push_pty_line(&mut self, line: String) {
        self.pty_output.push_back(line);
        if self.pty_output.len() > PTY_MAX_LINES {
            self.pty_output.pop_front();
        }
    }

    /// 도구 실행 결과 chip 블록을 stream에 push (휘발성 — 세션 저장 안 됨).

    /// 사용자 승인/거부 후 호출. true면 mutating 실행, false면 거부 결과를 conversation에 기록.

    pub(crate) fn resolve_provider(&self) -> Result<(String, Option<String>), String> {
        let id = self
            .selected_model
            .as_deref()
            .ok_or_else(|| "모델 미선택".to_string())?;
        let provider = self
            .selected_option()
            .map(|o| o.provider)
            .ok_or_else(|| format!("선택된 모델을 찾을 수 없습니다: {}", id))?;
        match provider {
            LlmProvider::OpenRouter => {
                let key = keystore::read_api_key()?;
                Ok((openrouter::BASE_URL.to_string(), Some(key)))
            }
            LlmProvider::OpenAICompat => {
                let base = if self.tabby_url_input.trim().is_empty() {
                    keystore::read_tabby_base_url()
                } else {
                    Some(self.tabby_url_input.clone())
                }
                .filter(|s| !s.trim().is_empty())
                .ok_or_else(|| "Tabby URL 미설정".to_string())?;
                let token = if self.tabby_token_input.trim().is_empty() {
                    keystore::read_tabby_token()
                } else {
                    Some(self.tabby_token_input.clone())
                }
                .filter(|s| !s.trim().is_empty());
                Ok((tabby::chat_base(&base), token))
            }
        }
    }

    pub(crate) fn selected_provider(&self) -> Option<LlmProvider> {
        self.selected_option().map(|o| o.provider)
    }

    pub(crate) fn selected_model_supports_tools(&self) -> bool {
        matches!(self.selected_provider(), Some(LlmProvider::OpenRouter))
    }

    pub(crate) fn tool_definitions_for_selected_model(&self) -> Option<serde_json::Value> {
        if self.selected_model_supports_tools() {
            Some(tools::tool_definitions(self.agent_mode.allow_mutating()))
        } else {
            None
        }
    }

    pub(crate) fn selected_option(&self) -> Option<&ModelOption> {
        let id = self.selected_model.as_deref()?;
        if let Some(provider) = self.selected_model_provider {
            if let Some(opt) = self
                .model_options
                .iter()
                .find(|o| o.id == id && o.provider == provider)
            {
                return Some(opt);
            }
        }
        self.model_options.iter().find(|o| o.id == id)
    }

    pub(crate) fn selected_model_exists_in_options(&self) -> bool {
        self.selected_model
            .as_deref()
            .map(|id| self.model_options.iter().any(|o| o.id == id))
            .unwrap_or(false)
    }

    /// 누적된 conversation을 가지고 다음 chat_stream을 시작.

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let event_sub = iced::event::listen_with(on_event);
        let interval = if self.streaming_block_id.is_some() {
            Duration::from_secs(15)
        } else {
            Duration::from_secs(60)
        };
        let timer_sub = iced::time::every(interval).map(|_| Message::AutoSave);
        Subscription::batch(vec![event_sub, timer_sub])
    }

    pub(crate) fn filtered_palette_commands(&self) -> Vec<&'static PaletteCommand> {
        let q = self.ui.command_palette_input.to_lowercase();
        if q.is_empty() {
            PALETTE_COMMANDS.iter().collect()
        } else {
            PALETTE_COMMANDS
                .iter()
                .filter(|c| {
                    c.label.to_lowercase().contains(&q) || c.hint.to_lowercase().contains(&q)
                })
                .collect()
        }
    }

    pub(crate) fn next_id(&mut self) -> u64 {
        let id = self.next_block_id;
        self.next_block_id += 1;
        id
    }
}
