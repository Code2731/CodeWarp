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

const PANEL_BG_ALPHA: f32 = 0.94;
const TOPBAR_BG_ALPHA: f32 = 0.88;
const CONTEXT_BG_ALPHA: f32 = 0.88;
const BORDER_ALPHA: f32 = 0.82;
const SUB_BORDER_ALPHA: f32 = 0.50;
const TOPBAR_BORDER_ALPHA: f32 = 0.95;

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

fn bg_weak_alpha(theme: &Theme, alpha: f32) -> Color {
    let p = theme.extended_palette();
    Color::from_rgba(
        p.background.weak.color.r,
        p.background.weak.color.g,
        p.background.weak.color.b,
        alpha,
    )
}

fn border_color(theme: &Theme, alpha: f32) -> Color {
    let p = theme.extended_palette();
    Color::from_rgba(
        p.background.strong.color.r,
        p.background.strong.color.g,
        p.background.strong.color.b,
        alpha,
    )
}

pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color {
    Color::from_rgba(color.r, color.g, color.b, alpha)
}

pub(crate) fn disabled_text_color() -> Color {
    Color::from_rgba(0.92, 0.94, 0.98, 0.75)
}

pub(crate) fn panel_style(theme: &Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(bg_weak_alpha(theme, PANEL_BG_ALPHA).into()),
        border: Border {
            color: border_color(theme, BORDER_ALPHA),
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
        background: Some(bg_weak_alpha(theme, PANEL_BG_ALPHA).into()),
        border: Border {
            color: border_color(theme, SUB_BORDER_ALPHA),
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
        background: Some(bg_weak_alpha(theme, TOPBAR_BG_ALPHA).into()),
        border: Border {
            color: border_color(theme, TOPBAR_BORDER_ALPHA),
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
        background: Some(p.primary.base.color.into()),
        text_color: p.primary.base.text,
        shadow: SHADOW_BTN,
        border: Border {
            color: with_alpha(p.primary.strong.color, 0.40),
            width: BORDER_WIDTH,
            radius: CONTROL_RADIUS.into(),
        },
        ..Default::default()
    };
    if matches!(status, button::Status::Hovered | button::Status::Pressed) {
        style.background = Some(p.primary.strong.color.into());
        style.border.color = with_alpha(p.primary.base.color, 0.60);
        style.shadow = Shadow {
            color: with_alpha(p.primary.strong.color, 0.35),
            offset: Vector { x: 0.0, y: 2.0 },
            blur_radius: 10.0,
        };
    }
    if matches!(status, button::Status::Disabled) {
        style.background = Some(with_alpha(p.primary.base.color, 0.22).into());
        style.border.color = with_alpha(p.primary.base.color, 0.20);
        style.text_color = disabled_text_color();
        style.shadow = Shadow::default();
    }
    style
}

pub(crate) fn secondary_btn(theme: &Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    if matches!(status, button::Status::Disabled) {
        return button::Style {
            background: Some(with_alpha(p.background.weak.color, 0.40).into()),
            text_color: disabled_text_color(),
            shadow: Shadow::default(),
            border: Border {
                color: with_alpha(p.background.strong.color, 0.30),
                width: BORDER_WIDTH,
                radius: CONTROL_RADIUS.into(),
            },
            ..Default::default()
        };
    }
    let (bg, border_color, shadow) =
        if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            (
                Color::from_rgba(
                    p.background.strong.color.r,
                    p.background.strong.color.g,
                    p.background.strong.color.b,
                    0.96,
                ),
                with_alpha(p.primary.base.color, 0.92),
                Shadow {
                    color: with_alpha(p.primary.strong.color, 0.18),
                    offset: Vector { x: 0.0, y: 2.0 },
                    blur_radius: 8.0,
                },
            )
        } else {
            (
                bg_weak_alpha(theme, 0.85),
                border_color(theme, 0.88),
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
        background: Some(bg_weak_alpha(theme, CONTEXT_BG_ALPHA).into()),
        border: Border {
            color: border_color(theme, BORDER_ALPHA),
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
                color: with_alpha(p.danger.base.color, 0.25),
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
            style.border.color = with_alpha(p.primary.strong.color, 0.65);
            style
        }
        text_input::Status::Focused { .. } => {
            style.border.color = p.primary.base.color;
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
