use super::ui::*;
use super::CodewarpViewer;
use crate::*;
use iced::widget::markdown;
use iced::widget::scrollable::Direction;
use iced::widget::{
    button, column, container, row, scrollable, text, text_editor, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    fn view_empty_chat(&self) -> Element<'_, Message> {
        const EXAMPLES: &[&str] = &[
            "이 프로젝트의 의존성을 알려줘",
            "src/main.rs의 첫 30줄을 요약해줘",
            "examples/hello.rs 만들어줘",
        ];
        let title = text("CodeWarp").size(FS_TITLE).font(bold_font());
        let subtitle =
            text("AI 코딩 데스크톱 — Plan으로 안전하게 둘러보고, Build로 변경 적용").size(FS_BODY);
        let about = column![
            text("CodeWarp란?").size(FS_LABEL).font(semibold_font()),
            text("Rust 네이티브 Iced 기반의 AI 코딩 데스크톱입니다. 프로젝트 컨텍스트, 도구 실행, 클라우드와 로컬 provider를 한 화면에서 다룹니다.")
                .size(FS_BODY)
                .line_height(1.35),
        ]
        .spacing(SPACE_XXS);

        let mut examples_col = column![text("다음을 시도해보세요")
            .size(FS_LABEL)
            .font(semibold_font())]
        .spacing(SPACE_SM);
        for ex in EXAMPLES {
            examples_col = examples_col.push(
                button(text(format!("▸ {}", ex)).size(FS_SUBTITLE))
                    .on_press(Message::InputChanged((*ex).to_string()))
                    .padding([7, 12])
                    .width(Length::Fill)
                    .style(secondary_btn),
            );
        }

        let modes = column![
            text("모드 (입력창 좌측 라벨 클릭 또는 슬래시)")
                .size(FS_LABEL)
                .font(semibold_font()),
            text("/plan   계획 먼저, 도구는 read-only").size(FS_BODY),
            text("/build  변경 적용 (write_file, run_command)").size(FS_BODY),
        ]
        .spacing(2);

        let shortcut_hint = |keys: &'static str, label: &'static str| {
            container(
                row![
                    text(keys)
                        .size(FS_LABEL)
                        .font(Font::with_name("JetBrains Mono")),
                    Space::new().width(Length::Fill),
                    text(label).size(FS_BODY),
                ]
                .spacing(SPACE_SM)
                .align_y(Alignment::Center),
            )
            .padding([PAD_XS, PAD_MD])
            .style(context_item_style)
        };
        let shortcuts = column![
            text("키보드 단축키").size(FS_LABEL).font(semibold_font()),
            shortcut_hint("Ctrl+K", "명령 팔레트"),
            shortcut_hint("Ctrl+N", "새 채팅"),
            shortcut_hint("Ctrl+,", "설정"),
            shortcut_hint("Ctrl+Shift+P / B", "Plan / Build 모드"),
        ]
        .spacing(SPACE_XS);

        container(
            column![
                title,
                subtitle,
                about,
                Space::new().height(Length::Fixed(PANEL_SECTION_GAP_LG)),
                examples_col,
                Space::new().height(Length::Fixed(PANEL_SECTION_GAP_LG)),
                modes,
                Space::new().height(Length::Fixed(SPACE_SM)),
                shortcuts,
            ]
            .spacing(SPACE_SM)
            .max_width(560),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(panel_style)
        .into()
    }

    pub(crate) fn view_stream(&self) -> Element<'_, Message> {
        let blocks_view: Element<Message> = if self.blocks.is_empty() {
            self.view_empty_chat()
        } else {
            // 마지막 user/assistant 블록 인덱스 — regenerate/edit 버튼 노출 결정
            let last_user_idx = last_user_block_idx(&self.blocks);
            let last_asst_idx = last_assistant_block_idx(&self.blocks);
            let streaming = self.streaming_block_id.is_some();
            let mut col = column![].spacing(10).width(Length::Fill);
            for (i, b) in self.blocks.iter().enumerate() {
                // ToolResult 블록은 별도 작은 chip으로 표시
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
                // 마지막 user/assistant 블록에만 ✎ / ↻ 버튼 (streaming 중엔 숨김)
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

                // Apply 후보 카드 (assistant 블록만, 후보 있을 때)
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
        };

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

        // @-mention 드롭다운 (show_mention 시 입력창 위에 표시)
        let mention_popup: Element<Message> = if self.show_mention {
            let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
            if filtered.is_empty() {
                Space::new().height(Length::Shrink).into()
            } else {
                let mut list = column![].spacing(2);
                for (i, path) in filtered.iter().enumerate() {
                    let label = path.to_string_lossy().to_string();
                    let is_selected = i == self.mention_selected;
                    let btn = button(text(label).size(FS_BODY))
                        .on_press(Message::MentionConfirm)
                        .padding([6, 10])
                        .width(Length::Fill)
                        .style(if is_selected {
                            primary_btn
                        } else {
                            secondary_btn
                        });
                    list = list.push(btn);
                }
                container(
                    scrollable(list)
                        .direction(Direction::Vertical(app_vscrollbar()))
                        .height(Length::Shrink),
                )
                .padding([4, 0])
                .style(|theme: &Theme| {
                    let p = theme.extended_palette();
                    container::Style {
                        background: Some(p.background.strong.color.into()),
                        border: iced::Border {
                            color: p.primary.base.color,
                            width: 1.0,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into()
            }
        } else {
            Space::new().height(Length::Shrink).into()
        };

        // 첨부 파일 칩 행 (attached_files가 있을 때만)
        let attach_row: Element<Message> = if !self.attached_files.is_empty() {
            let mut chips = row![].spacing(6).align_y(Alignment::Center);
            for (i, (path, _)) in self.attached_files.iter().enumerate() {
                let rel_path = path.strip_prefix(&self.cwd).unwrap_or(path.as_path());
                let name = shorten_tail(&rel_path.display().to_string(), 36);
                chips = chips.push(
                    container(
                        row![
                            text(format!("📄 {name}")).size(FS_LABEL),
                            button(text("✕").size(FS_MICRO))
                                .on_press(Message::RemoveAttachment(i))
                                .padding([1, 4])
                                .style(secondary_btn),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .padding([3, 8])
                    .style(|theme: &Theme| {
                        let p = theme.extended_palette();
                        container::Style {
                            background: Some(p.background.strong.color.into()),
                            border: iced::Border {
                                color: p.primary.weak.color,
                                width: 1.0,
                                radius: 12.0.into(),
                            },
                            ..Default::default()
                        }
                    }),
                );
            }
            container(
                scrollable(chips)
                    .direction(Direction::Horizontal(hscrollbar()))
                    .width(Length::Fill),
            )
            .padding([4, 0])
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

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
