use super::render_diff;
use super::ui::*;
use crate::*;
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    pub(crate) fn view_inline_confirm(&self) -> Element<'_, Message> {
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
                            (summary, None)
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
                            Some(_) => {
                                format!("📝 {} ({} bytes)", args.path, args.content.len())
                            }
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
}
