use iced::widget::{button, container, scrollable, text_input};
use iced::{Border, Color, Font, Shadow, Theme, Vector, font};

const BORDER_WIDTH: f32 = 1.0;
const PANEL_RADIUS: f32 = 12.0;
const CONTROL_RADIUS: f32 = 10.0;
const CONTEXT_ITEM_RADIUS: f32 = 10.0;
const INPUT_BG_ALPHA: f32 = 0.88;
const INPUT_PLACEHOLDER_ALPHA: f32 = 0.82;
const INPUT_SELECTION_ALPHA: f32 = 0.36;
const INPUT_DISABLED_ALPHA: f32 = 0.72;

const SHADOW_PANEL: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
    offset: Vector { x: 0.0, y: 4.0 },
    blur_radius: 12.0,
};
const SHADOW_BTN: Shadow = Shadow {
    color: Color::from_rgba(0.0, 0.0, 0.0, 0.25),
    offset: Vector { x: 0.0, y: 2.0 },
    blur_radius: 6.0,
};

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
        shadow: SHADOW_PANEL,
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

pub(crate) fn sub_panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(Color::from_rgba8(0x0d, 0x13, 0x1e, 0.94).into()),
        border: Border {
            color: Color::from_rgba8(0x1e, 0x29, 0x3b, 0.50),
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
        shadow: SHADOW_PANEL,
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

pub(crate) fn primary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    let mut style = button::Style {
        background: Some(Color::from_rgb8(0x0e, 0xa5, 0xe9).into()),
        text_color: p.primary.base.text,
        shadow: SHADOW_BTN,
        border: Border {
            color: Color::from_rgba8(0x38, 0xbd, 0xf8, 0.40),
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    };
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background = Some(Color::from_rgb8(0x38, 0xbd, 0xf8).into());
        style.border.color = Color::from_rgba8(0x56, 0xcf, 0xff, 0.60);
        style.shadow = Shadow {
            color: Color::from_rgba(0.06, 0.65, 0.91, 0.35),
            offset: Vector { x: 0.0, y: 2.0 },
            blur_radius: 10.0,
        };
    }
    if matches!(status, button::Status::Disabled) {
        style.background = Some(with_alpha(Color::from_rgb8(0x0e, 0xa5, 0xe9), 0.22).into());
        style.border.color = with_alpha(Color::from_rgb8(0x0e, 0xa5, 0xe9), 0.20);
        style.text_color = disabled_text_color();
        style.shadow = Shadow::default();
    }
    style
}

pub(crate) fn secondary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    if matches!(status, button::Status::Disabled) {
        return button::Style {
            background: Some(with_alpha(Color::from_rgb8(0x10, 0x16, 0x25), 0.40).into()),
            text_color: disabled_text_color(),
            shadow: Shadow::default(),
            border: Border {
                color: with_alpha(Color::from_rgb8(0x1e, 0x29, 0x3b), 0.30),
                width: BORDER_WIDTH,
                radius: CONTROL_RADIUS.into(),
            },
            ..Default::default()
        };
    }
    let (bg, border_color, shadow) =
        if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            (
                Color::from_rgba8(0x18, 0x23, 0x34, 0.96),
                Color::from_rgba8(0x0e, 0xa5, 0xe9, 0.92),
                Shadow {
                    color: Color::from_rgba(0.06, 0.65, 0.91, 0.18),
                    offset: Vector { x: 0.0, y: 2.0 },
                    blur_radius: 8.0,
                },
            )
        } else {
            (
                Color::from_rgba8(0x10, 0x16, 0x25, 0.85),
                Color::from_rgba8(0x1e, 0x29, 0x3b, 0.88),
                SHADOW_BTN,
            )
        };
    button::Style {
        background: Some(bg.into()),
        text_color: p.background.base.text,
        shadow,
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
        shadow: SHADOW_PANEL,
        text_color: Some(p.background.base.text),
        ..Default::default()
    }
}

pub(crate) fn danger_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    if matches!(status, button::Status::Disabled) {
        return button::Style {
            background: Some(with_alpha(p.danger.base.color, 0.22).into()),
            text_color: disabled_text_color(),
            shadow: Shadow::default(),
            border: Border {
                color: with_alpha(p.danger.base.color, 0.20),
                width: BORDER_WIDTH,
                radius: CONTROL_RADIUS.into(),
            },
            ..Default::default()
        };
    }
    let (bg, shadow) = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        (
            p.danger.strong.color,
            Shadow {
                color: Color::from_rgba(0.94, 0.36, 0.44, 0.25),
                offset: Vector { x: 0.0, y: 2.0 },
                blur_radius: 8.0,
            },
        )
    } else {
        (p.danger.base.color, SHADOW_BTN)
    };
    button::Style {
        background: Some(bg.into()),
        text_color: p.danger.base.text,
        shadow,
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

const SCROLL_BG_ALPHA: f32 = 0.06;
const SCROLL_RADIUS: f32 = 999.0;

fn scroll_rail(theme: &Theme) -> scrollable::Rail {
    let p = theme.extended_palette();
    scrollable::Rail {
        background: Some(iced::Background::Color(Color::from_rgba(
            p.background.base.color.r,
            p.background.base.color.g,
            p.background.base.color.b,
            SCROLL_BG_ALPHA,
        ))),
        border: Border {
            radius: SCROLL_RADIUS.into(),
            ..Border::default()
        },
        scroller: scrollable::Scroller {
            background: iced::Background::Color(Color::from_rgba(
                p.background.strong.text.r,
                p.background.strong.text.g,
                p.background.strong.text.b,
                0.35,
            )),
            border: Border {
                radius: SCROLL_RADIUS.into(),
                ..Border::default()
            },
        },
    }
}

pub(crate) fn dark_scrollable(theme: &Theme, _status: scrollable::Status) -> scrollable::Style {
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scroll_rail(theme),
        horizontal_rail: scroll_rail(theme),
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: iced::Background::Color(Color::from_rgba(
                theme.extended_palette().background.strong.color.r,
                theme.extended_palette().background.strong.color.g,
                theme.extended_palette().background.strong.color.b,
                0.08,
            )),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
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
