use crate::palette::{
    PALETTE_COMMANDS, PALETTE_INPUT_ID, move_palette_selection, palette_selection_for_result_count,
    resolve_palette_selection,
};
use iced::widget::{Id, operation};

impl App {
    pub(crate) fn open_command_palette(&mut self) -> iced::Task<Message> {
        self.ui.show_command_palette = true;
        self.ui.command_palette_input.clear();
        self.ui.active_palette_idx = palette_selection_for_result_count(PALETTE_COMMANDS.len());
        operation::focus(Id::new(PALETTE_INPUT_ID))
    }

    pub(crate) fn close_command_palette(&mut self) -> iced::Task<Message> {
        self.ui.show_command_palette = false;
        self.ui.active_palette_idx = None;
        self.hovered_palette_idx = None;
        iced::Task::none()
    }

    pub(crate) fn update_command_palette_input(&mut self, value: String) -> iced::Task<Message> {
        self.ui.command_palette_input = value;
        self.ui.active_palette_idx =
            palette_selection_for_result_count(self.filtered_palette_commands().len());
        iced::Task::none()
    }

    pub(crate) fn execute_palette_command(&mut self, idx: usize) -> iced::Task<Message> {
        let filtered = self.filtered_palette_commands();
        let Some(cmd) = filtered.get(idx) else {
            return iced::Task::none();
        };
        let action = cmd.action;
        let _ = self.close_command_palette();
        self.ui.command_palette_input.clear();
        match action {
            super::PaletteAction::NewChat => iced::Task::done(Message::NewChat),
            super::PaletteAction::PlanMode => {
                iced::Task::done(Message::SetAgentMode(super::AgentMode::Plan))
            }
            super::PaletteAction::BuildMode => {
                iced::Task::done(Message::SetAgentMode(super::AgentMode::Build))
            }
            super::PaletteAction::OpenSettings => iced::Task::done(Message::OpenSettings),
            super::PaletteAction::PickCwd => iced::Task::done(Message::PickCwd),
            super::PaletteAction::CycleSort => iced::Task::done(Message::CycleSortMode),
            super::PaletteAction::ToggleFavorite => iced::Task::done(Message::ToggleFavorite),
        }
    }

    pub(crate) fn move_palette_selection_or_mention(
        &mut self,
        delta: i32,
    ) -> iced::Task<Message> {
        if self.ui.show_command_palette {
            self.ui.active_palette_idx = move_palette_selection(
                self.ui.active_palette_idx,
                delta,
                self.filtered_palette_commands().len(),
            );
            return iced::Task::none();
        }
        iced::Task::done(Message::MentionMove(delta))
    }

    pub(crate) fn resolve_active_palette_command(&self) -> Option<usize> {
        if self.ui.show_command_palette {
            resolve_palette_selection(
                self.ui.active_palette_idx,
                self.filtered_palette_commands().len(),
            )
        } else {
            None
        }
    }

    fn palette_activation_message(&self) -> Option<Message> {
        self.resolve_active_palette_command()
            .map(Message::ExecuteCommand)
    }

    pub(crate) fn activate_palette_selection(&mut self) -> iced::Task<Message> {
        self.palette_activation_message()
            .map_or_else(iced::Task::none, iced::Task::done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_keyboard_moves_active_selection() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::OpenCommandPalette);
        let _ = app.update(Message::PaletteMove(1));

        assert_eq!(app.ui.active_palette_idx, Some(1));
    }

    #[test]
    fn palette_keyboard_enter_resolves_active_command() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::OpenCommandPalette);
        let _ = app.update(Message::PaletteMove(1));
        let task = app.update(Message::ActivatePaletteSelection);

        assert!(matches!(
            app.palette_activation_message(),
            Some(Message::ExecuteCommand(1))
        ));
        assert_eq!(task.units(), 1);
    }

    #[test]
    fn palette_keyboard_closed_is_noop() {
        let (mut app, _) = App::new();
        let initial_active = app.ui.active_palette_idx;

        let _ = app.update(Message::PaletteMove(1));

        assert_eq!(app.ui.active_palette_idx, initial_active);
    }

    #[test]
    fn palette_keyboard_empty_enter_is_noop() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::OpenCommandPalette);
        let _ = app.update(Message::CommandPaletteChanged("no matching command".into()));
        let task = app.update(Message::ActivatePaletteSelection);

        assert!(app.ui.show_command_palette);
        assert_eq!(app.ui.active_palette_idx, None);
        assert!(app.palette_activation_message().is_none());
        assert_eq!(task.units(), 0);
    }

    #[test]
    fn palette_filter_resets_active_selection_through_dispatch() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::OpenCommandPalette);
        let _ = app.update(Message::PaletteMove(1));
        let _ = app.update(Message::CommandPaletteChanged("설정".into()));

        assert_eq!(app.ui.active_palette_idx, Some(0));
    }

    #[test]
    fn palette_mouse_entry_updates_active_and_hover_state() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::OpenCommandPalette);
        let _ = app.update(Message::PaletteHovered(Some(2)));

        assert_eq!(app.ui.active_palette_idx, Some(2));
        assert_eq!(app.hovered_palette_idx, Some(2));
    }

    #[test]
    fn palette_close_clears_active_and_hover_state() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::OpenCommandPalette);
        let _ = app.update(Message::PaletteHovered(Some(2)));
        let _ = app.update(Message::CloseCommandPalette);

        assert_eq!(app.ui.active_palette_idx, None);
        assert_eq!(app.hovered_palette_idx, None);
    }
}
