use super::ui::*;
use crate::*;
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    pub(crate) fn view_usage_summary(&self) -> Element<'_, Message> {
        const MODEL_ID_PREVIEW_MAX: usize = 24;
        if self.usage.by_model.is_empty() {
            return text("(사용 기록 없음)").size(FS_LABEL).into();
        }
        let mut entries: Vec<(&String, &session::ModelUsage)> =
            self.usage.by_model.iter().collect();
        entries.sort_by(|a, b| {
            b.1.total_cost
                .partial_cmp(&a.1.total_cost)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut col = column![].spacing(2);
        for (id, u) in entries.iter().take(5) {
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

    pub(crate) fn view_sidebar(&self) -> Element<'_, Message> {
        const CWD_PREVIEW_MAX: usize = 36;
        let cwd_display = self.cwd.display().to_string();
        let cwd_short = shorten_tail(&cwd_display, CWD_PREVIEW_MAX);
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
}
