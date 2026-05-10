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
const UI_FONT_FAMILY: &str = "Pretendard";
const SOFT_BG_ALPHA: f32 = 0.45;
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
        background: Some(with_alpha(p.background.strong.color, SOFT_BG_ALPHA).into()),
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
        style.background = Some(with_alpha(p.primary.base.color, SOFT_BG_ALPHA).into());
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
    font_with_weight(font::Weight::Semibold)
}

pub(crate) fn bold_font() -> Font {
    font_with_weight(font::Weight::Bold)
}

fn font_with_weight(weight: font::Weight) -> Font {
    let mut f = Font::with_name(UI_FONT_FAMILY);
    f.weight = weight;
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
    fn shorten_tail_keeps_text_when_exactly_max_chars() {
        assert_eq!(shorten_tail("codewarp", 8), "codewarp");
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

    #[test]
    fn shorten_tail_two_chars_keeps_last_char() {
        let out = shorten_tail("abcdef", 2);
        assert_eq!(out, "…f");
        assert_eq!(out.chars().count(), 2);
    }

    #[test]
    fn shorten_tail_three_chars_keeps_last_two_chars() {
        let out = shorten_tail("abcdef", 3);
        assert_eq!(out, "…ef");
        assert_eq!(out.chars().count(), 3);
    }

    #[test]
    fn shorten_tail_counts_unicode_by_char_not_byte() {
        let src = format!("ab{}cd", '\u{1F600}');
        let out = shorten_tail(&src, 4);
        assert_eq!(out, "…😀cd");
        assert_eq!(out.chars().count(), 4);
        assert!(out.ends_with("😀cd"));
    }
}

#[cfg(test)]
mod color_tests {
    use super::{disabled_text_color, with_alpha};
    use iced::Color;

    #[test]
    fn with_alpha_preserves_rgb_and_sets_alpha() {
        let base = Color::from_rgb(0.1, 0.2, 0.3);
        let out = with_alpha(base, 0.7);
        assert_eq!(out.r, base.r);
        assert_eq!(out.g, base.g);
        assert_eq!(out.b, base.b);
        assert_eq!(out.a, 0.7);
    }

    #[test]
    fn disabled_text_color_matches_design_value() {
        let c = disabled_text_color();
        assert_eq!(c.r, 0.92);
        assert_eq!(c.g, 0.94);
        assert_eq!(c.b, 0.98);
        assert_eq!(c.a, 0.75);
    }

    #[test]
    fn with_alpha_handles_alpha_bounds() {
        let base = Color::from_rgb(0.25, 0.5, 0.75);
        let transparent = with_alpha(base, 0.0);
        let opaque = with_alpha(base, 1.0);
        assert_eq!(transparent.a, 0.0);
        assert_eq!(opaque.a, 1.0);
        assert_eq!(transparent.r, base.r);
        assert_eq!(opaque.b, base.b);
    }
}

#[cfg(test)]
mod font_tests {
    use super::{bold_font, semibold_font, UI_FONT_FAMILY};
    use iced::font::Weight;
    use iced::Font;

    #[test]
    fn semibold_font_uses_semibold_weight() {
        assert_eq!(semibold_font().weight, Weight::Semibold);
    }

    #[test]
    fn bold_font_uses_bold_weight() {
        assert_eq!(bold_font().weight, Weight::Bold);
    }

    #[test]
    fn font_helpers_use_pretendard_family() {
        let expected_family = Font::with_name(UI_FONT_FAMILY).family;
        assert_eq!(semibold_font().family, expected_family);
        assert_eq!(bold_font().family, expected_family);
    }
}
