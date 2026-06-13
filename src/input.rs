use super::{AgentMode, Message};
use iced::keyboard::key::Named;
use iced::keyboard::{Key, Modifiers};

pub(crate) fn handle_key(key: Key, modifiers: Modifiers) -> Option<Message> {
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
                _ => handle_key(key, modifiers),
            }
        }
        iced::Event::Window(iced::window::Event::CloseRequested) => {
            Some(Message::WindowCloseRequested)
        }
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        iced::Event::Window(iced::window::Event::FileHovered(_)) => Some(Message::FileDragHover),
        iced::Event::Window(iced::window::Event::FilesHoveredLeft) => Some(Message::FileDragHover),
        _ => None,
    }
}
