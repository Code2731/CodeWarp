// update_settings_ui.rs — Settings UI toggle/input/palette methods (main.rs child module)
use super::*;
use crate::view::{SIDEBAR_WIDTH_COMPACT, SIDEBAR_WIDTH_WIDE};
use iced::Task;

impl App {
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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
