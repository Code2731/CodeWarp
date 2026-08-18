use crate::view::render_diff;
use crate::view::ui::{
    FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE, danger_btn, editor_input, panel_style, primary_btn,
    secondary_btn, semibold_font,
};
use crate::{AgentMode, App, Message};
use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{Binding, KeyPress};
use iced::widget::{Space, button, column, container, row, text, text_editor};
use iced::{Alignment, Element, Length, Theme};

impl App {
    fn view_mode_label(&self) -> Element<'_, Message> {
        button(
            text(self.agent_mode.label())
                .size(FS_LABEL)
                .font(semibold_font()),
        )
        .on_press(Message::ToggleAgentMode)
        .padding([7, 12])
        .style(secondary_btn)
        .into()
    }

    fn view_slash_hint(&self) -> Element<'_, Message> {
        if self.input.starts_with('/') {
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
        }
    }

    fn view_input_action_btn(&self) -> Element<'_, Message> {
        let send_disabled = self.input.trim().is_empty()
            || self.ui.compare_pending
            || self.streaming_block_id.is_some()
            || (!self.ui.compare_both && self.selected_model.is_none());
        if self.streaming_block_id.is_some() || self.ui.compare_pending {
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
        }
    }

    fn view_chat_editor(&self) -> Element<'_, Message> {
        let submit_msg = if self.ui.show_mention {
            Message::MentionConfirm
        } else {
            Message::Send
        };
        let mention_visible = self.ui.show_mention;
        text_editor(&self.editor_content)
            .placeholder("질문을 입력하세요…  (@파일 첨부, /plan, /build)")
            .size(FS_BODY)
            .line_height(1.55)
            .padding(10)
            .style(editor_input)
            .key_binding(move |press| {
                let KeyPress {
                    ref key, modifiers, ..
                } = press;
                let is_enter = matches!(key.as_ref(), Key::Named(Named::Enter));
                let is_shift = modifiers.shift();
                if mention_visible && matches!(key.as_ref(), Key::Named(Named::ArrowUp)) {
                    return Some(Binding::Custom(Message::MentionMove(-1)));
                }
                if mention_visible && matches!(key.as_ref(), Key::Named(Named::ArrowDown)) {
                    return Some(Binding::Custom(Message::MentionMove(1)));
                }
                if is_enter && !is_shift {
                    return Some(Binding::Custom(submit_msg.clone()));
                }
                if is_enter && is_shift {
                    return Some(Binding::Enter);
                }
                Binding::from_key_press(press)
            })
            .on_action(Message::InputAction)
            .into()
    }

    fn view_input_hint(&self) -> Element<'_, Message> {
        text("Enter: send | Shift+Enter: newline | Ctrl+K: commands | Ctrl+N: new chat")
            .size(FS_MICRO)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(theme.extended_palette().background.strong.color),
            })
            .into()
    }

    fn view_compare_diff(&self) -> Element<'_, Message> {
        if let (Some(old), Some(new)) = (&self.compare_old_text, &self.compare_new_text) {
            container(
                column![
                    row![
                        text("Compare Diff").size(FS_SUBTITLE).font(semibold_font()),
                        Space::new().width(Length::Fill),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                    render_diff(old, new),
                ]
                .spacing(6),
            )
            .padding(12)
            .style(panel_style)
            .width(Length::Fill)
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        }
    }

    pub(crate) fn view_stream(&self) -> Element<'_, Message> {
        let blocks_view: Element<Message> = self.view_blocks();
        let confirm_panel: Element<Message> = if self.ui.show_write_confirm {
            self.view_inline_confirm()
        } else {
            Space::new().height(Length::Shrink).into()
        };
        let mention_popup = self.view_mention_popup();
        let attach_row = self.view_attach_row();

        let input_row = row![
            self.view_mode_label(),
            self.view_chat_editor(),
            self.view_input_action_btn(),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        column![
            container(blocks_view)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding([14, 18]),
            container(self.view_compare_diff()).padding([0, 14]),
            container(confirm_panel).padding([0, 14]),
            container(self.view_slash_hint()).padding([0, 14]),
            container(mention_popup).padding([0, 14]),
            container(attach_row).padding([0, 14]),
            container(self.view_input_hint()).padding([0, 14]),
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
