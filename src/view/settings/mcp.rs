use crate::view::ui::*;
use crate::*;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

impl App {
    pub(crate) fn view_mcp_settings(&self) -> Element<'_, Message> {
        let header = text("MCP 서버 (Model Context Protocol)")
            .size(14)
            .font(semibold_font());
        let hint = text("stdio MCP 서버를 등록해 AI tool을 동적으로 확장합니다.").size(FS_LABEL);

        let mut server_list = column![].spacing(4);
        for (i, s) in self.mcp_servers.iter().enumerate() {
            let tool_count = self
                .mcp_tools
                .iter()
                .filter(|t| t.server_name == s.name)
                .count();
            let label = format!("{} — {} (tool {}개)", s.name, s.command, tool_count);
            server_list = server_list.push(
                row![
                    text(shorten_tail(&label, 72))
                        .size(FS_BODY)
                        .width(Length::Fill),
                    button(text("✕").size(FS_LABEL))
                        .on_press(Message::RemoveMcpServer(i))
                        .padding([2, 6])
                        .style(danger_btn),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
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

        container(column![header, hint, empty_state, server_list, add_row].spacing(8))
            .padding([14, 16])
            .width(Length::Fill)
            .style(panel_style)
            .into()
    }
}
