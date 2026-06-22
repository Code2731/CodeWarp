use crate::view::ui::{
    danger_btn, field_input, panel_style, primary_btn, secondary_btn, semibold_font, FS_BODY,
    FS_LABEL, FS_MICRO, FS_SUBTITLE,
};
use crate::{AgentMode, App, Message};
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length, Theme};

impl App {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn view_stream(&self) -> Element<'_, Message> {
        let blocks_view: Element<Message> = self.view_blocks();

        let send_disabled = self.input.trim().is_empty()
            || self.compare_pending
            || self.streaming_block_id.is_some()
            || (!self.compare_both && self.selected_model.is_none());

        // 입력창 좌측 모드 라벨 (클릭으로 Plan ↔ Build 토글)
        let mode_label = button(
            text(self.agent_mode.label())
                .size(FS_LABEL)
                .font(semibold_font()),
        )
        .on_press(Message::ToggleAgentMode)
        .padding([7, 12])
        .style(secondary_btn);

        // 슬래시 hint: 입력이 '/'로 시작하면 입력창 위에 명령 버튼 줄
        let slash_hint: Element<Message> = if self.input.starts_with('/') {
            container(
                row![
                    text("커맨드:").size(FS_LABEL).font(semibold_font()),
                    button(text("/plan").size(FS_LABEL).font(semibold_font()))
                        .on_press(Message::SetAgentMode(AgentMode::Plan))
                        .padding([3, 10])
                        .style(if self.agent_mode == AgentMode::Plan {
                            primary_btn
                        } else {
                            secondary_btn
                        }),
                    button(text("/build").size(FS_LABEL).font(semibold_font()))
                        .on_press(Message::SetAgentMode(AgentMode::Build))
                        .padding([3, 10])
                        .style(if self.agent_mode == AgentMode::Build {
                            primary_btn
                        } else {
                            secondary_btn
                        }),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([6, 10])
            .style(panel_style)
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        let mention_popup = self.view_mention_popup();
        let attach_row = self.view_attach_row();

        let action_btn: Element<Message> =
            if self.streaming_block_id.is_some() || self.compare_pending {
                button(text("■ 중지").size(FS_SUBTITLE).font(semibold_font()))
                    .on_press(Message::StopStream)
                    .padding([8, 18])
                    .style(danger_btn)
                    .into()
            } else {
                button(text("전송  ⏎").size(FS_SUBTITLE).font(semibold_font()))
                    .on_press_maybe(if send_disabled {
                        None
                    } else {
                        Some(Message::Send)
                    })
                    .padding([8, 18])
                    .style(primary_btn)
                    .into()
            };

        // mention 팝업 활성 시 Enter → MentionConfirm, 비활성 시 → Send
        let submit_msg = if self.show_mention {
            Message::MentionConfirm
        } else {
            Message::Send
        };

        let input_row = row![
            mode_label,
            text_input(
                "질문을 입력하세요…  (@파일 첨부, /plan, /build)",
                &self.input
            )
            .on_input(Message::InputChanged)
            .on_submit(submit_msg)
            .padding(10)
            .size(FS_BODY)
            .style(field_input),
            action_btn,
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        let input_hint = text("Enter: send | Ctrl+K: commands | Ctrl+N: new chat")
            .size(FS_MICRO)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.strong.color),
            });

        let confirm_panel: Element<Message> = if self.show_write_confirm {
            self.view_inline_confirm()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        column![
            container(blocks_view)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([14, 18]),
            container(confirm_panel).padding([0, 14]),
            container(slash_hint).padding([0, 14]),
            container(mention_popup).padding([0, 14]),
            container(attach_row).padding([0, 14]),
            container(input_hint).padding([0, 14]),
            container(input_row)
                .padding([10, 14])
                .style(panel_style)
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }
}
