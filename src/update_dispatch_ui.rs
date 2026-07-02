impl App {
    pub(crate) fn dispatch_ui(&mut self, msg: &Message) -> Option<Task<Message>> {
        match msg {
            Message::InputChanged(v) => Some(self.on_input_changed(v.clone())),
            Message::InputAction(action) => {
                self.editor_content.perform(action.clone());
                let new_text = self.editor_content.text();
                if new_text != self.input {
                    self.input = new_text;
                    return Some(self.on_input_changed(self.input.clone()));
                }
                Some(Task::none())
            }
            Message::WindowResized(width, _height) => {
                self.window_width = *width;
                self.sidebar_width = if *width < 900.0 {
                    crate::view::SIDEBAR_WIDTH_COMPACT
                } else if *width > 1400.0 {
                    crate::view::SIDEBAR_WIDTH_WIDE
                } else {
                    crate::view::SIDEBAR_WIDTH
                };
                Some(Task::none())
            }
            Message::PickCwd => Some(App::pick_cwd()),
            Message::CwdPicked(maybe_path) => Some(self.apply_picked_cwd(maybe_path.clone())),
            Message::PickAttachment => Some(self.pick_attachment()),
            Message::AttachmentPicked(maybe_path) => Some(self.on_attachment_picked(maybe_path.clone())),
            Message::ApproveWrites => Some(self.approve_pending_writes()),
            Message::DenyWrites => Some(self.deny_pending_writes()),
            Message::ToggleConfirmExpand(idx) => Some(self.toggle_write_confirm_expand(*idx)),
            Message::DiscardWriteCall(idx) => Some(self.discard_write_call(*idx)),
            Message::ToggleFilterCoding(v) => Some(self.set_filter_coding(*v)),
            Message::ToggleFilterReasoning(v) => Some(self.set_filter_reasoning(*v)),
            Message::ToggleFilterGeneral(v) => Some(self.set_filter_general(*v)),
            Message::ToggleFilterFavorites(v) => Some(self.set_filter_favorites_only(*v)),
            Message::ToggleCompareBoth(v) => Some(self.set_compare_both(*v)),
            Message::CycleSortMode => Some(self.cycle_model_sort_mode()),
            Message::CycleSidebarWidth => Some(self.cycle_sidebar_width()),
            Message::SetAgentMode(mode) => Some(self.set_agent_mode(*mode)),
            Message::ToggleAgentMode => Some(self.toggle_agent_mode()),
            Message::OpenCommandPalette => Some(self.open_command_palette()),
            Message::CloseCommandPalette => Some(self.close_command_palette()),
            Message::CloseAllOverlays => Some(self.close_all_overlays()),
            Message::CommandPaletteChanged(v) => Some(self.update_command_palette_input(v.clone())),
            Message::ExecuteCommand(idx) => Some(self.execute_palette_command(*idx)),
            Message::ToggleFavorite => Some(self.toggle_favorite()),
            Message::ThemeHexChanged(field, value) => {
                self.on_theme_hex_changed(field.clone(), value.clone());
                Some(Task::none())
            }
            Message::ApplyTheme => Some(self.apply_theme()),
            Message::ResetTheme => Some(self.reset_theme()),
            Message::ThemeSaved(r) => Some(self.on_theme_saved(r.clone())),
            Message::FileTreeToggle(p) => Some(self.toggle_file_tree_dir(p.clone())),
            Message::RefreshFileTree => Some(self.refresh_file_tree()),
            Message::SkeletonTick => {
                self.skeleton_phase = (self.skeleton_phase + 1) % 4;
                Some(Task::none())
            }
            Message::ToggleTldrView(id) => {
                self.toggle_tldr_view(*id);
                Some(Task::none())
            }
            Message::CodeBlockHovered(id, hovered) => {
                if *hovered {
                    self.hovered_code_blocks.insert(*id);
                } else {
                    self.hovered_code_blocks.remove(id);
                }
                Some(Task::none())
            }
            Message::DismissToast => {
                self.toast = None;
                Some(Task::none())
            }
            Message::ToggleShortcutGuide => Some(self.toggle_shortcut_guide()),
            Message::MentionMove(delta) => Some(self.move_mention_selection(*delta)),
            Message::MentionConfirm => Some(self.confirm_mention()),
            Message::MentionCandidatesLoaded(paths) => Some(self.load_mention_candidates(paths.clone())),
            _ => None,
        }
    }
}
