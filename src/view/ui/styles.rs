use iced::widget::{button, container, text_input};
use iced::{font, Border, Color, Font, Theme};

const BORDER_WIDTH: f32 = 1.0;
const PANEL_RADIUS: f32 = 12.0;
const CONTROL_RADIUS: f32 = 10.0;
const CONTEXT_ITEM_RADIUS: f32 = 10.0;
const INPUT_BG_ALPHA: f32 = 0.88;
const INPUT_PLACEHOLDER_ALPHA: f32 = 0.82;
const INPUT_SELECTION_ALPHA: f32 = 0.36;
const INPUT_DISABLED_ALPHA: f32 = 0.72;

pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(color.r, color.g, color.b, alpha)
}

pub(crate) fn disabled_text_color() -> Color {
    Color::from_rgba(0.92, 0.94, 0.98, 0.75)
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

pub(crate) const UI_FONT_FAMILY: &str = "Pretendard";

pub(crate) fn font_with_weight(weight: font::Weight) -> Font {
    let mut f = Font::with_name(UI_FONT_FAMILY);
    f.weight = weight;
    f
}

pub(crate) fn semibold_font() -> Font {
    font_with_weight(font::Weight::Semibold)
}

pub(crate) fn bold_font() -> Font {
    font_with_weight(font::Weight::Bold)
}
