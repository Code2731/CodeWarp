use crate::session::theme_presets;
use crate::view::ui::{
    FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE, app_vscrollbar, dark_scrollable, primary_btn,
    secondary_btn, section_header, semibold_font,
};
use crate::{App, Message};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, row, scrollable, text, text_input};
use iced::{Alignment, Color, Element, Length, Theme};

const THEME_FIELDS: &[(&str, &str)] = &[
    ("background", "배경색"),
    ("text", "글자색"),
    ("primary", "강조색 (Primary)"),
    ("success", "성공 (Success)"),
    ("warning", "경고 (Warning)"),
    ("danger", "위험 (Danger)"),
    ("accent_user", "사용자 블록 강조"),
    ("accent_assistant", "어시스턴트 블록 강조"),
    ("accent_error", "에러 블록 강조"),
];

impl App {
    pub(crate) fn view_theme_tab(&self) -> Element<'_, Message> {
        let mut col = column![].spacing(4);

        for (idx, (field, label)) in THEME_FIELDS.iter().enumerate() {
            let hex_val = self
                .ui
                .theme_hex_inputs
                .get(idx)
                .cloned()
                .unwrap_or_default();
            let preview_color = parse_hex(&hex_val).unwrap_or(Color::from_rgb8(0x03, 0x07, 0x12));

            let preview = container(text(""))
                .width(Length::Fixed(24.0))
                .height(Length::Fixed(24.0))
                .style(move |_: &Theme| container::Style {
                    background: Some(preview_color.into()),
                    border: iced::Border {
                        color: Color::from_rgb8(0x1e, 0x29, 0x3b),
                        width: 1.0,
                        radius: 6.0.into(),
                    },
                    ..Default::default()
                });

            let input = text_input(&format!("#{:06x}", 0x0ea5e9), &hex_val)
                .on_input(move |v| Message::ThemeHexChanged(field.to_string(), v))
                .padding([4, 8])
                .size(FS_BODY)
                .width(Length::Fixed(140.0));

            let row = row![
                preview,
                text(*label).size(FS_LABEL).width(Length::Fixed(160.0)),
                input,
            ]
            .spacing(8)
            .align_y(Alignment::Center);

            col = col.push(row);
        }

        let msg_el: Element<Message> = if self.theme_apply_msg.is_empty() {
            iced::widget::Space::new().height(Length::Shrink).into()
        } else {
            text(&self.theme_apply_msg).size(FS_MICRO).into()
        };
        let action_row = row![
            button(text("적용").size(FS_BODY))
                .on_press(Message::ApplyTheme)
                .padding([6, 16])
                .style(primary_btn),
            button(text("기본값 리셋").size(FS_BODY))
                .on_press(Message::ResetTheme)
                .padding([6, 16])
                .style(secondary_btn),
            Space::new().width(Length::Fill),
            msg_el,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let motion_control = iced::widget::toggler(self.ui.reduced_motion)
            .label("동작 줄이기")
            .text_size(FS_BODY)
            .on_toggle(Message::SetReducedMotion);

        let motion_section = column![
            section_header("접근성"),
            text("스트리밍 중 깜빡임과 펄스를 고정된 표시 상태로 유지합니다.").size(FS_LABEL),
            motion_control,
        ]
        .spacing(6);

        let presets = theme_presets();
        let mut preset_row = row![].spacing(6);
        for (i, p) in presets.iter().enumerate() {
            let swatch = |c: [u8; 3]| {
                container(text(""))
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(14.0))
                    .style(move |_: &Theme| container::Style {
                        background: Some(Color::from_rgb8(c[0], c[1], c[2]).into()),
                        border: iced::Border {
                            color: Color::from_rgba8(0x1e, 0x29, 0x3b, 0.60),
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        ..Default::default()
                    })
            };
            let btn = button(
                row![
                    swatch(p.config.background),
                    swatch(p.config.primary),
                    swatch(p.config.text),
                    text(p.name).size(FS_SUBTITLE).font(semibold_font()),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .on_press(Message::ApplyThemePreset(i))
            .padding([6, 10])
            .style(secondary_btn);
            preset_row = preset_row.push(btn);
        }

        container(
            column![
                section_header("커스텀 테마"),
                scrollable(
                    column![
                        col,
                        Space::new().height(Length::Fixed(8.0)),
                        action_row,
                        Space::new().height(Length::Fixed(12.0)),
                        motion_section,
                        Space::new().height(Length::Fixed(12.0)),
                        section_header("프리셋 테마"),
                        text("자주 쓰는 색 조합을 클릭 한 번으로 불러옵니다.").size(FS_LABEL),
                        preset_row,
                    ]
                    .spacing(6),
                )
                .direction(Direction::Vertical(app_vscrollbar()))
                .style(dark_scrollable),
            ]
            .spacing(6),
        )
        .padding(8)
        .width(Length::Fill)
        .into()
    }
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::from_rgb8(r, g, b))
}
