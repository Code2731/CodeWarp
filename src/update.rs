// update.rs — App update 메서드 (main.rs child module)
use super::*;
use iced::{Subscription, Task};
use std::time::Duration;

use crate::view::{SIDEBAR_WIDTH, SIDEBAR_WIDTH_COMPACT, SIDEBAR_WIDTH_WIDE};

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

    pub(crate) fn open_settings_overlay(&mut self) -> Task<Message> {
        self.ui.show_settings = true;
        self.ui.settings_tab = SettingsTab::Provider;
        Task::none()
    }

    pub(crate) fn close_settings_overlay(&mut self) -> Task<Message> {
        self.ui.show_settings = false;
        Task::none()
    }

    pub(crate) fn set_settings_tab(&mut self, tab: SettingsTab) -> Task<Message> {
        self.ui.settings_tab = tab;
        Task::none()
    }

    pub(crate) fn update_mcp_name_input(&mut self, value: String) -> Task<Message> {
        self.mcp_input.name_input = value;
        Task::none()
    }

    pub(crate) fn update_mcp_command_input(&mut self, value: String) -> Task<Message> {
        self.mcp_input.command_input = value;
        Task::none()
    }

    pub(crate) fn toggle_write_confirm_expand(&mut self, idx: usize) -> Task<Message> {
        self.ui.expanded_confirm_idx = if self.ui.expanded_confirm_idx == Some(idx) {
            None
        } else {
            Some(idx)
        };
        Task::none()
    }

    pub(crate) fn set_filter_coding(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_coding = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    pub(crate) fn set_filter_reasoning(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_reasoning = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    pub(crate) fn set_filter_general(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_general = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    pub(crate) fn set_filter_favorites_only(&mut self, enabled: bool) -> Task<Message> {
        self.model_filter.filter_favorites_only = enabled;
        self.refresh_model_combo();
        Task::none()
    }

    pub(crate) fn cycle_model_sort_mode(&mut self) -> Task<Message> {
        self.model_filter.sort_mode = self.model_filter.sort_mode.cycle();
        self.refresh_model_combo();
        Task::none()
    }

    pub(crate) fn cycle_sidebar_width(&mut self) -> Task<Message> {
        self.sidebar_width = if (self.sidebar_width - SIDEBAR_WIDTH_COMPACT).abs() < f32::EPSILON {
            SIDEBAR_WIDTH
        } else if (self.sidebar_width - SIDEBAR_WIDTH).abs() < f32::EPSILON {
            SIDEBAR_WIDTH_WIDE
        } else {
            SIDEBAR_WIDTH_COMPACT
        };
        self.status = format!("사이드바 너비: {:.0}px", self.sidebar_width);
        Task::none()
    }

    pub(crate) fn open_command_palette(&mut self) -> Task<Message> {
        self.ui.show_command_palette = true;
        self.ui.command_palette_input.clear();
        Task::none()
    }

    pub(crate) fn close_command_palette(&mut self) -> Task<Message> {
        self.ui.show_command_palette = false;
        Task::none()
    }

    pub(crate) fn update_command_palette_input(&mut self, value: String) -> Task<Message> {
        self.ui.command_palette_input = value;
        Task::none()
    }

    pub(crate) fn ask_delete_session(&mut self, id: u64) -> Task<Message> {
        self.ui.pending_delete_session = if self.ui.pending_delete_session == Some(id) {
            None
        } else {
            Some(id)
        };
        Task::none()
    }

    pub(crate) fn cancel_delete_session(&mut self) -> Task<Message> {
        self.ui.pending_delete_session = None;
        Task::none()
    }

    pub(crate) fn toggle_favorite(&mut self) -> Task<Message> {
        if let Some(id) = &self.selected_model {
            if self.model_filter.favorites.contains(id) {
                self.model_filter.favorites.remove(id);
            } else {
                self.model_filter.favorites.insert(id.clone());
            }
            let favs: Vec<String> = self.model_filter.favorites.iter().cloned().collect();
            let _ = session::write_favorites(&favs);
            self.refresh_model_combo();
        }
        Task::none()
    }

    pub(crate) fn set_compare_both(&mut self, enabled: bool) -> Task<Message> {
        self.compare_both = enabled;
        self.status = if enabled {
            "Compare 모드 — OpenRouter와 Tabby가 각각 답변합니다.".into()
        } else {
            "Single 모드 — 선택한 모델 하나만 답변합니다.".into()
        };
        Task::none()
    }

    pub(crate) fn set_agent_mode(&mut self, mode: AgentMode) -> Task<Message> {
        self.agent_mode = mode;
        self.status = format!("{} 모드", mode.label());
        Task::none()
    }

    pub(crate) fn toggle_agent_mode(&mut self) -> Task<Message> {
        self.agent_mode = match self.agent_mode {
            AgentMode::Plan => AgentMode::Build,
            AgentMode::Build => AgentMode::Plan,
        };
        self.status = format!("{} 모드", self.agent_mode.label());
        Task::none()
    }

    pub(crate) fn close_all_overlays(&mut self) -> Task<Message> {
        self.ui.show_command_palette = false;
        self.ui.show_settings = false;
        self.show_write_confirm = false;
        self.close_mention();
        Task::none()
    }

    pub(crate) fn execute_palette_command(&mut self, idx: usize) -> Task<Message> {
        let filtered = self.filtered_palette_commands();
        let Some(cmd) = filtered.get(idx) else {
            return Task::none();
        };
        let action = cmd.action;
        self.ui.show_command_palette = false;
        self.ui.command_palette_input.clear();
        match action {
            PaletteAction::NewChat => Task::done(Message::NewChat),
            PaletteAction::PlanMode => Task::done(Message::SetAgentMode(AgentMode::Plan)),
            PaletteAction::BuildMode => Task::done(Message::SetAgentMode(AgentMode::Build)),
            PaletteAction::OpenSettings => Task::done(Message::OpenSettings),
            PaletteAction::PickCwd => Task::done(Message::PickCwd),
            PaletteAction::CycleSort => Task::done(Message::CycleSortMode),
            PaletteAction::ToggleFavorite => Task::done(Message::ToggleFavorite),
        }
    }

    pub(crate) fn apply_picked_cwd(
        &mut self,
        maybe_path: Option<std::path::PathBuf>,
    ) -> Task<Message> {
        if let Some(path) = maybe_path {
            self.cwd = path.clone();
            let _ = keystore::write_cwd(&path.display().to_string());
            self.status = format!("작업 폴더: {}", path.display());
            self.ensure_system_message();
        }
        Task::none()
    }

    pub(crate) fn set_key_input(&mut self, value: String) -> Task<Message> {
        self.key_input = value;
        Task::none()
    }

    pub(crate) fn set_tabby_url(&mut self, value: String) -> Task<Message> {
        self.tabby_url_input = value;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }

    pub(crate) fn set_tabby_token(&mut self, value: String) -> Task<Message> {
        self.tabby_token_input = value;
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        Task::none()
    }

    pub(crate) fn toggle_tabby_token_visible(&mut self) -> Task<Message> {
        self.show_tabby_token = !self.show_tabby_token;
        Task::none()
    }

    pub(crate) fn set_inference_command(&mut self, value: String) -> Task<Message> {
        self.inference_command_input = value.clone();
        let _ = keystore::write_inference_command(&value);
        Task::none()
    }

    pub(crate) fn set_inference_model(&mut self, value: String) -> Task<Message> {
        self.inference_selected_model = value;
        Task::none()
    }

    pub(crate) fn set_hf_token_input(&mut self, value: String) -> Task<Message> {
        self.hf_token_input = value;
        Task::none()
    }

    pub(crate) fn toggle_hf_token_visible(&mut self) -> Task<Message> {
        self.show_hf_token = !self.show_hf_token;
        Task::none()
    }

    pub(crate) fn set_hf_repo_input(&mut self, value: String) -> Task<Message> {
        self.hf_repo_input = value;
        Task::none()
    }

    pub(crate) fn set_pty_input(&mut self, value: String) -> Task<Message> {
        self.pty_input = value;
        Task::none()
    }

    pub(crate) fn pty_ctrl_c(&mut self) -> Task<Message> {
        if let Some(s) = &self.pty_session {
            s.ctrl_c();
        }
        Task::none()
    }

    pub(crate) fn pty_clear(&mut self) -> Task<Message> {
        self.pty_output.clear();
        Task::none()
    }

    pub(crate) fn file_drag_hover(&mut self) -> Task<Message> {
        Task::none()
    }

    pub(crate) fn file_attach_error(&mut self, msg: String) -> Task<Message> {
        self.status = msg;
        Task::none()
    }

    // ── Key persistence helpers ──────────────────────────────────

    pub(crate) fn save_api_key(&mut self) -> Task<Message> {
        let key = self.key_input.clone();
        self.busy = true;
        self.status = "키 저장 중…".into();
        Task::perform(
            async move { keystore::write_api_key(&key) },
            Message::KeySaved,
        )
    }

    pub(crate) fn on_key_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.has_key = true;
                self.key_input.clear();
                self.ui.show_settings = false;
                self.status = "키 저장됨".into();
                Task::done(Message::FetchModels)
            }
            Err(e) => {
                self.status = format!("저장 실패: {}", e);
                Task::none()
            }
        }
    }

    pub(crate) fn clear_api_key(&mut self) -> Task<Message> {
        self.busy = true;
        Task::perform(async { keystore::delete_api_key() }, Message::KeyCleared)
    }

    pub(crate) fn on_key_cleared(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.has_key = false;
                self.models.clear();
                self.model_ids.clear();
                self.selected_model = None;
                self.selected_model_provider = None;
                let _ = keystore::clear_selected_model();
                self.status = "키 삭제됨".into();
            }
            Err(e) => self.status = format!("삭제 실패: {}", e),
        }
        Task::none()
    }

    // ── Tabby connection helpers ──────────────────────────────────

    pub(crate) fn set_openai_compat_label(&mut self, value: String) -> Task<Message> {
        self.openai_compat_label = value;
        let _ = keystore::write_openai_compat_label(&self.openai_compat_label);
        let new_label = self.openai_compat_label.clone();
        for opt in &mut self.model_options {
            if opt.provider == LlmProvider::OpenAICompat {
                opt.provider_label = new_label.clone();
            }
        }
        self.refresh_model_combo();
        Task::none()
    }

    pub(crate) fn save_tabby_settings(&mut self) -> Task<Message> {
        let url = self.tabby_url_input.clone();
        let token = self.tabby_token_input.clone();
        self.busy = true;
        self.status = "Tabby 설정 저장 중…".into();
        Task::perform(
            async move {
                keystore::write_tabby_base_url(&url)?;
                keystore::write_tabby_token(&token)?;
                Ok(())
            },
            Message::TabbySaved,
        )
    }

    pub(crate) fn on_tabby_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(()) => {
                self.status = "Tabby 설정 저장됨".into();
                if !self.tabby_url_input.trim().is_empty() {
                    return Task::done(Message::FetchTabbyModels);
                }
            }
            Err(e) => self.status = format!("Tabby 저장 실패: {}", e),
        }
        Task::none()
    }

    pub(crate) fn clear_tabby_settings(&mut self) -> Task<Message> {
        let _ = keystore::clear_tabby_base_url();
        let _ = keystore::clear_tabby_token();
        self.tabby_url_input.clear();
        self.tabby_token_input.clear();
        self.tabby_connect_retry_left = 0;
        self.tabby_retry_generation = self.tabby_retry_generation.saturating_add(1);
        self.tabby_status = None;
        self.status = "Tabby 설정 삭제됨".into();
        self.model_options
            .retain(|o| o.provider != LlmProvider::OpenAICompat);
        self.refresh_model_combo();
        if let Some(sel) = self.selected_model.clone() {
            if !self.model_options.iter().any(|o| o.id == sel) {
                if let Some(first) = self.model_options.first() {
                    self.selected_model = Some(first.id.clone());
                    self.selected_model_provider = Some(first.provider);
                } else {
                    self.selected_model = None;
                    self.selected_model_provider = None;
                }
                if let Some(id) = &self.selected_model {
                    let _ = keystore::write_selected_model(id);
                }
            }
        }
        Task::none()
    }

    // ── Inference/Model dir helpers ───────────────────────────────

    // ── PTY helpers ───────────────────────────────────────────────

    pub(crate) fn toggle_pty(&mut self) -> Task<Message> {
        self.pty_visible = !self.pty_visible;
        if self.pty_visible && self.pty_session.is_none() {
            return Task::done(Message::PtyStart);
        }
        Task::none()
    }

    pub(crate) fn send_pty_input(&mut self) -> Task<Message> {
        let line = self.pty_input.trim_end().to_string();
        if let Some(s) = &self.pty_session {
            s.write_line(&line);
        }
        self.pty_input.clear();
        Task::none()
    }

    // ── Attachment helpers ────────────────────────────────────────

    pub(crate) fn remove_attachment(&mut self, idx: usize) -> Task<Message> {
        if idx < self.attached_files.len() {
            let removed = self.attached_files.remove(idx);
            let removed_name = removed
                .0
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| removed.0.display().to_string());
            self.status = format!(
                "Removed attachment: {} ({} left)",
                removed_name,
                self.attached_files.len()
            );
        }
        Task::none()
    }

    pub(crate) fn clear_attachments(&mut self) -> Task<Message> {
        if !self.attached_files.is_empty() {
            let removed_count = self.attached_files.len();
            let removed_bytes: u64 = self
                .attached_files
                .iter()
                .map(|(_, content)| content.len() as u64)
                .sum();
            self.attached_files.clear();
            self.status = format!(
                "Cleared attachments: {} files ({})",
                removed_count,
                fmt_bytes(removed_bytes)
            );
        }
        Task::none()
    }

    // ── Mention helpers ───────────────────────────────────────────

    pub(crate) fn move_mention_selection(&mut self, delta: i32) -> Task<Message> {
        if !self.show_mention || self.mention_candidates.is_empty() {
            return Task::none();
        }
        let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
        let n = filtered.len();
        if n == 0 {
            return Task::none();
        }
        self.mention_selected =
            (self.mention_selected as i64 + delta as i64).rem_euclid(n as i64) as usize;
        Task::none()
    }

    pub(crate) fn confirm_mention(&mut self) -> Task<Message> {
        if !self.show_mention {
            return Task::none();
        }
        let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
        let Some(chosen) = filtered.into_iter().nth(self.mention_selected) else {
            return Task::none();
        };
        if let Some(at_pos) = self.input.rfind('@') {
            self.input.truncate(at_pos);
        }
        self.close_mention();
        if self.is_already_attached(&chosen) {
            self.status = format!("Already attached: {}", chosen.display());
            return Task::none();
        }
        let full_path = self.cwd.join(&chosen);
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&full_path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        chosen.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((chosen, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }

    pub(crate) fn load_mention_candidates(
        &mut self,
        paths: Vec<std::path::PathBuf>,
    ) -> Task<Message> {
        self.mention_candidates = paths;
        Task::none()
    }

    // ── Write confirm helpers ────────────────────────────────────

    // ── HF token helpers ─────────────────────────────────────────

    pub(crate) fn save_hf_token(&mut self) -> Task<Message> {
        let t = self.hf_token_input.clone();
        Task::perform(
            async move { keystore::write_hf_token(&t) },
            Message::HfTokenSaved,
        )
    }

    pub(crate) fn on_hf_token_saved(&mut self, result: Result<(), String>) -> Task<Message> {
        match result {
            Ok(()) => self.status = "HF 토큰 저장됨".into(),
            Err(e) => self.status = format!("HF 토큰 저장 실패: {}", e),
        }
        Task::none()
    }

    // ── HF preset helpers ────────────────────────────────────────

    pub(crate) fn apply_model_preset(&mut self, idx: usize) -> Task<Message> {
        if let Some(p) = MODEL_PRESETS.get(idx) {
            self.hf_repo_input = p.repo_id.into();
            self.hf_revision = None;
            self.hf_folder_name = None;
        }
        Task::none()
    }

    pub(crate) fn prepare_exl2_preset_download(&mut self, idx: usize) -> Task<Message> {
        if let Some(p) = EXL2_PRESETS.get(idx) {
            self.hf_repo_input = p.repo_id.into();
            self.hf_revision = Some(p.revision.into());
            self.hf_folder_name = Some(p.folder_name.into());
            self.status = format!(
                "프리셋 다운로드 시작 준비: {} ({} @ {})",
                p.label, p.repo_id, p.revision
            );
            return Task::done(Message::StartHfDownload);
        }
        self.status = format!("잘못된 프리셋 인덱스: {}", idx);
        Task::none()
    }

    // ── File attachment result helpers ────────────────────────────

    // ── Model select / account helpers ────────────────────────────

    pub(crate) fn select_model(&mut self, opt: ModelOption) -> Task<Message> {
        let _ = keystore::write_selected_model(&opt.id);
        self.selected_model_provider = Some(opt.provider);
        self.selected_model = Some(opt.id);
        Task::none()
    }

    pub(crate) fn on_account_loaded(
        &mut self,
        result: Result<openrouter::AuthKeyData, String>,
    ) -> Task<Message> {
        if let Ok(data) = result {
            self.account = Some(data);
        }
        Task::none()
    }

    pub(crate) fn fetch_models_cmd(&mut self) -> Task<Message> {
        let key = match keystore::read_api_key() {
            Ok(k) => k,
            Err(e) => {
                self.status = e;
                return Task::none();
            }
        };
        self.busy = true;
        self.status = "모델 리스트 가져오는 중…".into();
        Task::perform(openrouter::list_models(key), Message::ModelsLoaded)
    }

    pub(crate) fn on_models_loaded(
        &mut self,
        result: Result<Vec<openrouter::OpenRouterModel>, String>,
    ) -> Task<Message> {
        self.busy = false;
        match result {
            Ok(models) => {
                let n = models.len();
                self.model_ids = models.iter().map(|m| m.id.clone()).collect();
                self.model_options
                    .retain(|o| o.provider != LlmProvider::OpenRouter);
                self.model_options.extend(models.iter().map(|m| {
                    let id = m.id.clone();
                    let ko_friendly = is_korean_friendly(&id);
                    let favorite = self.model_filter.favorites.contains(&id);
                    ModelOption {
                        id,
                        provider: LlmProvider::OpenRouter,
                        provider_label: String::new(),
                        ko_friendly,
                        favorite,
                        context_length: m.context_length,
                        prompt_per_million: parse_price_per_million(
                            m.pricing.as_ref().and_then(|p| p.prompt.as_deref()),
                        ),
                        completion_per_million: parse_price_per_million(
                            m.pricing.as_ref().and_then(|p| p.completion.as_deref()),
                        ),
                    }
                }));
                self.refresh_model_combo();
                let saved_in_list = self.selected_model_exists_in_options();
                if !saved_in_list && self.tabby_url_input.trim().is_empty() {
                    self.selected_model = self.model_ids.first().cloned();
                    self.selected_model_provider = self
                        .selected_model
                        .as_ref()
                        .map(|_| LlmProvider::OpenRouter);
                    if let Some(id) = &self.selected_model {
                        let _ = keystore::write_selected_model(id);
                    }
                }
                self.models = models;
                self.status = format!("모델 {} 로드됨", n);
            }
            Err(e) => self.status = format!("페치 실패: {}", openrouter::humanize_error(&e)),
        }
        Task::none()
    }

    // ── Session lifecycle helpers ──────────────────────────────────

    pub(crate) fn fetch_account_cmd(&mut self) -> Task<Message> {
        let key = match keystore::read_api_key() {
            Ok(k) => k,
            Err(_) => return Task::none(),
        };
        Task::perform(openrouter::get_account_info(key), Message::AccountLoaded)
    }

    pub(crate) fn pick_cwd(&self) -> Task<Message> {
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("작업 폴더 선택")
                    .pick_folder()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::CwdPicked,
        )
    }

    pub(crate) fn pick_attachment(&self) -> Task<Message> {
        let cwd = self.cwd.clone();
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title("첨부 파일 선택")
                    .set_directory(cwd)
                    .pick_file()
                    .await
                    .map(|h| h.path().to_path_buf())
            },
            Message::AttachmentPicked,
        )
    }

    pub(crate) fn on_attachment_picked(
        &mut self,
        maybe_path: Option<std::path::PathBuf>,
    ) -> Task<Message> {
        let Some(path) = maybe_path else {
            return Task::none();
        };
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        path.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((path, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }

    pub(crate) fn on_file_dropped(&mut self, path: std::path::PathBuf) -> Task<Message> {
        if self.is_already_attached(&path) {
            self.status = format!("Already attached: {}", path.display());
            return Task::none();
        }
        let existing_total = self.total_attached_bytes();
        Task::perform(
            async move {
                let content = tokio::fs::read_to_string(&path)
                    .await
                    .map_err(|e| format!("File read failed: {e}"))?;
                if content.len() > MAX_ATTACH_BYTES as usize {
                    return Err(format!(
                        "Attachment too large (max {}): {}",
                        fmt_bytes(MAX_ATTACH_BYTES),
                        path.display()
                    ));
                }
                let next_total = existing_total + content.len() as u64;
                if next_total > MAX_ATTACH_BYTES {
                    return Err(format!(
                        "Attachment limit exceeded: {} / {}",
                        fmt_bytes(next_total),
                        fmt_bytes(MAX_ATTACH_BYTES)
                    ));
                }
                Ok((path, content))
            },
            |r| match r {
                Ok((p, s)) => Message::FileReadDone(p, s),
                Err(msg) => Message::FileAttachError(msg),
            },
        )
    }

    pub(crate) fn add_mcp_server(&mut self) -> Task<Message> {
        let name = self.mcp_input.name_input.trim().to_string();
        let command = self.mcp_input.command_input.trim().to_string();
        if name.is_empty() || command.is_empty() {
            self.status = "MCP 서버 이름과 명령을 모두 입력하세요.".into();
            return Task::none();
        }
        let server = mcp::McpServer {
            name: name.clone(),
            command,
        };
        self.mcp_servers.push(server.clone());
        self.mcp_input.name_input.clear();
        self.mcp_input.command_input.clear();
        if let Err(e) = mcp::save_servers(&self.mcp_servers) {
            self.status = format!("MCP 저장 실패: {e}");
            return Task::none();
        }
        self.status = format!("MCP 서버 추가됨: {name} — tool 목록 로드 중…");
        Task::perform(
            async move {
                mcp::list_tools(&server)
                    .await
                    .map(|tools| (name.clone(), tools))
                    .map_err(|e| format!("[{name}] {e}"))
            },
            |r| match r {
                Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                Err(msg) => Message::McpToolsFailed(msg),
            },
        )
    }

    pub(crate) fn pty_start(&mut self) -> Task<Message> {
        match pty::spawn_pty(&self.cwd) {
            Ok((session, stream)) => {
                self.pty_session = Some(session);
                self.pty_output.clear();
                self.status = "터미널 시작됨".into();
                Task::run(stream, |event| match event {
                    pty::PtyEvent::Line(l) => Message::PtyLine(l),
                    pty::PtyEvent::Exited => Message::PtyExited,
                })
            }
            Err(e) => {
                self.status = format!("터미널 시작 실패: {e}");
                Task::none()
            }
        }
    }

    pub(crate) fn on_pty_line(&mut self, line: String) -> Task<Message> {
        let clean = pty::strip_ansi(&line);
        if !clean.trim().is_empty() {
            self.push_pty_line(clean);
        }
        Task::none()
    }

    pub(crate) fn on_pty_exited(&mut self) -> Task<Message> {
        self.pty_session = None;
        self.push_pty_line("-- 셸 종료 --".into());
        self.status = "터미널 종료됨".into();
        Task::none()
    }

    pub(crate) fn start_hf_download(&mut self) -> Task<Message> {
        if self.hf_dl.is_some() {
            self.status = "이미 다운로드가 진행 중입니다.".into();
            return Task::none();
        }
        if let Some(h) = self.hf_abort_handle.take() {
            h.abort();
        }
        let repo = self.hf_repo_input.trim().to_string();
        if repo.is_empty() {
            self.status = "HF repo ID 비어있음".into();
            return Task::none();
        }
        let mut dir = self.model_dir_input.trim().to_string();
        if dir.is_empty() {
            dir = default_models_dir();
            self.status = format!("다운로드 경로 자동 설정: {}", dir);
        }
        let resolved_dir = resolve_user_path(&dir);
        dir = resolved_dir.display().to_string();
        self.model_dir_input = dir.clone();
        if let Err(e) = std::fs::create_dir_all(&resolved_dir) {
            self.status = format!("다운로드 경로 생성 실패 ({}): {}", dir, e);
            return Task::none();
        }
        let _ = keystore::write_model_dir(&dir);
        let token = if self.hf_token_input.trim().is_empty() {
            keystore::read_hf_token()
        } else {
            Some(self.hf_token_input.trim().to_string())
        };
        let download_folder_name = self
            .hf_folder_name
            .take()
            .unwrap_or_else(|| repo.replace('/', "--"));
        let revision = self.hf_revision.take();
        self.hf_dl = Some(HfDownload {
            folder_name: download_folder_name.clone(),
            total_files: 0,
            file_idx: 0,
            file_name: String::new(),
            file_bytes_done: 0,
            file_bytes_total: None,
        });
        self.status = format!("다운로드 시작: {}", repo);
        let (task, handle) = Task::run(
            hf::download_repo(
                repo,
                resolved_dir,
                token,
                revision,
                Some(download_folder_name),
            ),
            Message::HfDownloadEvent,
        )
        .abortable();
        self.hf_abort_handle = Some(handle);
        task
    }

    pub(crate) fn on_hf_download_event(&mut self, ev: hf::DownloadEvent) -> Task<Message> {
        if let Some(dl) = self.hf_dl.as_mut() {
            match &ev {
                hf::DownloadEvent::Started { total_files } => {
                    dl.total_files = *total_files;
                }
                hf::DownloadEvent::FileStart { idx, name, size } => {
                    dl.file_idx = *idx;
                    dl.file_name = name.clone();
                    dl.file_bytes_done = 0;
                    dl.file_bytes_total = *size;
                }
                hf::DownloadEvent::FileProgress {
                    idx,
                    bytes_done,
                    bytes_total,
                } => {
                    dl.file_idx = *idx;
                    dl.file_bytes_done = *bytes_done;
                    dl.file_bytes_total = *bytes_total;
                }
                hf::DownloadEvent::FileDone => {}
                hf::DownloadEvent::AllDone => {
                    let folder_name = dl.folder_name.clone();
                    let model_path = downloaded_model_path(&self.model_dir_input, &folder_name);
                    let Some(resolved_model_path) =
                        resolve_tabbyapi_model_dir_for_folder(&model_path, &folder_name)
                    else {
                        self.status = format!(
                            "다운로드 결과에서 TabbyAPI 모델 경로를 확정할 수 없습니다: {} (config.json+가중치 파일이 필요하며, 여러 하위 모델이면 폴더 이름에 bpw 힌트가 필요합니다.)",
                            model_path.display()
                        );
                        self.tabby_status = Some(Err(self.status.clone()));
                        self.hf_dl = None;
                        self.hf_abort_handle = None;
                        return Task::none();
                    };
                    self.inference_engine = InferenceEngine::TabbyApi;
                    self.inference_selected_model = resolved_model_path.display().to_string();
                    self.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                    if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir())
                    {
                        self.inference_binary_path = launcher.display().to_string();
                        let _ = keystore::write_inference_binary(&self.inference_binary_path);
                    } else {
                        self.inference_binary_path.clear();
                        let _ = keystore::clear_inference_binary();
                    }
                    self.tabby_url_input =
                        format!("http://localhost:{}", self.inference_port_input);
                    let _ = keystore::write_tabby_base_url(&self.tabby_url_input);
                    if self.openai_compat_label.trim().is_empty() {
                        self.openai_compat_label = "TabbyAPI".into();
                        let _ = keystore::write_openai_compat_label("TabbyAPI");
                    }
                    self.status = format!(
                        "다운로드 완료: {} — Runtime에서 시작을 누른 뒤 연결 테스트",
                        folder_name
                    );
                    self.hf_dl = None;
                    self.hf_abort_handle = None;
                }
                hf::DownloadEvent::Error(e) => {
                    self.status = format!("다운로드 실패: {}", compose_hf_download_error(e));
                    self.hf_dl = None;
                    self.hf_abort_handle = None;
                }
            }
        }
        Task::none()
    }

    pub(crate) fn cancel_hf_download(&mut self) -> Task<Message> {
        if let Some(h) = self.hf_abort_handle.take() {
            h.abort();
        }
        self.hf_dl = None;
        self.status = "다운로드 취소됨".into();
        Task::none()
    }

    // ── MCP server helpers ────────────────────────────────────────

    pub(crate) fn remove_mcp_server(&mut self, idx: usize) -> Task<Message> {
        if idx < self.mcp_servers.len() {
            let removed = self.mcp_servers.remove(idx);
            self.mcp_tools.retain(|t| t.server_name != removed.name);
            let _ = mcp::save_servers(&self.mcp_servers);
            self.status = format!("MCP 서버 제거됨: {}", removed.name);
        }
        Task::none()
    }

    pub(crate) fn on_mcp_tools_loaded(
        &mut self,
        server_name: String,
        tools: Vec<mcp::McpTool>,
    ) -> Task<Message> {
        self.mcp_tools.retain(|t| t.server_name != server_name);
        let count = tools.len();
        self.mcp_tools.extend(tools);
        self.status = format!("MCP [{server_name}] tool {count}개 로드 완료");
        Task::none()
    }

    pub(crate) fn on_mcp_tools_failed(&mut self, msg: String) -> Task<Message> {
        self.status = format!("MCP tool 로드 실패: {msg}");
        Task::none()
    }

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
