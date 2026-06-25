use super::ui::{
    FS_BODY, FS_LABEL, FS_SUBTITLE, PAD_LG, PAD_MD, PAD_XS, SCROLL_GUTTER_PAD_X, SPACE_SM,
    SPACE_XS, app_vscrollbar, danger_btn, panel_style, primary_btn, secondary_btn, semibold_font,
    shorten_tail,
};
use crate::{App, Message};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Element, Length, Theme};

mod context;
mod usage;

impl App {
    #[allow(clippy::too_many_lines)]
    pub(super) fn view_sidebar(&self) -> Element<'_, Message> {
        const CWD_PREVIEW_MAX: usize = 36;
        let cwd_display = self.cwd.display().to_string();
        let cwd_short = shorten_tail(&cwd_display, CWD_PREVIEW_MAX);
        let active_label = if self.current_session_title.trim().is_empty() {
            "새 채팅".to_string()
        } else {
            self.current_session_title.clone()
        };
        let mut sessions_col = column![
            container(
                text(format!("📌 {active_label}"))
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
            }),
        ]
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
                button(text(format!("📂 {title}")).size(FS_BODY))
                    .on_press(Message::SwitchSession(s.id))
                    .padding([4, 8])
                    .width(Length::Fill)
                    .style(secondary_btn),
                trailing,
            ]
            .spacing(2);
            sessions_col = sessions_col.push(row_widget);
        }

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
            self.view_sidebar_context_area(),
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
}
