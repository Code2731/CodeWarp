use iced::widget::container;
use iced::{Color, Shadow, Theme, Vector};

pub(super) fn user_accent() -> Color {
    Color::from_rgb8(0x0e, 0xa5, 0xe9)
}

pub(super) fn assistant_accent() -> Color {
    Color::from_rgb8(0xa7, 0x8b, 0xfa)
}

pub(super) fn error_accent() -> Color {
    Color::from_rgb8(0xf0, 0x5b, 0x6f)
}

pub(super) fn block_container_style(
    is_user: bool,
    is_error_assistant: bool,
) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let p = theme.extended_palette();
        let (bg, fg, accent) = if is_user {
            (
                iced::Color::from_rgba(
                    p.primary.weak.color.r,
                    p.primary.weak.color.g,
                    p.primary.weak.color.b,
                    0.30,
                ),
                p.background.base.text,
                user_accent(),
            )
        } else if is_error_assistant {
            (
                iced::Color::from_rgba(
                    p.danger.weak.color.r,
                    p.danger.weak.color.g,
                    p.danger.weak.color.b,
                    0.25,
                ),
                p.background.base.text,
                error_accent(),
            )
        } else {
            (
                iced::Color::from_rgba(
                    p.background.weak.color.r,
                    p.background.weak.color.g,
                    p.background.weak.color.b,
                    0.65,
                ),
                p.background.base.text,
                assistant_accent(),
            )
        };
        container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            border: iced::Border {
                color: accent,
                width: 1.0,
                radius: 10.0.into(),
            },
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.12),
                offset: Vector { x: 0.0, y: 2.0 },
                blur_radius: 6.0,
            },
            ..Default::default()
        }
    }
}
