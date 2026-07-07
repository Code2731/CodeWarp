use crate::view::ui::{
    FS_BODY, FS_LABEL, FS_MICRO, danger_btn, field_input, panel_style, primary_btn, section_header,
    shorten_tail,
};
use crate::{App, Message};
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, mouse_area, row, text, text_input, tooltip};
use iced::{Alignment, Color, Element, Length, Theme};

impl App {
    pub(crate) fn view_mcp_settings(&self) -> Element<'_, Message> {
        let hint = text("stdio MCP 서버를 등록해 AI tool을 동적으로 확장합니다.").size(FS_LABEL);

        let mut server_list = column![].spacing(4);
        for (i, s) in self.mcp_servers.iter().enumerate() {
            let tool_count = self
                .mcp_tools
                .iter()
                .filter(|t| t.server_name == s.name)
                .count();
            let label = format!("{} — {} (tool {tool_count}개)", s.name, s.command);
            let is_hovered = self.hovered_mcp_idx == Some(i);
            let row_widget = row![
                text(shorten_tail(&label, 72))
                    .size(FS_BODY)
                    .width(Length::Fill),
                tooltip(
                    button(text("✕").size(FS_LABEL))
                        .on_press(Message::RemoveMcpServer(i))
                        .padding([2, 6])
                        .style(danger_btn),
                    text("서버 제거").size(FS_MICRO),
                    Position::Bottom,
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            let item = container(row_widget)
                .padding([4, 6])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    container::Style {
                        background: Some(
                            (if is_hovered {
                                Color::from_rgba(
                                    p.primary.base.color.r,
                                    p.primary.base.color.g,
                                    p.primary.base.color.b,
                                    0.06,
                                )
                            } else {
                                Color::from_rgba(0.0, 0.0, 0.0, 0.0)
                            })
                            .into(),
                        ),
                        border: iced::Border {
                            color: if is_hovered {
                                Color::from_rgba(
                                    p.primary.base.color.r,
                                    p.primary.base.color.g,
                                    p.primary.base.color.b,
                                    0.30,
                                )
                            } else {
                                Color::from_rgba(0.0, 0.0, 0.0, 0.0)
                            },
                            width: 1.0,
                            radius: 6.0.into(),
                        },
                        ..Default::default()
                    }
                });
            server_list = server_list.push(
                mouse_area(item)
                    .on_enter(Message::McpServerHovered(Some(i)))
                    .on_exit(Message::McpServerHovered(None)),
            );
        }

        let add_row = row![
            text_input("서버 이름 (예: filesystem)", &self.mcp_input.name_input)
                .on_input(Message::McpNameChanged)
                .padding(6)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fixed(140.0)),
            text_input(
                "명령 (예: npx -y @modelcontextprotocol/server-filesystem /tmp)",
                &self.mcp_input.command_input
            )
            .on_input(Message::McpCommandChanged)
            .on_submit(Message::AddMcpServer)
            .padding(6)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fill),
            button(text("추가").size(FS_BODY))
                .on_press(Message::AddMcpServer)
                .padding([6, 12])
                .style(primary_btn),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let empty_state: Element<Message> = if self.mcp_servers.is_empty() {
            container(text("등록된 MCP 서버가 없습니다. 먼저 서버를 추가해 주세요.").size(FS_LABEL))
                .padding([8, 10])
                .style(panel_style)
                .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        container(
            column![
                section_header("MCP 서버"),
                hint,
                empty_state,
                server_list,
                add_row
            ]
            .spacing(8),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }
}
