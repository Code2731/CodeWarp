use iced::widget::scrollable::Scrollbar;
use iced::widget::{button, container, text_input};
use iced::{font, Border, Color, Font, Theme};

pub(crate) const FS_TITLE: f32 = 30.0;
pub(crate) const FS_SUBTITLE: f32 = 13.0;
pub(crate) const FS_BODY: f32 = 12.0;
pub(crate) const FS_LABEL: f32 = 11.0;
pub(crate) const FS_MICRO: f32 = 10.0;
pub(crate) const SPACE_XXS: f32 = 2.0;
pub(crate) const SPACE_XS: f32 = 4.0;
pub(crate) const SPACE_SM: f32 = 6.0;
pub(crate) const PAD_XXS: u16 = 2;
pub(crate) const PAD_XS: u16 = 4;
pub(crate) const PAD_SM: u16 = 6;
pub(crate) const PAD_MD: u16 = 8;
pub(crate) const PAD_LG: u16 = 14;
pub(crate) const CONTEXT_LIST_HEIGHT: f32 = 176.0;
pub(crate) const MAIN_ROW_SPACING: f32 = 10.0;
pub(crate) const MAIN_PAD_Y: u16 = 8;
pub(crate) const MAIN_PAD_X: u16 = 10;
pub(crate) const TOPBAR_ROW_SPACING: f32 = 10.0;
pub(crate) const TOPBAR_PAD_Y: u16 = 10;
pub(crate) const TOPBAR_PAD_X: u16 = 16;
pub(crate) const CONTROL_PAD_Y: u16 = 7;
pub(crate) const CONTROL_PAD_X: u16 = 12;
pub(crate) const SIDEBAR_WIDTH: f32 = 260.0;
pub(crate) const RIGHT_PANEL_WIDTH: f32 = 280.0;
pub(crate) const PANEL_SECTION_GAP_LG: f32 = 14.0;
pub(crate) const STATUSBAR_ROW_SPACING: f32 = 8.0;
pub(crate) const STATUSBAR_PAD_Y: u16 = 4;
pub(crate) const STATUSBAR_PAD_X: u16 = 14;
pub(crate) const SCROLL_GUTTER_PAD_X: u16 = 12;
const VSCROLLBAR_WIDTH: u32 = 10;
const VSCROLLBAR_MARGIN: u32 = 2;
const BORDER_WIDTH: f32 = 1.0;
const PANEL_RADIUS: f32 = 12.0;
const CONTROL_RADIUS: f32 = 10.0;
const CONTEXT_ITEM_RADIUS: f32 = 10.0;
const UI_FONT_FAMILY: &str = "Pretendard";
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

pub(crate) fn app_vscrollbar() -> Scrollbar {
    Scrollbar::new()
        .width(VSCROLLBAR_WIDTH)
        .scroller_width(VSCROLLBAR_WIDTH)
        .margin(VSCROLLBAR_MARGIN)
}

pub(crate) fn panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Color::from_rgba8(0x0b, 0x0f, 0x19, 0.94).into()),
        border: Border {
            color: Color::from_rgba8(0x1e, 0x29, 0x3b, 0.82),
            width: BORDER_WIDTH,
            radius: PANEL_RADIUS.into(),
        },
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

pub(crate) fn topbar_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Color::from_rgba8(0x11, 0x18, 0x27, 0.88).into()),
        border: Border {
            color: Color::from_rgba8(0x1e, 0x29, 0x3b, 0.95),
            width: BORDER_WIDTH,
            radius: PANEL_RADIUS.into(),
        },
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

pub(crate) fn primary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let mut style = button::Style {
        background: Some(Color::from_rgb8(0x0e, 0xa5, 0xe9).into()),
        text_color: p.primary.base.text,
        border: Border {
            color: Color::from_rgba8(0x10, 0xb9, 0x81, 0.75),
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    };
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background = Some(Color::from_rgb8(0x38, 0xbd, 0xf8).into());
        style.border.color = Color::from_rgba8(0x34, 0xd3, 0x99, 0.90);
    }
    if matches!(status, button::Status::Disabled) {
        style.background = Some(with_alpha(Color::from_rgb8(0x0e, 0xa5, 0xe9), 0.22).into());
        style.border.color = with_alpha(Color::from_rgb8(0x0e, 0xa5, 0xe9), 0.20);
        style.text_color = disabled_text_color();
    }
    style
}

pub(crate) fn secondary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let bg = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgba8(0x18, 0x23, 0x34, 0.96)
    } else {
        Color::from_rgba8(0x10, 0x16, 0x25, 0.85)
    };
    let border_color = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        Color::from_rgba8(0x0e, 0xa5, 0xe9, 0.92)
    } else {
        Color::from_rgba8(0x1e, 0x29, 0x3b, 0.88)
    };
    button::Style {
        background: Some(bg.into()),
        text_color: p.background.base.text,
        border: Border {
            color: border_color,
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    }
}

pub(crate) fn context_item_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Color::from_rgba8(0x0f, 0x16, 0x26, 0.88).into()),
        border: Border {
            color: Color::from_rgba8(0x1e, 0x29, 0x3b, 0.82),
            width: BORDER_WIDTH,
            radius: CONTEXT_ITEM_RADIUS.into(),
        },
        text_color: Some(p.background.base.text),
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
            color: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                p.danger.base.color
            } else {
                p.danger.strong.color
            },
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
            style.border.color = Color::from_rgba8(0x38, 0xbd, 0xf8, 0.65);
            style
        }
        text_input::Status::Focused { .. } => {
            style.border.color = Color::from_rgb8(0x0e, 0xa5, 0xe9);
            style
        }
        text_input::Status::Disabled => {
            style.background = with_alpha(p.background.weak.color, 0.40).into();
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

    #[test]
    fn shorten_tail_keeps_unicode_when_exactly_max_chars() {
        let src = format!("ab{}cd", '\u{1F600}');
        assert_eq!(src.chars().count(), 5);
        assert_eq!(shorten_tail(&src, 5), src);
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
