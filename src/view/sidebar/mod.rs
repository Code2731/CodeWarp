use super::ui::{
    FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE, PAD_LG, PAD_MD, PAD_XS, SCROLL_GUTTER_PAD_X,
    SPACE_SM, SPACE_XS, app_vscrollbar, danger_btn, dark_scrollable, field_input, panel_style,
    primary_btn, secondary_btn, semibold_font, shorten_tail,
};
use crate::{App, InactiveSession, Message};
use iced::widget::scrollable::Direction;
use iced::widget::tooltip::Position;
use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, text, text_input, tooltip,
};
use iced::{Alignment, Color, Element, Length, Theme};

mod context;
mod file_tree;
mod usage;

impl App {
    fn view_active_session_label(&self) -> Element<'_, Message> {
        let active_label = if self.current_session_title.trim().is_empty() {
            "새 채팅".to_string()
        } else {
            self.current_session_title.clone()
        };
        container(
            text(format!("📌 {active_label}"))
                .size(FS_BODY)
                .font(semibold_font()),
        )
        .padding([6, 8])
        .width(Length::Fill)
        .style(|theme: &Theme| {
            let p = theme.extended_palette();
            container::Style {
                background: Some(
                    Color::from_rgba(
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
        })
        .into()
    }

    fn view_session_list_empty(&self) -> Option<Element<'_, Message>> {
        if self.ui.renaming_session_id.is_some() {
            return None;
        }
        if self.inactive_sessions.is_empty() {
            Some(
                text("저장된 세션이 없습니다")
                    .size(FS_MICRO)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(Color::from_rgba(
                            theme.extended_palette().background.strong.text.r,
                            theme.extended_palette().background.strong.text.g,
                            theme.extended_palette().background.strong.text.b,
                            0.5,
                        )),
                    })
                    .into(),
            )
        } else {
            Some(Space::new().height(Length::Shrink).into())
        }
    }

    fn view_session_trailing(&self, sid: u64) -> Element<'_, Message> {
        let is_pending = self.ui.pending_delete_session == Some(sid);
        let is_renaming = self.ui.renaming_session_id == Some(sid);
        if is_pending {
            let confirm: Element<Message> = tooltip(
                button(text("✓").size(FS_MICRO))
                    .on_press(Message::DeleteSession(sid))
                    .padding([2, 6])
                    .style(primary_btn),
                text("삭제 확인").size(FS_MICRO),
                Position::Bottom,
            )
            .into();
            let cancel: Element<Message> = tooltip(
                button(text("✗").size(FS_MICRO))
                    .on_press(Message::CancelDeleteSession)
                    .padding([2, 6])
                    .style(secondary_btn),
                text("취소").size(FS_MICRO),
                Position::Bottom,
            )
            .into();
            row![confirm, cancel].spacing(2).into()
        } else if is_renaming {
            Space::new().width(Length::Shrink).into()
        } else {
            let rename: Element<Message> = tooltip(
                button(text("✎").size(FS_MICRO))
                    .on_press(Message::StartRenameSession(sid))
                    .padding([2, 4])
                    .style(secondary_btn),
                text("세션 이름 변경").size(FS_MICRO),
                Position::Bottom,
            )
            .into();
            let del: Element<Message> = tooltip(
                button(text("✕").size(FS_MICRO))
                    .on_press(Message::AskDeleteSession(sid))
                    .padding([2, 6])
                    .style(danger_btn),
                text("세션 삭제").size(FS_MICRO),
                Position::Bottom,
            )
            .into();
            row![rename, del].spacing(2).into()
        }
    }

    fn view_session_row_content(&self, s: &InactiveSession) -> Element<'_, Message> {
        let sid = s.id;
        let is_renaming = self.ui.renaming_session_id == Some(sid);
        let title = if s.title.trim().is_empty() {
            "(빈 세션)".to_string()
        } else {
            s.title.clone()
        };
        let msg_count = s.conversation.len();
        if is_renaming {
            let rename_input = self.ui.rename_input.clone();
            let input: Element<Message> = text_input("세션 이름…", &self.ui.rename_input)
                .on_input(move |v| Message::RenameSession(sid, v))
                .on_submit(Message::RenameSession(sid, rename_input))
                .padding([4, 6])
                .size(FS_BODY)
                .style(field_input)
                .into();
            let confirm: Element<Message> = tooltip(
                button(text("✓").size(FS_MICRO))
                    .on_press(Message::RenameSession(sid, self.ui.rename_input.clone()))
                    .padding([2, 6])
                    .style(primary_btn),
                text("확인").size(FS_MICRO),
                Position::Bottom,
            )
            .into();
            let cancel: Element<Message> = tooltip(
                button(text("✗").size(FS_MICRO))
                    .on_press(Message::CancelRenameSession)
                    .padding([2, 6])
                    .style(secondary_btn),
                text("취소").size(FS_MICRO),
                Position::Bottom,
            )
            .into();
            row![input, confirm, cancel]
                .spacing(2)
                .align_y(Alignment::Center)
                .into()
        } else {
            let session_btn: Element<Message> = button(
                row![
                    text(format!("📂 {title}")).size(FS_BODY),
                    text(format!(" {msg_count}"))
                        .size(FS_MICRO)
                        .style(|theme: &Theme| iced::widget::text::Style {
                            color: Some(theme.extended_palette().background.strong.text),
                        }),
                ]
                .align_y(Alignment::Center),
            )
            .on_press(Message::SwitchSession(sid))
            .padding([4, 8])
            .width(Length::Fill)
            .style(secondary_btn)
            .into();
            row![session_btn, self.view_session_trailing(sid)]
                .spacing(2)
                .into()
        }
    }

    fn view_session_row(&self, s: &InactiveSession) -> Element<'_, Message> {
        let is_hovered = self.hovered_session == Some(s.id);
        let row_content = self.view_session_row_content(s);
        container(
            mouse_area(row_content)
                .on_enter(Message::SessionHovered(Some(s.id)))
                .on_exit(Message::SessionHovered(None)),
        )
        .style(move |theme: &Theme| {
            if is_hovered {
                let p = theme.extended_palette();
                container::Style {
                    background: Some(
                        Color::from_rgba(
                            p.primary.weak.color.r,
                            p.primary.weak.color.g,
                            p.primary.weak.color.b,
                            0.12,
                        )
                        .into(),
                    ),
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            } else {
                container::Style {
                    border: iced::Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            }
        })
        .into()
    }

    fn view_sidebar_body(&self) -> Element<'_, Message> {
        let cwd_short = shorten_tail(&self.cwd.display().to_string(), 36);
        let mut sessions_col = column![self.view_active_session_label()].spacing(2);
        if let Some(empty) = self.view_session_list_empty() {
            sessions_col = sessions_col.push(empty);
        }
        let search_q = self.ui.session_search.to_lowercase();
        for s in &self.inactive_sessions {
            if !search_q.is_empty() && !s.title.to_lowercase().contains(&search_q) {
                continue;
            }
            sessions_col = sessions_col.push(self.view_session_row(s));
        }
        column![
            button(text("＋ 새 채팅").size(FS_SUBTITLE).font(semibold_font()))
                .on_press(Message::NewChat)
                .padding([6, 12])
                .width(Length::Fill)
                .style(primary_btn),
            Space::new().height(Length::Fixed(8.0)),
            text("채팅").size(FS_LABEL).font(semibold_font()),
            text_input("세션 검색…", &self.ui.session_search)
                .on_input(Message::SessionSearchChanged)
                .padding([4, 6])
                .size(FS_MICRO)
                .style(field_input),
            scrollable(sessions_col)
                .direction(Direction::Vertical(app_vscrollbar(),))
                .style(dark_scrollable)
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
            self.view_file_tree(),
            Space::new().height(Length::Fixed(14.0)),
            self.view_sidebar_context_area(),
        ]
        .spacing(SPACE_SM)
        .into()
    }

    fn view_resize_row(&self) -> Element<'_, Message> {
        row![
            text(format!("너비 {:.0}px", self.sidebar_width)).size(FS_LABEL),
            Space::new().width(Length::Fill),
            tooltip(
                button(text("◀▶").size(FS_LABEL).font(semibold_font()))
                    .on_press(Message::CycleSidebarWidth)
                    .padding([PAD_XS, PAD_MD])
                    .style(secondary_btn),
                text("사이드바 너비 변경").size(FS_MICRO),
                Position::Bottom,
            ),
        ]
        .spacing(SPACE_XS)
        .align_y(Alignment::Center)
        .into()
    }

    pub(super) fn view_sidebar(&self) -> Element<'_, Message> {
        container(
            column![
                scrollable(container(self.view_sidebar_body()).padding([0, SCROLL_GUTTER_PAD_X]),)
                    .direction(Direction::Vertical(app_vscrollbar()))
                    .style(dark_scrollable)
                    .height(Length::Fill),
                self.view_resize_row(),
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
