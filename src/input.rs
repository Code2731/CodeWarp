use super::{AgentMode, Message};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

pub(crate) fn handle_key(key: &Key, modifiers: Modifiers) -> Option<Message> {
    if matches!(key.as_ref(), Key::Named(Named::Escape)) {
        return Some(Message::CloseAllOverlays);
    }
    if modifiers.command() {
        return match key.as_ref() {
            Key::Character("k") => Some(Message::OpenCommandPalette),
            Key::Character("n") => Some(Message::NewChat),
            Key::Character(",") => Some(Message::OpenSettings),
            Key::Character("`") => Some(Message::PtyToggle),
            Key::Character("p") if modifiers.shift() => {
                Some(Message::SetAgentMode(AgentMode::Plan))
            }
            Key::Character("b") if modifiers.shift() => {
                Some(Message::SetAgentMode(AgentMode::Build))
            }
            Key::Character("/") => Some(Message::ToggleShortcutGuide),
            _ => None,
        };
    }
    None
}

/// Keyboard and window events are routed through a function pointer for
/// `iced::event::listen_with`, so this remains a small stateless adapter.
pub(crate) fn on_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) => {
            match key.as_ref() {
                Key::Named(Named::ArrowUp) => Some(Message::MentionMove(-1)),
                Key::Named(Named::ArrowDown) => Some(Message::MentionMove(1)),
                _ => handle_key(&key, modifiers),
            }
        }
        iced::Event::Window(iced::window::Event::Resized(size)) => {
            Some(Message::WindowResized(size.width, size.height))
        }
        iced::Event::Window(iced::window::Event::CloseRequested) => {
            Some(Message::WindowCloseRequested)
        }
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        iced::Event::Window(
            iced::window::Event::FileHovered(_) | iced::window::Event::FilesHoveredLeft,
        ) => Some(Message::FileDragHover),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_escape_closes_overlays() {
        let result = handle_key(
            &Key::Named(iced::keyboard::key::Named::Escape),
            Modifiers::default(),
        );
        assert!(matches!(result, Some(Message::CloseAllOverlays)));
    }

    #[test]
    fn input_cmd_k_opens_palette() {
        let result = handle_key(&Key::Character("k".into()), Modifiers::COMMAND);
        assert!(matches!(result, Some(Message::OpenCommandPalette)));
    }

    #[test]
    fn input_cmd_n_new_chat() {
        let result = handle_key(&Key::Character("n".into()), Modifiers::COMMAND);
        assert!(matches!(result, Some(Message::NewChat)));
    }

    #[test]
    fn input_cmd_comma_settings() {
        let result = handle_key(&Key::Character(",".into()), Modifiers::COMMAND);
        assert!(matches!(result, Some(Message::OpenSettings)));
    }

    #[test]
    fn input_arrow_keys_are_not_handle_key() {
        assert!(
            handle_key(
                &Key::Named(iced::keyboard::key::Named::ArrowUp),
                Modifiers::default()
            )
            .is_none()
        );
        assert!(
            handle_key(
                &Key::Named(iced::keyboard::key::Named::ArrowDown),
                Modifiers::default()
            )
            .is_none()
        );
    }

    #[test]
    fn input_cmd_backtick_toggles_terminal() {
        let result = handle_key(&Key::Character("`".into()), Modifiers::COMMAND);
        assert!(matches!(result, Some(Message::PtyToggle)));
    }

    #[test]
    fn input_cmd_shift_p_sets_plan_mode() {
        let result = handle_key(
            &Key::Character("p".into()),
            Modifiers::COMMAND | Modifiers::SHIFT,
        );
        assert!(matches!(
            result,
            Some(Message::SetAgentMode(AgentMode::Plan))
        ));
    }

    #[test]
    fn input_cmd_shift_b_sets_build_mode() {
        let result = handle_key(
            &Key::Character("b".into()),
            Modifiers::COMMAND | Modifiers::SHIFT,
        );
        assert!(matches!(
            result,
            Some(Message::SetAgentMode(AgentMode::Build))
        ));
    }

    #[test]
    fn input_non_shortcut_returns_none() {
        assert!(handle_key(&Key::Character("z".into()), Modifiers::default()).is_none());
        assert!(
            handle_key(
                &Key::Named(iced::keyboard::key::Named::Enter),
                Modifiers::default()
            )
            .is_none()
        );
    }
}
