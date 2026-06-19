use iced::widget::container;
use iced::Theme;

pub(super) fn block_container_style(
    is_user: bool,
    is_error_assistant: bool,
) -> impl Fn(&Theme) -> container::Style {
    move |theme: &Theme| {
        let p = theme.extended_palette();
        let (bg, fg, border) = if is_user {
            (
                iced::Color::from_rgba(
                    p.primary.weak.color.r,
                    p.primary.weak.color.g,
                    p.primary.weak.color.b,
                    0.35,
                ),
                p.background.base.text,
                p.primary.strong.color,
            )
        } else if is_error_assistant {
            (
                iced::Color::from_rgba(
                    p.danger.weak.color.r,
                    p.danger.weak.color.g,
                    p.danger.weak.color.b,
                    0.30,
                ),
                p.background.base.text,
                p.danger.strong.color,
            )
        } else {
            (
                iced::Color::from_rgba(
                    p.background.weak.color.r,
                    p.background.weak.color.g,
                    p.background.weak.color.b,
                    0.70,
                ),
                p.background.base.text,
                p.background.strong.color,
            )
        };
        container::Style {
            background: Some(bg.into()),
            text_color: Some(fg),
            border: iced::Border {
                color: border,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        }
    }
}
