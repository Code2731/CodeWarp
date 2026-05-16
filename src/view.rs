// view.rs — App 뷰 메서드 (main.rs child module)
use super::*;
mod ui;
use iced::widget::markdown::{self};
use iced::widget::scrollable::Direction;
use iced::widget::{
    button, checkbox, column, combo_box, container, pick_list, row, scrollable, stack, text,
    text_editor, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Theme};
use ui::*;

impl App {
    pub(crate) fn view(&self) -> Element<'_, Message> {
        let topbar = self.view_topbar();

        let main_view: Element<Message> = row![
            self.view_sidebar(),
            container(self.view_stream())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(panel_style),
            self.view_rightpanel(),
        ]
        .spacing(10)
        .padding([8, 10])
        .height(Length::Fill)
        .into();

        // overlay가 필요하면 stack으로 메인 위에 띄움 (backdrop + 가운데 모달 박스)
        let middle: Element<Message> = if self.show_command_palette {
            stack![main_view, modal_overlay(self.view_command_palette())].into()
        } else if self.show_settings {
            stack![main_view, modal_overlay(self.view_settings())].into()
        } else {
            // write_confirm은 입력창 위 인라인 패널(view_stream 안에서 처리)
            main_view
        };

        let statusbar = self.view_statusbar();

        let mut col = column![topbar, middle];
        if self.pty_visible {
            col = col.push(self.view_pty_panel());
        }
        col.push(statusbar)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_topbar(&self) -> Element<'_, Message> {
        let model_picker: Element<Message> = if self.model_ids.is_empty() {
            container(text("모델 없음").size(FS_BODY))
                .padding([6, 10])
                .style(panel_style)
                .into()
        } else {
            {
                let selected_opt = self
                    .selected_model
                    .as_ref()
                    .and_then(|id| self.model_options.iter().find(|o| &o.id == id));
                iced::widget::container(
                    combo_box(
                        &self.model_combo_state,
                        "모델 검색…",
                        selected_opt,
                        Message::SelectModel,
                    )
                    .size(FS_BODY),
                )
                .width(Length::FillPortion(2))
                .max_width(420.0)
                .into()
            }
        };

        let is_fav = self
            .selected_model
            .as_ref()
            .map(|id| self.favorites.contains(id))
            .unwrap_or(false);
        let fav_btn = button(text(if is_fav { "★" } else { "☆" }).size(16))
            .on_press(Message::ToggleFavorite)
            .padding([7, 12])
            .style(secondary_btn);

        let filters = row![
            checkbox(self.filter_coding)
                .label("코딩")
                .on_toggle(Message::ToggleFilterCoding)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.filter_reasoning)
                .label("추론")
                .on_toggle(Message::ToggleFilterReasoning)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.filter_general)
                .label("범용")
                .on_toggle(Message::ToggleFilterGeneral)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.filter_favorites_only)
                .label("⭐만")
                .on_toggle(Message::ToggleFilterFavorites)
                .size(16)
                .text_size(FS_BODY),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let sort_btn = button(text(self.sort_mode.label()).size(FS_BODY))
            .on_press(Message::CycleSortMode)
            .padding([7, 12])
            .style(secondary_btn);

        let bar = row![
            filters,
            Space::new().width(Length::Fill),
            sort_btn,
            model_picker,
            fav_btn,
            button(text("⚙").size(16).align_y(Alignment::Center))
                .on_press(Message::OpenSettings)
                .padding([7, 12])
                .style(secondary_btn),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        container(bar)
            .padding([10, 16])
            .style(topbar_style)
            .width(Length::Fill)
            .into()
    }

    fn view_usage_summary(&self) -> Element<'_, Message> {
        const MODEL_ID_PREVIEW_MAX: usize = 24;
        if self.usage.by_model.is_empty() {
            return text("(사용 기록 없음)").size(FS_LABEL).into();
        }
        // 비용 큰 순 5개
        let mut entries: Vec<(&String, &session::ModelUsage)> =
            self.usage.by_model.iter().collect();
        entries.sort_by(|a, b| {
            b.1.total_cost
                .partial_cmp(&a.1.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut col = column![].spacing(2);
        for (id, u) in entries.iter().take(5) {
            // model id가 너무 길면 끝부분만
            let short_id = shorten_tail(id, MODEL_ID_PREVIEW_MAX);
            col = col.push(
                row![
                    text(short_id).size(FS_LABEL),
                    Space::new().width(Length::Fill),
                    text(format!("${:.4}", u.total_cost))
                        .size(FS_LABEL)
                        .font(Font::with_name("JetBrains Mono")),
                ]
                .spacing(6),
            );
        }
        let total: f64 = self.usage.by_model.values().map(|u| u.total_cost).sum();
        col = col.push(Space::new().height(Length::Fixed(4.0)));
        col = col.push(
            row![
                text("총합").size(FS_LABEL).font(semibold_font()),
                Space::new().width(Length::Fill),
                text(format!("${:.4}", total))
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono")),
            ]
            .spacing(6),
        );
        col.into()
    }

    fn view_sidebar(&self) -> Element<'_, Message> {
        const CWD_PREVIEW_MAX: usize = 36;
        let cwd_display = self.cwd.display().to_string();
        // 너무 긴 경로는 끝부분만 표시
        let cwd_short = shorten_tail(&cwd_display, CWD_PREVIEW_MAX);

        // 세션 목록 (활성 + 비활성)
        let active_label = if self.current_session_title.trim().is_empty() {
            "새 채팅".to_string()
        } else {
            self.current_session_title.clone()
        };
        let mut sessions_col = column![container(
            text(format!("📌 {}", active_label))
                .size(FS_BODY)
                .font(semibold_font())
        )
        .padding([6, 8])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            container::Style {
                background: Some(
                    iced::Color::from_rgba(
                        p.primary.base.color.r,
                        p.primary.base.color.g,
                        p.primary.base.color.b,
                        0.16,
                    )
                    .into(),
                ),
                border: iced::Border {
                    color: p.primary.base.color,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        }),]
        .spacing(2);
        for s in &self.inactive_sessions {
            let title = if s.title.trim().is_empty() {
                "(빈 세션)".to_string()
            } else {
                s.title.clone()
            };
            let is_pending = self.pending_delete_session == Some(s.id);
            let trailing: Element<Message> = if is_pending {
                row![
                    button(text("✓").size(11))
                        .on_press(Message::DeleteSession(s.id))
                        .padding([2, 6])
                        .style(primary_btn),
                    button(text("✗").size(11))
                        .on_press(Message::CancelDeleteSession)
                        .padding([2, 6])
                        .style(secondary_btn),
                ]
                .spacing(2)
                .into()
            } else {
                button(text("✕").size(11))
                    .on_press(Message::AskDeleteSession(s.id))
                    .padding([2, 6])
                    .style(danger_btn)
                    .into()
            };
            let row_widget = row![
                button(text(format!("📂 {}", title)).size(FS_BODY))
                    .on_press(Message::SwitchSession(s.id))
                    .padding([4, 8])
                    .width(Length::Fill)
                    .style(secondary_btn),
                trailing,
            ]
            .spacing(2);
            sessions_col = sessions_col.push(row_widget);
        }

        let context_body = if self.attached_files.is_empty() {
            column![
                text("컨텍스트").size(FS_LABEL).font(semibold_font()),
                text("선택 안 됨").size(FS_SUBTITLE),
            ]
            .spacing(6)
        } else {
            let mut context_list = column![].spacing(4);
            for (i, (path, _)) in self.attached_files.iter().enumerate() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let short_name = shorten_tail(&name, 28);
                context_list = context_list.push(
                    container(
                        row![
                            text(format!("📄 {short_name}")).size(FS_BODY),
                            Space::new().width(Length::Fill),
                            button(text("✕").size(FS_MICRO))
                                .on_press(Message::RemoveAttachment(i))
                                .padding([1, 4])
                                .style(danger_btn),
                        ]
                        .spacing(4)
                        .align_y(Alignment::Center),
                    )
                    .padding([3, 6])
                    .style(|theme: &Theme| {
                        let p = theme.extended_palette();
                        container::Style {
                            background: Some(p.background.strong.color.into()),
                            border: iced::Border {
                                color: p.primary.weak.color,
                                width: 1.0,
                                radius: 10.0.into(),
                            },
                            ..Default::default()
                        }
                    }),
                );
            }
            let context_header = row![
                text(format!("컨텍스트 ({})", self.attached_files.len()))
                    .size(FS_LABEL)
                    .font(semibold_font()),
                Space::new().width(Length::Fill),
            ]
            .spacing(4)
            .align_y(Alignment::Center);
            column![
                context_header,
                scrollable(context_list)
                    .direction(Direction::Vertical(vscrollbar()))
                    .height(Length::Fixed(140.0)),
            ]
            .spacing(6)
        };

        let body = column![
            button(text("＋ 새 채팅").size(FS_SUBTITLE).font(semibold_font()))
                .on_press(Message::NewChat)
                .padding([6, 12])
                .width(Length::Fill)
                .style(primary_btn),
            Space::new().height(Length::Fixed(8.0)),
            text("채팅").size(FS_LABEL).font(semibold_font()),
            scrollable(sessions_col)
                .direction(Direction::Vertical(vscrollbar(),))
                .height(Length::Fixed(220.0)),
            Space::new().height(Length::Fixed(14.0)),
            text("모델 사용량 (누적)")
                .size(FS_LABEL)
                .font(semibold_font()),
            self.view_usage_summary(),
            Space::new().height(Length::Fixed(14.0)),
            text("작업 폴더").size(FS_LABEL).font(semibold_font()),
            text(cwd_short).size(FS_BODY),
            button(text("📁 폴더 변경").size(FS_LABEL))
                .on_press(Message::PickCwd)
                .padding([4, 8])
                .style(secondary_btn),
            Space::new().height(Length::Fixed(14.0)),
            text("프로젝트").size(FS_LABEL).font(semibold_font()),
            text("CodeWarp").size(FS_SUBTITLE).font(semibold_font()),
            Space::new().height(Length::Fixed(14.0)),
            context_body,
        ]
        .spacing(6);

        container(
            scrollable(body)
                .direction(Direction::Vertical(vscrollbar()))
                .height(Length::Fill),
        )
        .width(Length::Fixed(220.0))
        .height(Length::Fill)
        .padding(14)
        .style(panel_style)
        .into()
    }

    fn view_rightpanel(&self) -> Element<'_, Message> {
        // 세션 통계 — blocks/conversation에서 derive
        let user_msg_count = self
            .conversation
            .iter()
            .filter(|m| m.role == "user")
            .count();
        let tool_results: Vec<(&str, &str, bool)> = self
            .blocks
            .iter()
            .filter_map(|b| match &b.body {
                BlockBody::ToolResult {
                    name,
                    summary,
                    success,
                } => Some((name.as_str(), summary.as_str(), *success)),
                _ => None,
            })
            .collect();
        let tool_count = tool_results.len();
        let success_count = tool_results.iter().filter(|(_, _, s)| *s).count();
        let fail_count = tool_count - success_count;

        let stats = column![
            text("세션 통계").size(FS_LABEL).font(semibold_font()),
            text(format!("· 메시지: {}", user_msg_count)).size(FS_BODY),
            text(format!(
                "· 도구 호출: {} (✓{} ✗{})",
                tool_count, success_count, fail_count
            ))
            .size(FS_BODY),
            text(format!("· 모드: {}", self.agent_mode.label())).size(FS_BODY),
        ]
        .spacing(2);

        // 도구 호출 로그 (역순 — 최근이 위)
        let mut log_col =
            column![text("도구 호출 로그").size(FS_LABEL).font(semibold_font())].spacing(2);
        if tool_results.is_empty() {
            log_col = log_col.push(text("// 도구 호출 시 여기 누적").size(FS_LABEL));
        } else {
            for (name, summary, success) in tool_results.iter().rev() {
                let icon = if *success { "✓" } else { "✗" };
                let line = text(format!("{} {} → {}", icon, name, summary))
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono"));
                log_col = log_col.push(line);
            }
        }

        // 도구 라운드 진행 표시 (streaming 중일 때만)
        let round_indicator: Element<Message> =
            if self.streaming_block_id.is_some() && self.tool_round > 0 {
                text(format!(
                    "▶ 도구 라운드 {}/{}",
                    self.tool_round, MAX_TOOL_ROUNDS
                ))
                .size(FS_LABEL)
                .font(semibold_font())
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().primary.base.color),
                })
                .into()
            } else {
                Space::new()
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .into()
            };

        let body = column![
            stats,
            Space::new().height(Length::Fixed(14.0)),
            round_indicator,
            Space::new().height(Length::Fixed(6.0)),
            log_col,
        ]
        .spacing(6);

        container(
            scrollable(body)
                .direction(Direction::Vertical(vscrollbar()))
                .height(Length::Fill),
        )
        .width(Length::Fixed(280.0))
        .height(Length::Fill)
        .padding(14)
        .style(panel_style)
        .into()
    }

    /// 빈 채팅(blocks가 없을 때) 화면 — 예시 프롬프트 + 슬래시 명령 + 단축키.
    fn view_empty_chat(&self) -> Element<'_, Message> {
        const EXAMPLES: &[&str] = &[
            "이 프로젝트의 의존성을 알려줘",
            "src/main.rs의 첫 30줄을 요약해줘",
            "examples/hello.rs 만들어줘",
        ];
        let title = text("CodeWarp").size(FS_TITLE).font(bold_font());
        let subtitle =
            text("AI 코딩 데스크톱 — Plan으로 안전하게 둘러보고, Build로 변경 적용").size(FS_BODY);

        let mut examples_col = column![text("다음을 시도해보세요")
            .size(FS_LABEL)
            .font(semibold_font())]
        .spacing(6);
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

        let shortcuts = text("Ctrl+K 명령 팔레트 · Ctrl+N 새 채팅 · Ctrl+, 설정").size(FS_LABEL);

        container(
            column![
                title,
                subtitle,
                Space::new().height(Length::Fixed(20.0)),
                examples_col,
                Space::new().height(Length::Fixed(20.0)),
                modes,
                Space::new().height(Length::Fixed(14.0)),
                shortcuts,
            ]
            .spacing(6)
            .max_width(560),
        )
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(panel_style)
        .into()
    }

    fn view_stream(&self) -> Element<'_, Message> {
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
                        let id = b.id;
                        text_editor(content)
                            .on_action(move |action| Message::EditorAction(id, action))
                            .height(Length::Shrink)
                            .padding(0)
                            .size(FS_SUBTITLE)
                            .into()
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
                let block_view = container(column![header, body_view, apply_section].spacing(6))
                    .padding(12)
                    .width(Length::Fill)
                    .style(move |theme: &Theme| {
                        let p = theme.extended_palette();
                        let (bg, fg, border) = if is_user {
                            (
                                p.primary.weak.color,
                                p.background.base.text,
                                p.primary.strong.color,
                            )
                        } else {
                            (
                                p.background.weak.color,
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
            scrollable(col)
                .id(self.stream_id.clone())
                .on_scroll(Message::StreamScrolled)
                .direction(Direction::Vertical(vscrollbar()))
                .height(Length::Fill)
                .into()
        };

        let send_disabled = self.input.trim().is_empty() || self.selected_model.is_none();

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
                        .direction(Direction::Vertical(vscrollbar()))
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
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
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
            container(chips).padding([4, 0]).into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        let action_btn: Element<Message> = if self.streaming_block_id.is_some() {
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
            container(input_row)
                .padding([10, 14])
                .style(panel_style)
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
    }

    fn view_command_palette(&self) -> Element<'_, Message> {
        let header = text("명령 팔레트").size(18).font(bold_font());
        let hint = column![
            text("탐색  Esc 닫기 · Ctrl+K 토글").size(FS_LABEL),
            text("작업  Ctrl+N 새 채팅 · Ctrl+, 설정").size(FS_LABEL),
            text("모드  Ctrl+Shift+P 계획 · Ctrl+Shift+B 빌드").size(FS_LABEL),
        ]
        .spacing(2);
        let input = text_input("명령 검색…", &self.command_palette_input)
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
                list = list.push(
                    button(
                        column![
                            text(cmd.label).size(FS_SUBTITLE).font(semibold_font()),
                            text(cmd.hint).size(FS_LABEL),
                        ]
                        .spacing(2),
                    )
                    .on_press(Message::ExecuteCommand(i))
                    .padding([6, 10])
                    .width(Length::Fill)
                    .style(secondary_btn),
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
                .direction(Direction::Vertical(vscrollbar(),))
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
            .width(Length::Fixed(560.0))
            .style(panel_style)
            .into()
    }

    fn view_inline_confirm(&self) -> Element<'_, Message> {
        let n = self.pending_write_calls.len();
        let header = text(format!(
            "⚠ AI가 {}개 도구 실행을 요청했습니다 (카드 클릭으로 미리보기)",
            n
        ))
        .size(FS_BODY)
        .font(semibold_font());

        let mut cards = column![].spacing(4);
        for (idx, tc) in self.pending_write_calls.iter().enumerate() {
            let is_expanded = self.expanded_confirm_idx == Some(idx);
            let arrow = if is_expanded { "▾" } else { "▸" };

            let (summary_text, expanded_view): (String, Option<Element<Message>>) =
                match tc.name.as_str() {
                    "write_file" => match tools::WriteFileArgs::parse(&tc.arguments) {
                        Ok(args) => {
                            let abs_path = self.cwd.join(&args.path);
                            let exists = abs_path.exists();
                            let icon = if exists { "📝" } else { "✨" };
                            let summary = format!(
                                "{} {}  {}  ({} bytes)",
                                arrow,
                                icon,
                                args.path,
                                args.content.len()
                            );
                            let expanded = if is_expanded {
                                let old = std::fs::read_to_string(&abs_path).unwrap_or_default();
                                Some(render_diff(&old, &args.content))
                            } else {
                                None
                            };
                            (summary, expanded)
                        }
                        Err(e) => (format!("{} [err] {}", arrow, e), None),
                    },
                    "run_command" => match tools::RunCommandArgs::parse(&tc.arguments) {
                        Ok(args) => {
                            let summary = format!("{} 🖥  $ {}", arrow, args.command);
                            (summary, None) // run_command는 펼칠 내용 없음 (명령어만)
                        }
                        Err(e) => (format!("{} [err] {}", arrow, e), None),
                    },
                    _ => (format!("{} [?] {}", arrow, tc.name), None),
                };

            let summary_btn: Element<Message> = button(text(summary_text).size(FS_BODY).font(
                if tc.name == "run_command" {
                    Font::with_name("JetBrains Mono")
                } else {
                    Font::with_name("Pretendard")
                },
            ))
            .on_press(Message::ToggleConfirmExpand(idx))
            .padding([2, 6])
            .width(Length::Fill)
            .style(secondary_btn)
            .into();

            let discard_btn: Element<Message> = button(text("✗").size(FS_LABEL))
                .on_press(Message::DiscardWriteCall(idx))
                .padding([2, 6])
                .style(danger_btn)
                .into();

            let row_widget = row![summary_btn, discard_btn].spacing(4);
            let mut card_col = column![row_widget].spacing(2);
            if let Some(expanded) = expanded_view {
                card_col = card_col.push(container(expanded).padding([0, 18]));
            }
            cards = cards.push(card_col);
        }

        let actions = row![
            button(text("거부").size(FS_BODY))
                .on_press(Message::DenyWrites)
                .padding([4, 14])
                .style(danger_btn),
            Space::new().width(Length::Fill),
            button(text("✓ 모두 승인").size(FS_BODY))
                .on_press(Message::ApproveWrites)
                .padding([4, 14])
                .style(primary_btn),
        ]
        .spacing(8)
        .align_y(Alignment::Center);

        container(
            column![
                header,
                Space::new().height(Length::Fixed(4.0)),
                container(scrollable(cards).direction(Direction::Vertical(vscrollbar(),)))
                    .max_height(140.0),
                Space::new().height(Length::Fixed(6.0)),
                actions,
            ]
            .spacing(2),
        )
        .padding(10)
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            container::Style {
                background: Some(p.background.weak.color.into()),
                border: iced::Border {
                    color: p.danger.weak.color,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }

    #[allow(dead_code)]
    fn view_write_confirm(&self) -> Element<'_, Message> {
        let mut col = column![
            text("파일 쓰기 승인 대기").size(22).font(bold_font()),
            text(format!(
                "AI가 {}개의 파일을 변경하려고 합니다. 내용을 검토한 뒤 승인 또는 거부하세요.",
                self.pending_write_calls.len()
            ))
            .size(FS_SUBTITLE),
            Space::new().height(Length::Fixed(14.0)),
        ]
        .spacing(6);

        for tc in &self.pending_write_calls {
            let card: Element<Message> = match tc.name.as_str() {
                "write_file" => match tools::WriteFileArgs::parse(&tc.arguments) {
                    Ok(args) => {
                        let abs_path = self.cwd.join(&args.path);
                        let old_content = std::fs::read_to_string(&abs_path).ok();
                        let header = match &old_content {
                            Some(_) => format!("📝 {} ({} bytes)", args.path, args.content.len()),
                            None => {
                                format!("✨ 새 파일: {} ({} bytes)", args.path, args.content.len())
                            }
                        };
                        let diff_view: Element<Message> = match old_content {
                            Some(old) => render_diff(&old, &args.content),
                            None => container(
                                text(args.content.clone())
                                    .size(FS_BODY)
                                    .font(Font::with_name("JetBrains Mono")),
                            )
                            .padding(10)
                            .width(Length::Fill)
                            .into(),
                        };
                        column![
                            text(header).size(15).font(semibold_font()),
                            Space::new().height(Length::Fixed(6.0)),
                            diff_view,
                        ]
                        .spacing(4)
                        .into()
                    }
                    Err(e) => column![
                        text(format!("[arguments 파싱 실패] {}", e)).size(FS_SUBTITLE),
                        text(tc.arguments.clone()).size(FS_LABEL),
                    ]
                    .spacing(4)
                    .into(),
                },
                "run_command" => match tools::RunCommandArgs::parse(&tc.arguments) {
                    Ok(args) => column![
                        text("🖥 셸 명령 실행").size(15).font(semibold_font()),
                        Space::new().height(Length::Fixed(6.0)),
                        container(
                            text(format!("$ {}", args.command))
                                .size(FS_SUBTITLE)
                                .font(Font::with_name("JetBrains Mono")),
                        )
                        .padding(10)
                        .width(Length::Fill),
                    ]
                    .spacing(4)
                    .into(),
                    Err(e) => column![
                        text(format!("[arguments 파싱 실패] {}", e)).size(FS_SUBTITLE),
                        text(tc.arguments.clone()).size(FS_LABEL),
                    ]
                    .spacing(4)
                    .into(),
                },
                other => column![
                    text(format!("[알 수 없는 도구] {}", other)).size(FS_SUBTITLE),
                    text(tc.arguments.clone()).size(FS_LABEL),
                ]
                .spacing(4)
                .into(),
            };
            col = col.push(
                container(card)
                    .padding(12)
                    .width(Length::Fill)
                    .style(panel_style),
            );
        }

        let actions = row![
            button(text("거부").size(FS_SUBTITLE))
                .on_press(Message::DenyWrites)
                .padding([6, 16])
                .style(danger_btn),
            button(text("✓ 모두 승인").size(FS_SUBTITLE))
                .on_press(Message::ApproveWrites)
                .padding([6, 16])
                .style(primary_btn),
        ]
        .spacing(8);

        col = col.push(Space::new().height(Length::Fixed(14.0)));
        col = col.push(actions);

        container(
            scrollable(col)
                .direction(Direction::Vertical(vscrollbar()))
                .height(Length::Fill),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(panel_style)
        .into()
    }

    /// OpenAICompat endpoint 활성화 표시. ● + 한 줄 라벨.
    /// 녹색=연결됨, 빨강=끊김/실패, 회색=미시도.
    fn endpoint_indicator(&self, size: f32) -> Element<'_, Message> {
        #[derive(Clone, Copy)]
        enum Kind {
            Ok,
            Err,
            Unknown,
        }
        let (kind, label): (Kind, String) = match &self.tabby_status {
            Some(Ok(s)) => (Kind::Ok, format!("연결됨 — {}", s)),
            Some(Err(e)) => (Kind::Err, format!("끊김 — {}", e)),
            None => (Kind::Unknown, "endpoint 미시도".into()),
        };
        let dot = text("●").size(size).style(move |theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::text::Style {
                color: Some(match kind {
                    Kind::Ok => p.success.base.color,
                    Kind::Err => p.danger.base.color,
                    Kind::Unknown => p.background.strong.color,
                }),
            }
        });
        row![dot, text(label).size(size)]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }

    fn view_settings(&self) -> Element<'_, Message> {
        #[derive(Clone, Copy)]
        enum TabHealth {
            Good,
            Warn,
            Bad,
        }

        let provider_health = match &self.tabby_status {
            Some(Err(_)) => TabHealth::Bad,
            _ if self.has_key || !self.tabby_url_input.trim().is_empty() => TabHealth::Good,
            _ => TabHealth::Warn,
        };
        let runtime_health = if self.inference_pid.is_some() {
            TabHealth::Good
        } else {
            TabHealth::Warn
        };
        let local_model_count =
            list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len();
        let model_health = if local_model_count > 0 {
            TabHealth::Good
        } else {
            TabHealth::Warn
        };
        let mcp_health = if self.mcp_servers.is_empty() || self.mcp_tools.is_empty() {
            TabHealth::Warn
        } else {
            TabHealth::Good
        };

        let tab_btn = |icon: &'static str,
                       label: &'static str,
                       note: String,
                       health: TabHealth,
                       tab: SettingsTab| {
            let dot = text("●").size(FS_MICRO).style(move |theme: &Theme| {
                let p = theme.extended_palette();
                let color = match health {
                    TabHealth::Good => p.success.base.color,
                    TabHealth::Warn => p.primary.base.color,
                    TabHealth::Bad => p.danger.base.color,
                };
                iced::widget::text::Style { color: Some(color) }
            });
            let btn = button(
                column![
                    row![
                        text(icon).size(FS_LABEL),
                        text(label).size(FS_LABEL).font(semibold_font()),
                        dot,
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                    text(note).size(FS_MICRO),
                ]
                .spacing(2),
            )
            .on_press(Message::SetSettingsTab(tab))
            .padding([8, 8])
            .width(Length::FillPortion(1));
            if self.settings_tab == tab {
                btn.style(primary_btn)
            } else {
                btn.style(secondary_btn)
            }
        };

        let header = row![
            text("Settings").size(18).font(bold_font()),
            Space::new().width(Length::Fill),
            button(text("닫기").size(FS_BODY))
                .on_press(Message::CloseSettings)
                .padding([4, 12])
                .style(secondary_btn),
        ]
        .align_y(Alignment::Center);

        let key_status = if self.has_key {
            text("OpenRouter 키: 저장됨").size(FS_SUBTITLE)
        } else {
            text("OpenRouter 키 미등록").size(FS_SUBTITLE)
        };

        let key_input = text_input("sk-or-v1-...", &self.key_input)
            .on_input(Message::KeyInputChanged)
            .on_submit(Message::SaveKey)
            .padding(10)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fill);

        let actions = row![
            button(text("저장").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy || self.key_input.trim().is_empty() {
                    None
                } else {
                    Some(Message::SaveKey)
                })
                .style(primary_btn),
            button(text("삭제").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy || !self.has_key {
                    None
                } else {
                    Some(Message::ClearKey)
                })
                .style(danger_btn),
        ]
        .spacing(8);

        let tabby_header = text("OpenAI 호환 endpoint").size(14).font(semibold_font());
        let label_input: Element<Message> = text_input(
            "라벨 — 모델 셀렉터에 [xLLM] / [Tabby] / [Local] 같이 표시",
            &self.openai_compat_label,
        )
        .on_input(Message::OpenAICompatLabelChanged)
        .padding(8)
        .size(FS_BODY)
        .style(field_input)
        .width(Length::Fill)
        .into();
        let tabby_url = text_input(
            "예: http://localhost:9000 (xLLM) 또는 http://localhost:8080 (Tabby)",
            &self.tabby_url_input,
        )
        .on_input(Message::TabbyUrlChanged)
        .padding(10)
        .size(FS_BODY)
        .style(field_input)
        .width(Length::Fill);
        let tabby_token_toggle: Element<Message> = button(
            text(if self.show_tabby_token {
                "토큰 숨기기"
            } else {
                "토큰 입력 (선택)"
            })
            .size(FS_LABEL),
        )
        .on_press(Message::ToggleTabbyTokenVisible)
        .padding([4, 10])
        .style(secondary_btn)
        .into();
        let tabby_token: Element<Message> = if self.show_tabby_token {
            text_input("token (인증 강제 시에만)", &self.tabby_token_input)
                .on_input(Message::TabbyTokenChanged)
                .padding(10)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fill)
                .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };
        let tabby_actions = row![
            button(text("저장").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy {
                    None
                } else {
                    Some(Message::SaveTabby)
                })
                .style(primary_btn),
            button(text("연결 테스트").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy || self.tabby_url_input.trim().is_empty() {
                    None
                } else {
                    Some(Message::FetchTabbyModels)
                })
                .style(secondary_btn),
            button(text("삭제").size(FS_SUBTITLE))
                .on_press_maybe(
                    if self.busy
                        || (self.tabby_url_input.is_empty() && self.tabby_token_input.is_empty())
                    {
                        None
                    } else {
                        Some(Message::ClearTabby)
                    }
                )
                .style(danger_btn),
        ]
        .spacing(8);
        let tabby_status_label: Element<Message> = self.endpoint_indicator(FS_LABEL);

        let provider_section = column![
            container(
                column![
                    text("OpenRouter (클라우드)").size(14).font(semibold_font()),
                    key_status,
                    key_input,
                    actions,
                    text("1. https://openrouter.ai 가입").size(FS_LABEL),
                    text("2. /keys 에서 키 발급 후 붙여넣기").size(FS_LABEL),
                ]
                .spacing(8),
            )
            .padding([12, 14])
            .style(panel_style),
            container(
                column![
                    tabby_header,
                    label_input,
                    tabby_url,
                    tabby_token_toggle,
                    tabby_token,
                    tabby_actions,
                    tabby_status_label,
                ]
                .spacing(8),
            )
            .padding([12, 14])
            .style(panel_style),
        ]
        .spacing(10);

        let active_section: Element<Message> = match self.settings_tab {
            SettingsTab::Provider => container(provider_section)
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match provider_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            SettingsTab::Runtime => container(self.view_inference_runner())
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match runtime_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            SettingsTab::Models => container(self.view_model_manager())
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match model_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            SettingsTab::Mcp => container(self.view_mcp_settings())
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match mcp_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
        };

        let tabs = row![
            tab_btn(
                "◎",
                "Provider",
                if self.has_key || !self.tabby_url_input.trim().is_empty() {
                    "configured".to_string()
                } else {
                    "not set".to_string()
                },
                provider_health,
                SettingsTab::Provider
            ),
            tab_btn(
                "▶",
                "Runtime",
                if self.inference_pid.is_some() {
                    "running".to_string()
                } else {
                    "stopped".to_string()
                },
                runtime_health,
                SettingsTab::Runtime
            ),
            tab_btn(
                "□",
                "Models",
                format!("{local_model_count} local"),
                model_health,
                SettingsTab::Models
            ),
            tab_btn(
                "◇",
                "MCP",
                format!(
                    "{} srv / {} tools",
                    self.mcp_servers.len(),
                    self.mcp_tools.len()
                ),
                mcp_health,
                SettingsTab::Mcp
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let summary = row![text(format!(
            "Providers: {}  •  Runtime: {}  •  Models: {}  •  MCP: {}",
            if self.has_key || !self.tabby_url_input.trim().is_empty() {
                "configured"
            } else {
                "empty"
            },
            if self.inference_pid.is_some() {
                "running"
            } else {
                "stopped"
            },
            list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len(),
            self.mcp_servers.len()
        ))
        .size(FS_LABEL),];

        let runtime_can_start = match self.inference_engine {
            InferenceEngine::Custom => !self.inference_command_input.trim().is_empty(),
            InferenceEngine::Ollama => false,
            _ => !self.inference_selected_model.trim().is_empty(),
        };

        let (active_tab_title, active_health, active_action, quick_label, quick_action) = match self
            .settings_tab
        {
            SettingsTab::Provider => (
                "Provider",
                provider_health,
                if self.has_key || !self.tabby_url_input.trim().is_empty() {
                    "권장 액션: 현재 설정 유지 후 연결 테스트를 가끔 실행해 주세요.".to_string()
                } else {
                    "권장 액션: OpenRouter 키 저장 또는 로컬 endpoint URL을 먼저 등록해 주세요."
                        .to_string()
                },
                if !self.tabby_url_input.trim().is_empty() {
                    "연결 테스트"
                } else {
                    "키 저장"
                },
                if !self.tabby_url_input.trim().is_empty() {
                    Some(Message::FetchTabbyModels)
                } else if !self.key_input.trim().is_empty() {
                    Some(Message::SaveKey)
                } else {
                    None
                },
            ),
            SettingsTab::Runtime => (
                "Runtime",
                runtime_health,
                if self.inference_pid.is_some() {
                    "권장 액션: 현재 로그를 확인하고 필요한 경우 중지 후 모델을 교체하세요."
                        .to_string()
                } else {
                    "권장 액션: 엔진/모델(또는 커스텀 명령) 입력 후 시작 버튼을 눌러주세요."
                        .to_string()
                },
                if self.inference_pid.is_some() {
                    "중지"
                } else {
                    "시작"
                },
                if self.inference_pid.is_some() {
                    Some(Message::StopInference)
                } else if runtime_can_start {
                    Some(Message::StartInference)
                } else {
                    None
                },
            ),
            SettingsTab::Models => (
                "Models",
                model_health,
                if local_model_count > 0 {
                    "권장 액션: 다운로드된 모델을 Runtime 탭에서 선택해 실행해 보세요.".to_string()
                } else {
                    "권장 액션: 추천 프리셋에서 1개를 선택해 먼저 다운로드해 주세요.".to_string()
                },
                if local_model_count > 0 {
                    "Runtime 탭으로"
                } else {
                    "기본 EXL2 다운로드"
                },
                if local_model_count > 0 {
                    Some(Message::SetSettingsTab(SettingsTab::Runtime))
                } else if self.hf_dl.is_none() {
                    Some(Message::DownloadExl2Preset(0))
                } else {
                    None
                },
            ),
            SettingsTab::Mcp => (
                "MCP",
                mcp_health,
                if self.mcp_servers.is_empty() {
                    "권장 액션: 서버 이름과 명령을 입력해 MCP 서버를 하나 추가해 주세요."
                        .to_string()
                } else if self.mcp_tools.is_empty() {
                    "권장 액션: 서버 명령이 유효한지 확인하고 tools 로드를 기다려 주세요."
                        .to_string()
                } else {
                    "권장 액션: 채팅에서 MCP 도구 호출이 정상 동작하는지 점검해 주세요.".to_string()
                },
                "서버 추가",
                if !self.mcp_name_input.trim().is_empty()
                    && !self.mcp_command_input.trim().is_empty()
                {
                    Some(Message::AddMcpServer)
                } else {
                    None
                },
            ),
        };
        let badge_label = match active_health {
            TabHealth::Good => "정상",
            TabHealth::Warn => "설정 필요",
            TabHealth::Bad => "오류",
        };
        let status_badge = container(
            row![
                text("●").size(FS_MICRO).style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let color = match active_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    iced::widget::text::Style { color: Some(color) }
                }),
                text(badge_label).size(FS_MICRO).font(semibold_font()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([3, 8])
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match active_health {
                TabHealth::Good => p.success.base.color,
                TabHealth::Warn => p.primary.base.color,
                TabHealth::Bad => p.danger.base.color,
            };
            container::Style {
                background: Some(iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.12).into()),
                border: iced::Border {
                    color: accent,
                    width: 1.0,
                    radius: 999.0.into(),
                },
                ..Default::default()
            }
        });
        let active_header = row![
            text(format!("{active_tab_title} 상세"))
                .size(FS_SUBTITLE)
                .font(semibold_font()),
            Space::new().width(Length::Fill),
            status_badge,
        ]
        .align_y(Alignment::Center);
        let quick_btn: Element<Message> = button(text(quick_label).size(FS_BODY))
            .on_press_maybe(quick_action)
            .padding([6, 12])
            .style(primary_btn)
            .into();
        let active_action_hint = container(
            row![
                text(active_action).size(FS_LABEL).width(Length::Fill),
                quick_btn,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match active_health {
                TabHealth::Good => p.success.base.color,
                TabHealth::Warn => p.primary.base.color,
                TabHealth::Bad => p.danger.base.color,
            };
            container::Style {
                background: Some(iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.08).into()),
                border: iced::Border {
                    color: accent,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        });

        // 스크롤바가 콘텐츠를 덮지 않도록 우측 gutter를 확보한다.
        let scroll_body = container(
            column![
                Space::new().height(Length::Fixed(8.0)),
                tabs,
                summary,
                Space::new().height(Length::Fixed(8.0)),
                active_header,
                active_action_hint,
                active_section,
            ]
            .spacing(10)
            .max_width(560),
        )
        .padding([0, 14])
        .width(Length::Fill);

        let body = column![
            header,
            scrollable(scroll_body)
                .direction(Direction::Vertical(vscrollbar()))
                .height(Length::Fill)
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fixed(620.0))
        .spacing(8);

        container(body)
            .padding(20)
            .width(Length::Shrink)
            .height(Length::Fill)
            .into()
    }

    /// inference 서버 (xLLM/vLLM/llama-server/Tabby/Ollama/Custom) — dropdown 기반.
    /// CodeWarp가 child process로 spawn 관리.
    fn view_inference_runner(&self) -> Element<'_, Message> {
        let header = text("inference 서버 (CodeWarp가 spawn 관리)")
            .size(FS_SUBTITLE)
            .font(semibold_font());

        let engine_pick: Element<Message> = pick_list(
            InferenceEngine::ALL,
            Some(self.inference_engine),
            Message::SelectInferenceEngine,
        )
        .placeholder("엔진 선택")
        .text_size(FS_BODY)
        .into();

        // 바이너리 경로 — 비어있으면 PATH default, 채워져 있으면 절대 경로 사용
        let binary_section: Element<Message> = if matches!(
            self.inference_engine,
            InferenceEngine::Ollama | InferenceEngine::Custom
        ) {
            // Ollama는 spawn 안 함, Custom은 명령에 포함 → 별도 binary 입력 불필요
            Space::new().height(Length::Shrink).into()
        } else {
            row![
                text("바이너리").size(FS_LABEL).font(semibold_font()),
                text_input(
                    "PATH의 기본값 사용 (비워두면 됨) 또는 절대 경로",
                    &self.inference_binary_path,
                )
                .on_input(Message::InferenceBinaryChanged)
                .padding(6)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fixed(300.0)),
                button(text("📁").size(FS_LABEL))
                    .on_press(Message::PickInferenceBinary)
                    .padding([4, 8])
                    .style(secondary_btn),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        };

        // 엔진별 모델 입력 분기
        let model_section: Element<Message> = match self.inference_engine {
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer => {
                let dl_dir = std::path::PathBuf::from(&self.model_dir_input);
                let models = list_downloaded_models(&dl_dir);
                if models.is_empty() {
                    container(text("받은 모델 없음 — Models 탭에서 먼저 다운로드").size(FS_LABEL))
                        .padding([8, 10])
                        .style(panel_style)
                        .into()
                } else {
                    let selected = if self.inference_selected_model.is_empty() {
                        None
                    } else {
                        Some(self.inference_selected_model.clone())
                    };
                    pick_list(models, selected, Message::SelectInferenceModel)
                        .placeholder("받은 모델 선택")
                        .text_size(FS_BODY)
                        .into()
                }
            }
            InferenceEngine::Tabby => text_input(
                "Tabby 카탈로그 (예: TabbyML/Qwen2.5-Coder-7B — Tabby가 자체 다운로드)",
                &self.inference_selected_model,
            )
            .on_input(Message::SelectInferenceModel)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
            InferenceEngine::Ollama => text_input(
                "Ollama 모델 (예: qwen2.5-coder:7b) — daemon은 별도로 떠있어야",
                &self.inference_selected_model,
            )
            .on_input(Message::SelectInferenceModel)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
            InferenceEngine::Custom => text_input(
                "직접 명령 (예: xllm serve --model ... --port 9000)",
                &self.inference_command_input,
            )
            .on_input(Message::InferenceCommandChanged)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
        };

        // 포트 (Ollama는 항상 11434, Custom은 명령에 포함되므로 hide)
        let port_section: Element<Message> = match self.inference_engine {
            InferenceEngine::Custom => Space::new().height(Length::Shrink).into(),
            _ => row![
                text("포트").size(FS_LABEL).font(semibold_font()),
                text_input("9000", &self.inference_port_input)
                    .on_input(Message::InferencePortChanged)
                    .padding(6)
                    .size(FS_BODY)
                    .style(field_input)
                    .width(Length::Fixed(100.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        };

        let running = self.inference_pid.is_some();
        let can_start = match self.inference_engine {
            InferenceEngine::Custom => !self.inference_command_input.trim().is_empty(),
            InferenceEngine::Ollama => false, // Ollama는 spawn 안 함 (daemon)
            _ => !self.inference_selected_model.trim().is_empty(),
        };

        let actions: Element<Message> = if running {
            let running_label = if let Some(pid) = self.inference_pid {
                format!("● 실행 중 (pid {})", pid)
            } else {
                "● 실행 중".to_string()
            };
            row![
                text(running_label).size(FS_LABEL).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.extended_palette().success.base.color),
                    }
                }),
                Space::new().width(Length::Fill),
                button(text("중지").size(FS_LABEL))
                    .on_press(Message::StopInference)
                    .padding([4, 12])
                    .style(danger_btn),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            let btn_label = if self.inference_engine == InferenceEngine::Ollama {
                "Ollama는 daemon — 시작 불필요"
            } else {
                "시작"
            };
            row![
                text("● 미실행")
                    .size(FS_LABEL)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().background.strong.color),
                    }),
                Space::new().width(Length::Fill),
                button(text(btn_label).size(FS_LABEL))
                    .on_press_maybe(if can_start {
                        Some(Message::StartInference)
                    } else {
                        None
                    })
                    .padding([4, 12])
                    .style(primary_btn),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        };

        // 로그 마지막 N줄 (있을 때만)
        let log_section: Element<Message> = if self.inference_log.is_empty() {
            Space::new().height(Length::Shrink).into()
        } else {
            let mut col =
                column![text("로그 (최근)").size(FS_MICRO).font(semibold_font())].spacing(1);
            for line in &self.inference_log {
                col = col.push(
                    text(line.clone())
                        .size(FS_MICRO)
                        .font(Font::with_name("JetBrains Mono")),
                );
            }
            container(col).padding([6, 10]).style(panel_style).into()
        };

        container(
            column![
                header,
                row![
                    text("엔진").size(FS_LABEL).font(semibold_font()),
                    engine_pick
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                binary_section,
                model_section,
                port_section,
                actions,
                log_section
            ]
            .spacing(8),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }

    fn view_mcp_settings(&self) -> Element<'_, Message> {
        let header = text("MCP 서버 (Model Context Protocol)")
            .size(14)
            .font(semibold_font());
        let hint = text("stdio MCP 서버를 등록해 AI tool을 동적으로 확장합니다.").size(FS_LABEL);

        // 등록된 서버 목록
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

        // 추가 입력 행
        let add_row = row![
            text_input("서버 이름 (예: filesystem)", &self.mcp_name_input)
                .on_input(Message::McpNameChanged)
                .padding(6)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fixed(140.0)),
            text_input(
                "명령 (예: npx -y @modelcontextprotocol/server-filesystem /tmp)",
                &self.mcp_command_input
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

    fn view_model_manager(&self) -> Element<'_, Message> {
        let header = text("모델 매니저 (HuggingFace 다운로드)")
            .size(14)
            .font(semibold_font());
        let local_model_count =
            list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len();
        let local_state: Element<Message> = if local_model_count == 0 {
            container(
                text("로컬 모델이 비어 있습니다. 프리셋 1개를 먼저 내려받으세요.").size(FS_LABEL),
            )
            .padding([8, 10])
            .style(panel_style)
            .into()
        } else {
            container(
                text(format!(
                    "로컬 모델 {}개가 준비되어 있습니다.",
                    local_model_count
                ))
                .size(FS_LABEL),
            )
            .padding([8, 10])
            .style(panel_style)
            .into()
        };

        // 다운로드 경로 — picker 버튼을 명확하게
        let dir_input = text_input("예: C:\\models 또는 ~/models", &self.model_dir_input)
            .on_input(Message::ModelDirChanged)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(360.0));
        let dir_row = row![
            dir_input,
            button(text("📁 찾아보기").size(FS_LABEL))
                .on_press(Message::PickModelDir)
                .padding([6, 12])
                .style(secondary_btn),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        // HF 토큰 toggle
        let token_toggle = button(
            text(if self.show_hf_token {
                "토큰 숨기기"
            } else {
                "토큰 입력"
            })
            .size(FS_LABEL),
        )
        .on_press(Message::ToggleHfTokenVisible)
        .padding([4, 10])
        .style(secondary_btn);
        let token_section: Element<Message> = if self.show_hf_token {
            row![
                text_input("hf_xxx... (gated repo용, 선택)", &self.hf_token_input)
                    .on_input(Message::HfTokenChanged)
                    .on_submit(Message::SaveHfToken)
                    .padding(8)
                    .size(FS_BODY)
                    .style(field_input)
                    .width(Length::Fixed(360.0)),
                button(text("저장").size(FS_LABEL))
                    .on_press(Message::SaveHfToken)
                    .padding([4, 10])
                    .style(primary_btn),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        // 추천 프리셋 — 카드를 두드러지게 (가장 많이 쓰이는 진입점)
        let mut presets_col =
            column![text("추천 프리셋 (클릭 → 입력란에 채움)").size(12)].spacing(4);
        for (i, p) in MODEL_PRESETS.iter().enumerate() {
            presets_col = presets_col.push(
                button(
                    column![
                        text(p.label).size(FS_SUBTITLE).font(semibold_font()),
                        text(p.note).size(FS_MICRO),
                        text(p.repo_id)
                            .size(FS_MICRO)
                            .font(Font::with_name("JetBrains Mono")),
                    ]
                    .spacing(2),
                )
                .on_press(Message::UsePreset(i))
                .padding([6, 12])
                .width(Length::Fill)
                .style(secondary_btn),
            );
        }

        // repo 입력 + 다운로드 시작
        let repo_input = text_input(
            "HF repo (예: Qwen/Qwen2.5-Coder-7B-Instruct)",
            &self.hf_repo_input,
        )
        .on_input(Message::HfRepoChanged)
        .on_submit(Message::StartHfDownload)
        .padding(8)
        .size(FS_BODY)
        .style(field_input)
        .width(Length::Fixed(360.0));
        let action_btn: Element<Message> = if self.hf_dl.is_some() {
            button(text("취소").size(FS_LABEL))
                .on_press(Message::CancelHfDownload)
                .padding([4, 10])
                .style(danger_btn)
                .into()
        } else {
            button(text("다운로드").size(FS_LABEL))
                .on_press(Message::StartHfDownload)
                .padding([4, 10])
                .style(primary_btn)
                .into()
        };
        let dl_row = row![repo_input, action_btn]
            .spacing(6)
            .align_y(Alignment::Center);

        // 진행률
        let progress: Element<Message> = if let Some(dl) = &self.hf_dl {
            let pct_text = match dl.file_bytes_total {
                Some(t) if t > 0 => {
                    format!("{:.0}%", (dl.file_bytes_done as f64 / t as f64) * 100.0)
                }
                _ => fmt_bytes(dl.file_bytes_done).to_string(),
            };
            column![
                text(format!(
                    "[{}/{}] {}",
                    dl.file_idx + 1,
                    dl.total_files.max(1),
                    dl.file_name
                ))
                .size(FS_LABEL)
                .font(Font::with_name("JetBrains Mono")),
                text(pct_text).size(FS_LABEL),
            ]
            .spacing(2)
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        // EXL2 프리셋 섹션 (TabbyAPI용, 클릭 → 즉시 다운로드)
        let is_downloading = self.hf_dl.is_some();
        let mut exl2_col =
            column![text("EXL2 프리셋 (TabbyAPI용) — 클릭하면 바로 다운로드").size(FS_BODY),]
                .spacing(4);
        for (i, p) in EXL2_PRESETS.iter().enumerate() {
            let btn = button(
                row![
                    column![
                        text(p.label).size(FS_SUBTITLE).font(semibold_font()),
                        text(p.note).size(FS_MICRO),
                        text(p.repo_id)
                            .size(FS_MICRO)
                            .font(Font::with_name("JetBrains Mono")),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    text(p.vram).size(FS_LABEL),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([6, 12]);
            exl2_col = exl2_col.push(if is_downloading {
                btn.style(secondary_btn)
            } else {
                btn.on_press(Message::DownloadExl2Preset(i))
                    .style(secondary_btn)
            });
        }

        container(
            column![
                header,
                local_state,
                text("HuggingFace에서 모델 받아 디스크에 저장.").size(FS_LABEL),
                Space::new().height(Length::Fixed(4.0)),
                text("저장 경로 (변경 가능)")
                    .size(FS_LABEL)
                    .font(semibold_font()),
                dir_row,
                Space::new().height(Length::Fixed(12.0)),
                exl2_col,
                Space::new().height(Length::Fixed(12.0)),
                text("HF 일반 모델 (safetensors · xLLM/vLLM용) — 클릭 → 입력란에 채움")
                    .size(FS_BODY),
                presets_col,
                Space::new().height(Length::Fixed(8.0)),
                text("또는 직접 입력").size(FS_LABEL).font(semibold_font()),
                dl_row,
                progress,
                Space::new().height(Length::Fixed(12.0)),
                // gated repo (Llama 등) 받을 때만 필요
                token_toggle,
                token_section,
            ]
            .spacing(6),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }

    fn view_pty_panel(&self) -> Element<'_, Message> {
        // 헤더 행: 제목 + 버튼들
        let header = row![
            text("터미널").size(FS_SUBTITLE).font(semibold_font()),
            Space::new().width(Length::Fill),
            button(text("✕ Clear").size(FS_LABEL))
                .on_press(Message::PtyClear)
                .padding([2, 8])
                .style(secondary_btn),
            button(text("✕").size(FS_LABEL))
                .on_press(Message::PtyToggle)
                .padding([2, 8])
                .style(secondary_btn),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .padding([4, 8]);

        // 출력 영역 (최근 줄이 아래)
        let mut out_col = column![].spacing(0);
        for line in &self.pty_output {
            out_col = out_col.push(
                text(line)
                    .size(FS_BODY)
                    .font(Font::with_name("JetBrains Mono")),
            );
        }
        let output_area = scrollable(out_col)
            .direction(Direction::Vertical(vscrollbar()))
            .height(Length::Fixed(200.0))
            .width(Length::Fill);

        // 입력 행
        let session_active = self.pty_session.is_some();
        let input_row = row![
            text_input(
                if session_active {
                    "> 명령 입력…"
                } else {
                    "터미널 종료됨 (Ctrl+` 로 재시작)"
                },
                &self.pty_input
            )
            .on_input(Message::PtyInputChanged)
            .on_submit(Message::PtySend)
            .padding(6)
            .size(FS_LABEL)
            .style(field_input)
            .font(Font::with_name("JetBrains Mono"))
            .width(Length::Fill),
            button(text("전송").size(FS_LABEL))
                .on_press_maybe(if session_active {
                    Some(Message::PtySend)
                } else {
                    None
                })
                .padding([6, 10])
                .style(primary_btn),
            button(text("^C").size(FS_LABEL))
                .on_press_maybe(if session_active {
                    Some(Message::PtyCtrlC)
                } else {
                    None
                })
                .padding([6, 8])
                .style(danger_btn),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        container(column![header, output_area, container(input_row).padding([4, 8])].spacing(0))
            .width(Length::Fill)
            .style(panel_style)
            .into()
    }

    fn view_statusbar(&self) -> Element<'_, Message> {
        let model_label = self
            .selected_model
            .clone()
            .unwrap_or_else(|| "(없음)".into());
        let credit_label = match &self.account {
            Some(a) => match (a.usage, a.limit) {
                (Some(u), Some(l)) => format!("잔액: ${:.2} / ${:.2}", (l - u).max(0.0), l),
                (Some(u), None) => format!("사용: ${:.4}", u),
                _ => "잔액: -".into(),
            },
            None => "잔액: -".into(),
        };
        let last_cost_label = match self.last_response_cost {
            Some(c) if c > 0.0 => format!("최근: ${:.4}", c),
            _ => String::new(),
        };
        let busy_prefix: Element<Message> = if self.streaming_block_id.is_some() {
            text("▶ ")
                .size(FS_LABEL)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().primary.base.color),
                })
                .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        };
        let mut bar = row![
            busy_prefix,
            text(&self.status).size(FS_LABEL),
            Space::new().width(Length::Fill),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if !last_cost_label.is_empty() {
            bar = bar.push(
                text(last_cost_label)
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono")),
            );
        }
        bar = bar
            .push(text(credit_label).size(FS_LABEL))
            .push(text(format!("모델: {}", model_label)).size(FS_LABEL))
            .push(
                text(if self.has_key {
                    "키: 등록됨"
                } else {
                    "키: 미등록"
                })
                .size(FS_LABEL),
            )
            .push(self.endpoint_indicator(FS_LABEL));

        container(bar)
            .padding([4, 14])
            .style(topbar_style)
            .width(Length::Fill)
            .into()
    }
}
