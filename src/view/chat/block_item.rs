use super::block_style::block_container_style;
use crate::view::CodewarpViewer;
use crate::view::ui::{
    FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE, primary_btn, secondary_btn, semibold_font,
};
use crate::{App, Block, BlockBody, Message, ViewMode};
use iced::widget::markdown;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, row, text, text_editor, tooltip};
use iced::{Alignment, Color, Element, Font, Length, Shadow, Theme, Vector};

impl App {
    pub(crate) fn view_block_item<'a>(
        &'a self,
        b: &'a Block,
        i: usize,
        last_user_idx: Option<usize>,
        last_asst_idx: Option<usize>,
        streaming: bool,
    ) -> Element<'a, Message> {
        if let BlockBody::ToolResult {
            name,
            summary,
            success,
        } = &b.body
        {
            return self.view_tool_result_chip(name, summary, *success);
        }
        let role_label = b.body.role_label();
        let has_content = !b.body.is_empty_for_history();
        let is_collapsed = self.ui.collapsed_blocks.contains(&b.id);
        let is_assistant = matches!(&b.body, BlockBody::Assistant(_));
        let header = row![
            self.view_collapse_btn(b.id, is_assistant, has_content, streaming, is_collapsed),
            text(role_label).size(FS_LABEL).font(semibold_font()),
            self.view_model_label(b),
            Space::new().width(Length::Fill),
            self.view_action_btn(
                i,
                last_user_idx,
                last_asst_idx,
                streaming,
                has_content,
                is_assistant,
                is_collapsed
            ),
            self.view_toggle_btn(b.id, b.view_mode, has_content, is_assistant, is_collapsed),
            self.view_copy_btn(b.id, has_content, is_collapsed),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let body_view = self.view_block_body(b, streaming, is_collapsed);

        let is_user = matches!(&b.body, BlockBody::User(_));
        let is_error_assistant =
            matches!(&b.body, BlockBody::Assistant(_)) && b.body.to_text().contains("[ERROR]");
        let a_user = self.theme_config.accent_user();
        let a_asst = self.theme_config.accent_assistant();
        let a_err = self.theme_config.accent_error();
        let accent_color = if is_user {
            a_user
        } else if is_error_assistant {
            a_err
        } else {
            a_asst
        };
        let accent_bar = self.view_accent_bar(accent_color, self.streaming_block_id == Some(b.id));
        let inner =
            container(column![header, body_view, self.view_block_apply_section(b),].spacing(6))
                .padding(12)
                .width(Length::Fill)
                .style(block_container_style(
                    is_user,
                    is_error_assistant,
                    a_user,
                    a_asst,
                    a_err,
                ));
        row![accent_bar, inner]
            .spacing(8)
            .align_y(Alignment::Start)
            .into()
    }

    fn view_tool_result_chip(
        &self,
        name: &str,
        summary: &str,
        success: bool,
    ) -> Element<'_, Message> {
        let accent_color = if success {
            Color::from_rgba8(0x4a, 0xd0, 0x7c, 1.0)
        } else {
            Color::from_rgba8(0xf0, 0x5b, 0x6f, 1.0)
        };
        let accent_bar = container(text(""))
            .width(Length::Fixed(3.0))
            .height(Length::Fill)
            .style(move |_: &Theme| container::Style {
                background: Some(accent_color.into()),
                border: iced::Border {
                    radius: 2.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });
        let icon = if success { "✓" } else { "✗" };
        let label = row![
            text(format!(" {icon} {name}"))
                .size(FS_LABEL)
                .font(Font::with_name("JetBrains Mono")),
            text(format!(" → {summary}"))
                .size(FS_LABEL)
                .font(Font::with_name("JetBrains Mono")),
        ]
        .spacing(0)
        .align_y(Alignment::Center);
        let chip = container(
            row![accent_bar, label]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding([4, 10])
        .style({
            let border_c = Color::from_rgba(accent_color.r, accent_color.g, accent_color.b, 0.35);
            move |_: &Theme| container::Style {
                background: Some(Color::from_rgba8(0x0f, 0x16, 0x26, 0.88).into()),
                border: iced::Border {
                    color: border_c,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.18),
                    offset: Vector { x: 0.0, y: 2.0 },
                    blur_radius: 6.0,
                },
                ..Default::default()
            }
        });
        chip.into()
    }

    fn view_collapse_btn(
        &self,
        id: u64,
        is_assistant: bool,
        has_content: bool,
        streaming: bool,
        is_collapsed: bool,
    ) -> Element<'_, Message> {
        if is_assistant && has_content && !streaming {
            let label = if is_collapsed { "▸" } else { "▾" };
            tooltip(
                button(text(label).size(FS_MICRO))
                    .on_press(Message::ToggleBlockCollapse(id))
                    .padding([2, 6])
                    .style(secondary_btn),
                text("접기/펼치기").size(FS_MICRO),
                Position::Bottom,
            )
            .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        }
    }

    fn view_copy_btn(
        &self,
        id: u64,
        has_content: bool,
        is_collapsed: bool,
    ) -> Element<'_, Message> {
        if has_content && !is_collapsed {
            tooltip(
                button(text("복사").size(FS_MICRO))
                    .on_press(Message::CopyBlock(id))
                    .padding([2, 8])
                    .style(secondary_btn),
                text("블록 복사").size(FS_MICRO),
                Position::Bottom,
            )
            .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        }
    }

    fn view_toggle_btn(
        &self,
        id: u64,
        view_mode: ViewMode,
        has_content: bool,
        is_assistant: bool,
        is_collapsed: bool,
    ) -> Element<'_, Message> {
        if has_content && is_assistant && !is_collapsed {
            let label = match view_mode {
                ViewMode::Rendered => "원문",
                ViewMode::Raw => "예쁘게",
            };
            tooltip(
                button(text(label).size(FS_MICRO))
                    .on_press(Message::ToggleBlockView(id))
                    .padding([2, 8])
                    .style(secondary_btn),
                text("보기 모드 전환").size(FS_MICRO),
                Position::Bottom,
            )
            .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn view_action_btn(
        &self,
        i: usize,
        last_user_idx: Option<usize>,
        last_asst_idx: Option<usize>,
        streaming: bool,
        has_content: bool,
        is_assistant: bool,
        is_collapsed: bool,
    ) -> Element<'_, Message> {
        if streaming {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        } else if Some(i) == last_user_idx {
            tooltip(
                button(text("✎").size(FS_MICRO))
                    .on_press(Message::EditLastUser)
                    .padding([2, 8])
                    .style(secondary_btn),
                text("마지막 질문 수정").size(FS_MICRO),
                Position::Bottom,
            )
            .into()
        } else if Some(i) == last_asst_idx && is_assistant && has_content && !is_collapsed {
            tooltip(
                button(text("↻").size(FS_MICRO))
                    .on_press(Message::RegenerateLast)
                    .padding([2, 8])
                    .style(secondary_btn),
                text("재생성").size(FS_MICRO),
                Position::Bottom,
            )
            .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        }
    }

    fn view_model_label(&self, b: &Block) -> Element<'_, Message> {
        match &b.model {
            Some(m) => text(format!("· {m}")).size(FS_MICRO).into(),
            None => Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into(),
        }
    }

    fn view_block_body<'a>(
        &'a self,
        b: &'a Block,
        _streaming: bool,
        is_collapsed: bool,
    ) -> Element<'a, Message> {
        if is_collapsed {
            return Self::view_collapsed_preview(b);
        }
        if self.streaming_block_id == Some(b.id) && self.streaming_raw.is_empty() {
            return super::skeleton::view_skeleton_block(self.skeleton_phase);
        }
        match (&b.body, b.view_mode) {
            (BlockBody::User(s), _) => text(s).size(FS_SUBTITLE).into(),
            (BlockBody::Assistant(content), ViewMode::Raw) => {
                if self.streaming_block_id == Some(b.id) && !self.streaming_raw.is_empty() {
                    let cursor = if super::skeleton::cursor_visible(self.skeleton_phase) {
                        text("▊").size(FS_SUBTITLE)
                    } else {
                        text(" ").size(FS_SUBTITLE)
                    };
                    row![text(&self.streaming_raw).size(FS_SUBTITLE), cursor]
                        .spacing(0)
                        .into()
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
                markdown::view_with(
                    b.md_items.iter(),
                    settings,
                    &CodewarpViewer {
                        hovered_set: &self.hovered_code_blocks,
                        on_hover: Message::CodeBlockHovered,
                    },
                )
            }
            (BlockBody::ToolResult { .. }, _) => {
                text("도구 결과 렌더링 경로 오류").size(FS_SUBTITLE).into()
            }
        }
    }

    fn view_accent_bar(&self, accent_color: Color, is_streaming: bool) -> Element<'_, Message> {
        container(text(""))
            .width(Length::Fixed(3.0))
            .height(Length::Fill)
            .style(move |_: &Theme| {
                let c = if is_streaming {
                    Color::from_rgba(
                        accent_color.r,
                        accent_color.g,
                        accent_color.b,
                        0.6 + 0.4 * (self.skeleton_phase as f32 / 3.0),
                    )
                } else {
                    accent_color
                };
                container::Style {
                    background: Some(c.into()),
                    border: iced::Border {
                        radius: 2.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
            .into()
    }

    fn view_collapsed_preview(b: &Block) -> Element<'_, Message> {
        let preview = match &b.body {
            BlockBody::Assistant(content) => {
                let text = content.text();
                let lines: Vec<&str> = text.lines().take(3).collect();
                lines.join("\n")
            }
            BlockBody::User(s) => {
                if s.len() > 120 {
                    format!("{}…", &s[..120])
                } else {
                    s.clone()
                }
            }
            _ => String::new(),
        };
        text(preview)
            .size(FS_SUBTITLE)
            .style(|_: &Theme| iced::widget::text::Style {
                color: Some(Color::from_rgba(0.67, 0.67, 0.67, 0.7)),
            })
            .into()
    }

    fn view_block_apply_section(&self, b: &Block) -> Element<'_, Message> {
        if b.apply_candidates.is_empty() {
            return Space::new().height(Length::Shrink).into();
        }
        let tldr_open = self.tldr_expanded.contains(&b.id);
        let mut col = column![
            row![
                text("적용 가능한 변경사항")
                    .size(FS_LABEL)
                    .font(semibold_font()),
                Space::new().width(Length::Fill),
                button(text("TL;DR").size(FS_MICRO))
                    .on_press(Message::ToggleTldrView(b.id))
                    .padding([2, 8])
                    .style(secondary_btn),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        ]
        .spacing(4);
        if tldr_open && let Some(entries) = self.tldr_data.get(&b.id) {
            col = col.push(super::tldr::view_tldr_summary(b.id, entries));
        }
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
    }
}
