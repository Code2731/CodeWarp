use iced::widget::{button, container, text_input};
use iced::{font, Border, Color, Font, Theme};

pub(crate) const FS_TITLE: f32 = 30.0;
pub(crate) const FS_SUBTITLE: f32 = 13.0;
pub(crate) const FS_BODY: f32 = 12.0;
pub(crate) const FS_LABEL: f32 = 11.0;
pub(crate) const FS_MICRO: f32 = 10.0;
const BORDER_WIDTH: f32 = 1.0;
const PANEL_RADIUS: f32 = 12.0;
const CONTROL_RADIUS: f32 = 10.0;
const TOPBAR_BG_ALPHA: f32 = 0.45;
const DISABLED_BG_ALPHA: f32 = 0.45;
const INPUT_BG_ALPHA: f32 = 0.88;
const INPUT_PLACEHOLDER_ALPHA: f32 = 0.82;
const INPUT_SELECTION_ALPHA: f32 = 0.36;
const INPUT_DISABLED_ALPHA: f32 = 0.72;

fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(color.r, color.g, color.b, alpha)
}

fn disabled_text_color() -> Color {
    Color::from_rgba(0.92, 0.94, 0.98, 0.75)
}

pub(crate) fn panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        border: Border {
            color: p.background.strong.color,
            width: BORDER_WIDTH,
            radius: PANEL_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn topbar_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(with_alpha(p.background.strong.color, TOPBAR_BG_ALPHA).into()),
        border: Border {
            color: p.background.strong.color,
            width: BORDER_WIDTH,
            radius: PANEL_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn primary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let mut style = button::Style {
        background: Some(p.primary.base.color.into()),
        text_color: p.primary.base.text,
        border: Border {
            color: p.primary.strong.color,
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    };
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background = Some(p.primary.strong.color.into());
    }
    if matches!(status, button::Status::Disabled) {
        style.background = Some(with_alpha(p.primary.base.color, DISABLED_BG_ALPHA).into());
        style.text_color = disabled_text_color();
    }
    style
}

pub(crate) fn secondary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let bg = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        p.background.strong.color
    } else {
        p.background.weak.color
    };
    button::Style {
        background: Some(bg.into()),
        text_color: p.background.base.text,
        border: Border {
            color: p.background.strong.color,
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn danger_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let bg = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        p.danger.strong.color
    } else {
        p.danger.base.color
    };
    button::Style {
        background: Some(bg.into()),
        text_color: p.danger.base.text,
        border: Border {
            color: p.danger.strong.color,
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn field_input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let p = theme.extended_palette();
    let mut style = text_input::default(theme, text_input::Status::Active);
    style.background = with_alpha(p.background.base.color, INPUT_BG_ALPHA).into();
    style.border = Border {
        color: p.background.strong.color,
        width: BORDER_WIDTH,
        radius: CONTROL_RADIUS.into(),
    };
    style.placeholder = with_alpha(p.background.weak.text, INPUT_PLACEHOLDER_ALPHA);
    style.selection = with_alpha(p.primary.strong.color, INPUT_SELECTION_ALPHA);

    match status {
        text_input::Status::Hovered => {
            style.border.color = p.background.base.text;
            style
        }
        text_input::Status::Focused { .. } => {
            style.border.color = p.primary.base.color;
            style
        }
        text_input::Status::Disabled => {
            style.background = with_alpha(p.background.weak.color, INPUT_DISABLED_ALPHA).into();
            style.value = with_alpha(p.background.strong.text, INPUT_DISABLED_ALPHA);
            style
        }
        text_input::Status::Active => style,
    }
}

pub(crate) fn semibold_font() -> Font {
    let mut f = Font::with_name("Pretendard");
    f.weight = font::Weight::Semibold;
    f
}

pub(crate) fn bold_font() -> Font {
    let mut f = Font::with_name("Pretendard");
    f.weight = font::Weight::Bold;
    f
}

pub(crate) fn shorten_tail(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    if max_chars == 1 {
        return "…".to_string();
    }
    let tail: String = s
        .chars()
        .rev()
        .take(max_chars.saturating_sub(1))
        .collect::<Vec<char>>()
        .into_iter()
        .rev()
        .collect();
    format!("…{}", tail)
}

#[cfg(test)]
mod tests {
    use super::shorten_tail;

    #[test]
    fn shorten_tail_keeps_short_text() {
        assert_eq!(shorten_tail("codewarp", 16), "codewarp");
    }

    #[test]
    fn shorten_tail_truncates_to_max_chars() {
        let out = shorten_tail("abcdefghijklmnopqrstuvwxyz", 8);
        assert_eq!(out, "…tuvwxyz");
        assert_eq!(out.chars().count(), 8);
    }

    #[test]
    fn shorten_tail_handles_zero_and_one() {
        assert_eq!(shorten_tail("abcdef", 0), "");
        assert_eq!(shorten_tail("abcdef", 1), "…");
    }
}
