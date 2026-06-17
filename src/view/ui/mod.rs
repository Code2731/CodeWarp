use iced::widget::scrollable::Scrollbar;

pub(crate) mod styles;
pub(crate) use styles::*;

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
pub(crate) const SIDEBAR_WIDTH_COMPACT: f32 = 200.0;
pub(crate) const SIDEBAR_WIDTH: f32 = 260.0;
pub(crate) const SIDEBAR_WIDTH_WIDE: f32 = 340.0;
pub(crate) const RIGHT_PANEL_WIDTH: f32 = 280.0;
pub(crate) const PANEL_SECTION_GAP_LG: f32 = 14.0;
pub(crate) const STATUSBAR_ROW_SPACING: f32 = 8.0;
pub(crate) const STATUSBAR_PAD_Y: u16 = 4;
pub(crate) const STATUSBAR_PAD_X: u16 = 14;
pub(crate) const SCROLL_GUTTER_PAD_X: u16 = 12;

const VSCROLLBAR_WIDTH: u32 = 10;
const VSCROLLBAR_MARGIN: u32 = 2;

pub(crate) fn app_vscrollbar() -> Scrollbar {
    Scrollbar::new()
        .width(VSCROLLBAR_WIDTH)
        .scroller_width(VSCROLLBAR_WIDTH)
        .margin(VSCROLLBAR_MARGIN)
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
mod ui_tests;
