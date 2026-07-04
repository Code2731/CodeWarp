// view_palette.rs — Command palette view (view child module)
use super::ui::{
    FS_BODY, FS_LABEL, FS_SUBTITLE, app_vscrollbar, bold_font, dark_scrollable, field_input,
    panel_style, secondary_btn, semibold_font,
};
use crate::{App, Message};
use iced::widget::scrollable::Direction;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input,
};
use iced::{Color, Element, Length, Theme};

impl App {
    pub(super) fn view_command_palette(&self) -> Element<'_, Message> {
        let header = text("명령 팔레트").size(18).font(bold_font());
        let hint = column![
            text("탐색  Esc 닫기 · Ctrl+K 토글").size(FS_LABEL),
            text("작업  Ctrl+N 새 채팅 · Ctrl+, 설정").size(FS_LABEL),
            text("모드  Ctrl+Shift+P 계획 · Ctrl+Shift+B 빌드").size(FS_LABEL),
        ]
        .spacing(2);
        let input = text_input("명령 검색…", &self.ui.command_palette_input)
            .on_input(Message::CommandPaletteChanged)
            .on_submit(Message::ExecuteCommand(0))
            .padding(10)
            .size(FS_BODY)
            .style(field_input);

        let filtered = self.filtered_palette_commands();
        let mut list = column![].spacing(4);
        if filtered.is_empty() {
            list = list.push(text("(매칭 없음)").size(FS_BODY));
        } else {
            for (i, cmd) in filtered.iter().enumerate() {
                let is_hovered = self.hovered_palette_idx == Some(i);
                let item = button(
                    column![
                        text(cmd.label).size(FS_SUBTITLE).font(semibold_font()),
                        text(cmd.hint).size(FS_LABEL),
                    ]
                    .spacing(2),
                )
                .on_press(Message::ExecuteCommand(i))
                .padding([6, 10])
                .width(Length::Fill)
                .style(secondary_btn);
                list = list.push(
                    container(
                        mouse_area(item)
                            .on_enter(Message::PaletteHovered(Some(i)))
                            .on_exit(Message::PaletteHovered(None)),
                    )
                    .style(move |theme: &Theme| {
                        if is_hovered {
                            let p = theme.extended_palette();
                            container::Style {
                                background: Some(
                                    Color::from_rgba(
                                        p.primary.base.color.r,
                                        p.primary.base.color.g,
                                        p.primary.base.color.b,
                                        0.06,
                                    )
                                    .into(),
                                ),
                                border: iced::Border {
                                    radius: 10.0.into(),
                                    ..Default::default()
                                },
                                ..Default::default()
                            }
                        } else {
                            container::Style::default()
                        }
                    }),
                );
            }
        }

        let body = column![
            header,
            hint,
            Space::new().height(Length::Fixed(8.0)),
            input,
            Space::new().height(Length::Fixed(8.0)),
            scrollable(list)
                .direction(Direction::Vertical(app_vscrollbar(),))
                .style(dark_scrollable)
                .height(Length::Fixed(320.0)),
            Space::new().height(Length::Fixed(8.0)),
            row![
                Space::new().width(Length::Fill),
                button(text("닫기").size(FS_BODY))
                    .on_press(Message::CloseCommandPalette)
                    .padding([4, 12])
                    .style(secondary_btn),
            ],
        ]
        .spacing(4);

        container(body)
            .padding(20)
            .width(Length::FillPortion(3))
            .max_width(560.0)
            .style(panel_style)
            .into()
    }
}
