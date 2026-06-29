use super::render_diff;
use super::ui::{
    FS_BODY, FS_LABEL, app_vscrollbar, danger_btn, dark_scrollable, primary_btn, secondary_btn,
    semibold_font,
};
use crate::{App, Message, tools};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Font, Length, Shadow, Theme, Vector};

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn view_inline_confirm(&self) -> Element<'_, Message> {
        let n = self.pending_write_calls.len();
        let header = text(format!(
            "⚠ AI가 {n}개 도구 실행을 요청했습니다 (카드 클릭으로 미리보기)"
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
                        Err(e) => (format!("{arrow} [err] {e}"), None),
                    },
                    "run_command" => match tools::RunCommandArgs::parse(&tc.arguments) {
                        Ok(args) => {
                            let summary = format!("{arrow} 🖥  $ {}", args.command);
                            (summary, None)
                        }
                        Err(e) => (format!("{arrow} [err] {e}"), None),
                    },
                    _ => (format!("{arrow} [?] {}", tc.name), None),
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
                container(
                    scrollable(cards)
                        .direction(Direction::Vertical(app_vscrollbar(),))
                        .style(dark_scrollable),
                )
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
                background: Some(
                    Color::from_rgba(
                        p.danger.weak.color.r,
                        p.danger.weak.color.g,
                        p.danger.weak.color.b,
                        0.10,
                    )
                    .into(),
                ),
                border: iced::Border {
                    color: p.danger.weak.color,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                    offset: Vector { x: 0.0, y: 2.0 },
                    blur_radius: 8.0,
                },
                ..Default::default()
            }
        })
        .into()
    }
}
