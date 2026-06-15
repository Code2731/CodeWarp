// view.rs — App 뷰 메서드 (main.rs child module)
use super::*;
mod chat;
mod settings;
mod ui;

use iced::widget::scrollable::Direction;
use iced::widget::{
    button, checkbox, column, combo_box, container, row, scrollable, stack, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Theme};
pub(crate) use ui::*;

/// 두 텍스트의 line-by-line diff를 색상 표시된 Element로 변환.
/// 추가 라인은 녹색, 삭제 라인은 빨강, 동일 라인은 흐리게.
fn render_diff<'a>(old: &str, new: &str) -> Element<'a, Message> {
    use similar::{ChangeTag, TextDiff};

    const MAX_LINES: usize = 400;
    let added = Color::from_rgb(0.55, 0.85, 0.55);
    let removed = Color::from_rgb(0.95, 0.45, 0.45);
    let equal = Color::from_rgb(0.5, 0.5, 0.55);

    let diff = TextDiff::from_lines(old, new);
    let mut col = column![].spacing(0);
    for (count, change) in diff.iter_all_changes().enumerate() {
        if count >= MAX_LINES {
            col = col.push(
                text(format!("…(diff 라인 {}+ 생략)", MAX_LINES))
                    .size(11)
                    .color(equal),
            );
            break;
        }
        let (sign, color) = match change.tag() {
            ChangeTag::Delete => ("-", removed),
            ChangeTag::Insert => ("+", added),
            ChangeTag::Equal => (" ", equal),
        };
        let raw = change.value().trim_end_matches('\n');
        let line_text = if raw.len() > 200 {
            format!("{} {}…", sign, &raw[..200])
        } else {
            format!("{} {}", sign, raw)
        };
        col = col.push(
            text(line_text)
                .size(11)
                .font(Font::with_name("JetBrains Mono"))
                .color(color),
        );
    }
    container(col).padding(10).width(Length::Fill).into()
}

/// 모달 오버레이: 반투명 백드롭 + 가운데 정렬된 콘텐츠 박스.
/// content는 view_settings/view_write_confirm 같은 기존 화면 함수의 결과.
fn modal_overlay<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    let modal_box = container(content)
        .padding(0)
        .width(Length::Shrink)
        .max_width(720.0)
        .max_height(720.0)
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            container::Style {
                background: Some(palette.background.base.color.into()),
                border: iced::Border {
                    color: palette.background.strong.color,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        });

    container(modal_box)
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(20)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.55).into()),
            ..Default::default()
        })
        .into()
}

/// markdown::view_with용 커스텀 Viewer.
/// - heading: Bold weight 강제
/// - paragraph: italic span을 Normal+SemiBold로 변환 (한국어 글리프 깨짐 회피)
struct CodewarpViewer;

impl<'a> iced::widget::markdown::Viewer<'a, Message> for CodewarpViewer {
    fn on_link_click(url: iced::widget::markdown::Uri) -> Message {
        Message::LinkClicked(url)
    }

    fn heading(
        &self,
        mut settings: iced::widget::markdown::Settings,
        level: &'a iced::widget::markdown::HeadingLevel,
        text: &'a iced::widget::markdown::Text,
        index: usize,
    ) -> Element<'a, Message> {
        let mut bold = Font::with_name("Pretendard");
        bold.weight = iced::font::Weight::Bold;
        settings.style.font = bold;
        iced::widget::markdown::heading(settings, level, text, index, Self::on_link_click)
    }

    fn paragraph(
        &self,
        settings: iced::widget::markdown::Settings,
        text: &iced::widget::markdown::Text,
    ) -> Element<'a, Message> {
        let spans_arc = text.spans(settings.style);
        let normalized: Vec<iced::advanced::text::Span<'static, iced::widget::markdown::Uri>> =
            spans_arc
                .iter()
                .map(|s| {
                    let mut s = s.clone();
                    if let Some(font) = s.font.as_mut() {
                        if !matches!(font.style, iced::font::Style::Normal) {
                            font.style = iced::font::Style::Normal;
                            if matches!(font.weight, iced::font::Weight::Normal) {
                                font.weight = iced::font::Weight::Semibold;
                            }
                        }
                    }
                    s
                })
                .collect();
        iced::widget::rich_text(normalized)
            .on_link_click(Self::on_link_click)
            .into()
    }

    fn code_block(
        &self,
        _settings: iced::widget::markdown::Settings,
        language: Option<&'a str>,
        code: &'a str,
        _lines: &'a [iced::widget::markdown::Text],
    ) -> Element<'a, Message> {
        use iced::widget::scrollable::Direction;
        let language_label = language.unwrap_or("text").to_ascii_lowercase();

        let header = row![
            container(
                text(language_label)
                    .size(11)
                    .font(Font::with_name("JetBrains Mono"))
            )
            .padding([2, 8])
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgba8(0x30, 0x36, 0x3d, 0.95).into()),
                border: iced::Border {
                    color: Color::from_rgba8(0x58, 0x6e, 0x75, 0.65),
                    width: 1.0,
                    radius: 999.0.into(),
                },
                ..Default::default()
            }),
            Space::new().width(Length::Fill),
            button(
                text("Copy")
                    .size(11)
                    .font(Font::with_name("JetBrains Mono"))
            )
            .on_press(Message::CopyText(code.to_string()))
            .padding([3, 10]),
        ]
        .spacing(8);

        let code_text = container(
            text(code)
                .size(12)
                .line_height(1.35)
                .font(Font::with_name("JetBrains Mono")),
        )
        .padding([12, 14]);

        let code_body = scrollable(code_text)
            .direction(Direction::Horizontal(hscrollbar()))
            .width(Length::Fill);

        container(column![header, code_body].spacing(0))
            .padding(10)
            .width(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgb8(0x0d, 0x11, 0x17).into()),
                border: iced::Border {
                    color: Color::from_rgba8(0x30, 0x36, 0x3d, 0.95),
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
    }
}

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
        .spacing(MAIN_ROW_SPACING)
        .padding([MAIN_PAD_Y, MAIN_PAD_X])
        .height(Length::Fill)
        .into();

        // overlay가 필요하면 stack으로 메인 위에 띄움 (backdrop + 가운데 모달 박스)
        let middle: Element<Message> = if self.ui.show_command_palette {
            stack![main_view, modal_overlay(self.view_command_palette())].into()
        } else if self.ui.show_settings {
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
            .map(|id| self.model_filter.favorites.contains(id))
            .unwrap_or(false);
        let fav_btn = button(text(if is_fav { "★" } else { "☆" }).size(16))
            .on_press(Message::ToggleFavorite)
            .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
            .style(secondary_btn);

        let filters = row![
            checkbox(self.model_filter.filter_coding)
                .label("코딩")
                .on_toggle(Message::ToggleFilterCoding)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.model_filter.filter_reasoning)
                .label("추론")
                .on_toggle(Message::ToggleFilterReasoning)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.model_filter.filter_general)
                .label("범용")
                .on_toggle(Message::ToggleFilterGeneral)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.model_filter.filter_favorites_only)
                .label("⭐만")
                .on_toggle(Message::ToggleFilterFavorites)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.compare_both)
                .label("둘 다 답변")
                .on_toggle(Message::ToggleCompareBoth)
                .size(16)
                .text_size(FS_BODY),
        ]
        .spacing(TOPBAR_ROW_SPACING)
        .align_y(Alignment::Center);

        let sort_btn = button(text(self.model_filter.sort_mode.label()).size(FS_BODY))
            .on_press(Message::CycleSortMode)
            .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
            .style(secondary_btn);

        let bar = row![
            filters,
            Space::new().width(Length::Fill),
            sort_btn,
            model_picker,
            fav_btn,
            button(text("⚙").size(16).align_y(Alignment::Center))
                .on_press(Message::OpenSettings)
                .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
                .style(secondary_btn),
        ]
        .spacing(TOPBAR_ROW_SPACING)
        .align_y(Alignment::Center);

        container(bar)
            .padding([TOPBAR_PAD_Y, TOPBAR_PAD_X])
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
            let is_pending = self.ui.pending_delete_session == Some(s.id);
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

        let context_total_bytes: u64 = self
            .attached_files
            .iter()
            .map(|(_, content)| content.len() as u64)
            .sum();
        let context_quota_label = format!(
            "{}/{}",
            fmt_bytes(context_total_bytes),
            fmt_bytes(MAX_ATTACH_BYTES)
        );
        let context_actions = |attached_count: usize| {
            let has_files = attached_count > 0;
            let clear_label = if has_files {
                format!("Clear ({attached_count})")
            } else {
                "Clear".to_string()
            };
            row![
                button(text("+ Add file").size(FS_MICRO))
                    .on_press(Message::PickAttachment)
                    .padding([PAD_XXS, PAD_MD])
                    .style(secondary_btn),
                button(text(clear_label).size(FS_MICRO))
                    .on_press_maybe(if has_files {
                        Some(Message::ClearAttachments)
                    } else {
                        None
                    })
                    .padding([PAD_XXS, PAD_MD])
                    .style(danger_btn),
            ]
            .spacing(SPACE_XS)
            .align_y(Alignment::Center)
        };

        let context_body = if self.attached_files.is_empty() {
            let context_header = row![
                text("Context (0)").size(FS_LABEL).font(semibold_font()),
                Space::new().width(Length::Fill),
                text(context_quota_label.clone())
                    .size(FS_MICRO)
                    .font(Font::with_name("JetBrains Mono")),
            ]
            .spacing(SPACE_XS)
            .align_y(Alignment::Center);
            column![
                context_header,
                context_actions(0),
                text("No files selected").size(FS_SUBTITLE),
            ]
            .spacing(SPACE_SM)
        } else {
            let mut context_list = column![].spacing(SPACE_XS);
            for (i, (path, content)) in self.attached_files.iter().enumerate() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                let short_name = shorten_tail(&name, 24);
                let rel_path = path.strip_prefix(&self.cwd).unwrap_or(path.as_path());
                let short_path = shorten_tail(&rel_path.display().to_string(), 42);
                let size_label = fmt_bytes(content.len() as u64);
                context_list = context_list.push(
                    container(
                        row![
                            column![
                                text(short_name).size(FS_BODY).font(semibold_font()),
                                text(short_path).size(FS_MICRO),
                            ]
                            .spacing(SPACE_XXS),
                            Space::new().width(Length::Fill),
                            text(size_label)
                                .size(FS_MICRO)
                                .font(Font::with_name("JetBrains Mono")),
                            button(text("x").size(FS_MICRO))
                                .on_press(Message::RemoveAttachment(i))
                                .padding([PAD_XXS, PAD_XS])
                                .style(danger_btn),
                        ]
                        .spacing(SPACE_XS)
                        .align_y(Alignment::Center),
                    )
                    .padding([PAD_XXS, PAD_SM])
                    .style(context_item_style),
                );
            }
            let context_header = row![
                text(format!("Context ({})", self.attached_files.len()))
                    .size(FS_LABEL)
                    .font(semibold_font()),
                Space::new().width(Length::Fill),
                text(context_quota_label)
                    .size(FS_MICRO)
                    .font(Font::with_name("JetBrains Mono")),
            ]
            .spacing(SPACE_XS)
            .align_y(Alignment::Center);
            column![
                context_header,
                context_actions(self.attached_files.len()),
                scrollable(context_list)
                    .direction(Direction::Vertical(app_vscrollbar()))
                    .height(Length::Fixed(CONTEXT_LIST_HEIGHT)),
            ]
            .spacing(SPACE_SM)
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
                .direction(Direction::Vertical(app_vscrollbar(),))
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
        .spacing(SPACE_SM);

        let resize_row = row![
            text(format!("너비 {:.0}px", self.sidebar_width)).size(FS_LABEL),
            Space::new().width(Length::Fill),
            button(text("◀▶").size(FS_LABEL).font(semibold_font()))
                .on_press(Message::CycleSidebarWidth)
                .padding([PAD_XS, PAD_MD])
                .style(secondary_btn),
        ]
        .spacing(SPACE_XS)
        .align_y(Alignment::Center);

        container(
            column![
                scrollable(container(body).padding([0, SCROLL_GUTTER_PAD_X]))
                    .direction(Direction::Vertical(app_vscrollbar()))
                    .height(Length::Fill),
                resize_row,
            ]
            .spacing(SPACE_SM),
        )
        .width(Length::Fixed(self.sidebar_width))
        .height(Length::Fill)
        .padding(PAD_LG)
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
            Space::new().height(Length::Fixed(PANEL_SECTION_GAP_LG)),
            round_indicator,
            Space::new().height(Length::Fixed(SPACE_SM)),
            log_col,
        ]
        .spacing(SPACE_SM);

        container(
            scrollable(container(body).padding([0, SCROLL_GUTTER_PAD_X]))
                .direction(Direction::Vertical(app_vscrollbar()))
                .height(Length::Fill),
        )
        .width(Length::Fixed(RIGHT_PANEL_WIDTH))
        .height(Length::Fill)
        .padding(PAD_LG)
        .style(panel_style)
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
                .direction(Direction::Vertical(app_vscrollbar(),))
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
            let is_expanded = self.ui.expanded_confirm_idx == Some(idx);
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
                container(scrollable(cards).direction(Direction::Vertical(app_vscrollbar(),)))
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
                .direction(Direction::Vertical(app_vscrollbar()))
                .height(Length::Fill),
        )
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill)
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
        let output_area = scrollable(container(out_col).padding([0, SCROLL_GUTTER_PAD_X]))
            .direction(Direction::Vertical(app_vscrollbar()))
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
        let streaming_indicator: Element<Message> = if self.streaming_block_id.is_some() {
            text("▶ 응답 생성 중...")
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
        let status_text: Element<Message> = if self.status.starts_with("[WARN]") {
            text(&self.status)
                .size(FS_LABEL)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().warning.base.color),
                })
                .into()
        } else if self.status.starts_with("[ERROR]") {
            text(&self.status)
                .size(FS_LABEL)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                })
                .into()
        } else {
            text(&self.status).size(FS_LABEL).into()
        };
        let mut bar = row![
            streaming_indicator,
            status_text,
            Space::new().width(Length::Fill),
        ]
        .spacing(STATUSBAR_ROW_SPACING)
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
            .padding([STATUSBAR_PAD_Y, STATUSBAR_PAD_X])
            .style(topbar_style)
            .width(Length::Fill)
            .into()
    }
}
