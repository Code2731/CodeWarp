use super::ui::*;
use super::CodewarpViewer;
use crate::*;
use iced::widget::markdown;
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, text_editor, Space};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    pub(crate) fn view_blocks(&self) -> Element<'_, Message> {
        if self.blocks.is_empty() {
            self.view_empty_chat()
        } else {
            let last_user_idx = last_user_block_idx(&self.blocks);
            let last_asst_idx = last_assistant_block_idx(&self.blocks);
            let streaming = self.streaming_block_id.is_some();
            let mut col = column![].spacing(10).width(Length::Fill);
            for (i, b) in self.blocks.iter().enumerate() {
                if let BlockBody::ToolResult {
                    name,
                    summary,
                    success,
                } = &b.body
                {
                    let icon = if *success { "✓" } else { "✗" };
                    let chip = container(
                        text(format!("{} {} → {}", icon, name, summary))
                            .size(FS_LABEL)
                            .font(Font::with_name("JetBrains Mono")),
                    )
                    .padding([4, 10])
                    .style({
                        let success = *success;
                        move |theme: &Theme| {
                            let p = theme.extended_palette();
                            container::Style {
                                background: Some(p.background.strong.color.into()),
                                border: iced::Border {
                                    color: if success {
                                        p.success.weak.color
                                    } else {
                                        p.danger.weak.color
                                    },
                                    width: 1.0,
                                    radius: 10.0.into(),
                                },
                                ..Default::default()
                            }
                        }
                    });
                    col = col.push(chip);
                    continue;
                }
                let role_label = b.body.role_label();
                let has_content = !b.body.is_empty_for_history();
                let copy_btn: Element<Message> = if has_content {
                    button(text("복사").size(FS_MICRO))
                        .on_press(Message::CopyBlock(b.id))
                        .padding([2, 8])
                        .style(secondary_btn)
                        .into()
                } else {
                    Space::new()
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                };
                let toggle_btn: Element<Message> =
                    if has_content && matches!(&b.body, BlockBody::Assistant(_)) {
                        let label = match b.view_mode {
                            ViewMode::Rendered => "원문",
                            ViewMode::Raw => "예쁘게",
                        };
                        button(text(label).size(FS_MICRO))
                            .on_press(Message::ToggleBlockView(b.id))
                            .padding([2, 8])
                            .style(secondary_btn)
                            .into()
                    } else {
                        Space::new()
                            .width(Length::Shrink)
                            .height(Length::Shrink)
                            .into()
                    };
                let action_btn: Element<Message> = if streaming {
                    Space::new()
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                } else if Some(i) == last_user_idx && matches!(&b.body, BlockBody::User(_)) {
                    button(text("✎").size(FS_MICRO))
                        .on_press(Message::EditLastUser)
                        .padding([2, 8])
                        .style(secondary_btn)
                        .into()
                } else if Some(i) == last_asst_idx
                    && matches!(&b.body, BlockBody::Assistant(_))
                    && has_content
                {
                    button(text("↻").size(FS_MICRO))
                        .on_press(Message::RegenerateLast)
                        .padding([2, 8])
                        .style(secondary_btn)
                        .into()
                } else {
                    Space::new()
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into()
                };
                let model_label: Element<Message> = match &b.model {
                    Some(m) => text(format!("· {}", m)).size(FS_MICRO).into(),
                    None => Space::new()
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .into(),
                };
                let header = row![
                    text(role_label).size(FS_LABEL).font(semibold_font()),
                    model_label,
                    Space::new().width(Length::Fill),
                    action_btn,
                    toggle_btn,
                    copy_btn,
                ]
                .spacing(6)
                .align_y(Alignment::Center);

                let body_view: Element<Message> = match (&b.body, b.view_mode) {
                    (BlockBody::User(s), _) => text(s).size(FS_SUBTITLE).into(),
                    (BlockBody::Assistant(content), ViewMode::Raw) => {
                        if self.streaming_block_id == Some(b.id) && !self.streaming_raw.is_empty() {
                            text(&self.streaming_raw).size(FS_SUBTITLE).into()
                        } else {
                            let id = b.id;
                            text_editor(content)
                                .on_action(move |action| Message::EditorAction(id, action))
                                .height(Length::Shrink)
                                .padding(0)
                                .size(FS_SUBTITLE)
                                .into()
                        }
                    }
                    (BlockBody::Assistant(_), ViewMode::Rendered) => {
                        let mut settings: markdown::Settings = (&self.theme()).into();
                        settings.style.inline_code_font = Font::with_name("JetBrains Mono");
                        settings.style.code_block_font = Font::with_name("JetBrains Mono");
                        markdown::view_with(b.md_items.iter(), settings, &CodewarpViewer)
                    }
                    (BlockBody::ToolResult { .. }, _) => {
                        text("도구 결과 렌더링 경로 오류").size(FS_SUBTITLE).into()
                    }
                };

                let apply_section: Element<Message> = if b.apply_candidates.is_empty() {
                    Space::new().height(Length::Shrink).into()
                } else {
                    let mut col = column![text("적용 가능한 변경사항")
                        .size(FS_LABEL)
                        .font(semibold_font())]
                    .spacing(4);
                    for (ci, (cand, applied)) in b.apply_candidates.iter().enumerate() {
                        let label = if *applied {
                            format!("✓ {} ({} bytes)", cand.path, cand.content.len())
                        } else {
                            format!("📝 {} ({} bytes)", cand.path, cand.content.len())
                        };
                        let btn: Element<Message> = if *applied {
                            text("적용됨").size(FS_LABEL).into()
                        } else {
                            button(text("적용").size(FS_LABEL))
                                .on_press(Message::ApplyChange(b.id, ci))
                                .padding([2, 10])
                                .style(primary_btn)
                                .into()
                        };
                        col = col.push(
                            row![
                                text(label)
                                    .size(FS_BODY)
                                    .font(Font::with_name("JetBrains Mono")),
                                Space::new().width(Length::Fill),
                                btn,
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        );
                    }
                    col.into()
                };

                let is_user = matches!(&b.body, BlockBody::User(_));
                let is_error_assistant = matches!(&b.body, BlockBody::Assistant(_))
                    && b.body.to_text().contains("[ERROR]");
                let block_view = container(column![header, body_view, apply_section].spacing(6))
                    .padding(12)
                    .width(Length::Fill)
                    .style(move |theme: &Theme| {
                        let p = theme.extended_palette();
                        let (bg, fg, border) = if is_user {
                            (
                                iced::Color::from_rgba(
                                    p.primary.weak.color.r,
                                    p.primary.weak.color.g,
                                    p.primary.weak.color.b,
                                    0.35,
                                ),
                                p.background.base.text,
                                p.primary.strong.color,
                            )
                        } else if is_error_assistant {
                            (
                                iced::Color::from_rgba(
                                    p.danger.weak.color.r,
                                    p.danger.weak.color.g,
                                    p.danger.weak.color.b,
                                    0.30,
                                ),
                                p.background.base.text,
                                p.danger.strong.color,
                            )
                        } else {
                            (
                                iced::Color::from_rgba(
                                    p.background.weak.color.r,
                                    p.background.weak.color.g,
                                    p.background.weak.color.b,
                                    0.70,
                                ),
                                p.background.base.text,
                                p.background.strong.color,
                            )
                        };
                        container::Style {
                            background: Some(bg.into()),
                            text_color: Some(fg),
                            border: iced::Border {
                                color: border,
                                width: 1.0,
                                radius: 10.0.into(),
                            },
                            ..Default::default()
                        }
                    });
                col = col.push(block_view);
            }
            scrollable(container(col).padding([0, SCROLL_GUTTER_PAD_X]))
                .id(self.stream_id.clone())
                .on_scroll(Message::StreamScrolled)
                .direction(Direction::Vertical(app_vscrollbar()))
                .height(Length::Fill)
                .into()
        }
    }
}
