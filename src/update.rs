// update.rs — App update 메서드 (main.rs child module)
use super::{App, Message, on_event, session};
use iced::{Subscription, Task};
use std::time::Duration;

impl App {
    #[allow(clippy::too_many_lines)]
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
            Message::InferenceCommandChanged(v) => self.set_inference_command(&v),
            Message::SelectInferenceEngine(e) => self.select_inference_engine(e),
            Message::SelectInferenceModel(m) => self.set_inference_model(m),
            Message::InferencePortChanged(v) => self.set_inference_port(&v),
            Message::InferenceBinaryChanged(v) => self.set_inference_binary(&v),
            Message::PickInferenceBinary => App::pick_inference_binary(),
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
            Message::ModelDirChanged(v) => self.set_model_dir(&v),
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
                self.select_downloaded_model(&folder_name)
            }
            Message::StartHfDownload => self.start_hf_download(),
            Message::HfDownloadEvent(ev) => self.on_hf_download_event(&ev),
            Message::CancelHfDownload => self.cancel_hf_download(),
            Message::RegenerateLast => self.regenerate_last(),
            Message::ApplyChange(block_id, idx) => self.apply_change(block_id, idx),
            Message::EditLastUser => self.edit_last_user(),

            // ── 파일 컨텍스트 첨부 ────────────────────────────────
            Message::FileDropped(path) => self.on_file_dropped(path),
            Message::FileDragHover => Self::file_drag_hover(),
            Message::FileReadDone(path, content) => self.on_file_read_done(path, content),
            Message::FileAttachError(msg) => self.file_attach_error(msg),

            // ── MCP ───────────────────────────────────────────────────
            Message::McpNameChanged(v) => self.update_mcp_name_input(v),
            Message::McpCommandChanged(v) => self.update_mcp_command_input(v),
            Message::AddMcpServer => self.add_mcp_server(),
            Message::RemoveMcpServer(idx) => self.remove_mcp_server(idx),
            Message::McpToolsLoaded(server_name, tools) => {
                self.on_mcp_tools_loaded(&server_name, tools)
            }
            Message::McpToolsFailed(msg) => self.on_mcp_tools_failed(&msg),
            Message::McpToolResult(tool_call_id, result) => {
                self.on_mcp_tool_result(&tool_call_id, result)
            }

            // ── PTY 터미널 ─────────────────────────────────────────
            Message::PtyToggle => self.toggle_pty(),
            Message::PtyStart => self.pty_start(),
            Message::PtyLine(line) => self.on_pty_line(&line),
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
            Message::WindowResized(width, _height) => {
                self.window_width = width;
                self.sidebar_width = if width < 900.0 {
                    crate::view::SIDEBAR_WIDTH_COMPACT
                } else if width > 1400.0 {
                    crate::view::SIDEBAR_WIDTH_WIDE
                } else {
                    crate::view::SIDEBAR_WIDTH
                };
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
            Message::FetchAccount => App::fetch_account_cmd(),
            Message::AccountLoaded(r) => self.on_account_loaded(r),
            Message::InputChanged(v) => self.on_input_changed(v),
            Message::InputAction(action) => {
                self.editor_content.perform(action);
                let new_text = self.editor_content.text();
                if new_text != self.input {
                    self.input = new_text;
                    return self.on_input_changed(self.input.clone());
                }
                Task::none()
            }
            Message::Send => {
                self.toast = None;
                self.send_message()
            }
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
            Message::ToggleBlockCollapse(id) => {
                if !self.ui.collapsed_blocks.remove(&id) {
                    self.ui.collapsed_blocks.insert(id);
                }
                Task::none()
            }
            Message::LinkClicked(uri) => self.on_link_clicked(&uri),
            Message::PickCwd => App::pick_cwd(),
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
            Message::NewChat => {
                self.toast = None;
                self.new_chat()
            }
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
            Message::ThemeHexChanged(field, value) => {
                self.on_theme_hex_changed(field, value);
                Task::none()
            }
            Message::ApplyTheme => self.apply_theme(),
            Message::ResetTheme => self.reset_theme(),
            Message::ThemeSaved(r) => self.on_theme_saved(r),
            Message::FileTreeToggle(p) => self.toggle_file_tree_dir(p),
            Message::RefreshFileTree => self.refresh_file_tree(),
            Message::SkeletonTick => {
                self.skeleton_phase = (self.skeleton_phase + 1) % 4;
                Task::none()
            }
            Message::ToggleTldrView(id) => {
                self.toggle_tldr_view(id);
                Task::none()
            }
            Message::CodeBlockHovered(id, hovered) => {
                if hovered {
                    self.hovered_code_blocks.insert(id);
                } else {
                    self.hovered_code_blocks.remove(&id);
                }
                Task::none()
            }
            Message::DismissToast => {
                self.toast = None;
                Task::none()
            }
        }
    }

    pub(crate) fn subscription(&self) -> Subscription<Message> {
        let event_sub = iced::event::listen_with(on_event);
        let interval = if self.streaming_block_id.is_some() {
            Duration::from_secs(15)
        } else {
            Duration::from_secs(60)
        };
        let timer_sub = iced::time::every(interval).map(|_| Message::AutoSave);
        let skeleton_sub = if self.streaming_block_id.is_some() {
            iced::time::every(Duration::from_millis(600)).map(|_| Message::SkeletonTick)
        } else {
            Subscription::none()
        };
        Subscription::batch(vec![event_sub, timer_sub, skeleton_sub])
    }
}
