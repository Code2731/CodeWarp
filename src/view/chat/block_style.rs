use iced::widget::container;
use iced::{Color, Shadow, Theme, Vector};

pub(super) fn block_container_style(
    is_user: bool,
    is_error_assistant: bool,
    is_hovered: bool,
    accent_user: Color,
    accent_assistant: Color,
    accent_error: Color,
) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let p = theme.extended_palette();
        let (bg, fg, accent) = if is_user {
            (
                iced::Color::from_rgba(
                    p.primary.weak.color.r,
                    p.primary.weak.color.g,
                    p.primary.weak.color.b,
                    if is_hovered { 0.40 } else { 0.30 },
                ),
                p.background.base.text,
                accent_user,
            )
        } else if is_error_assistant {
            (
                iced::Color::from_rgba(
                    p.danger.weak.color.r,
                    p.danger.weak.color.g,
                    p.danger.weak.color.b,
                    if is_hovered { 0.35 } else { 0.25 },
                ),
                p.background.base.text,
                accent_error,
            )
        } else {
            (
                iced::Color::from_rgba(
                    p.background.weak.color.r,
                    p.background.weak.color.g,
                    p.background.weak.color.b,
                    if is_hovered { 0.80 } else { 0.65 },
                ),
                p.background.base.text,
                accent_assistant,
            )
        };
        container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            border: iced::Border {
                color: if is_hovered {
                    Color::from_rgba(accent.r, accent.g, accent.b, 0.8)
                } else {
                    accent
                },
                width: if is_hovered { 1.5 } else { 1.0 },
                radius: 10.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                offset: Vector { x: 0.0, y: 2.0 },
                blur_radius: if is_hovered { 10.0 } else { 6.0 },
            },
            ..Default::default()
        }
    }
}
