use super::super::ui::{
    CONTEXT_LIST_HEIGHT, FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE, PAD_MD, PAD_SM, PAD_XS, PAD_XXS,
    SPACE_SM, SPACE_XS, SPACE_XXS, app_vscrollbar, context_item_style, danger_btn, dark_scrollable,
    secondary_btn, semibold_font, shorten_tail, with_alpha,
};
use crate::{App, MAX_ATTACH_BYTES, Message, fmt_bytes};
use iced::widget::scrollable::Direction;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text, tooltip};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    fn view_context_quota_label(&self) -> String {
        let total: u64 = self
            .attached_files
            .iter()
            .map(|(_, content)| content.len() as u64)
            .sum();
        format!("{}/{}", fmt_bytes(total), fmt_bytes(MAX_ATTACH_BYTES))
    }

    fn view_context_actions(&self, attached_count: usize) -> Element<'_, Message> {
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
        .into()
    }

    fn view_context_header(&self, count: usize) -> Element<'_, Message> {
        row![
            text(format!("Context ({count})"))
                .size(FS_LABEL)
                .font(semibold_font()),
            Space::new().width(Length::Fill),
            text(self.view_context_quota_label())
                .size(FS_MICRO)
                .font(Font::with_name("JetBrains Mono")),
        ]
        .spacing(SPACE_XS)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_context_empty(&self) -> Element<'_, Message> {
        column![
            self.view_context_header(0),
            self.view_context_actions(0),
            text("No files selected").size(FS_SUBTITLE),
        ]
        .spacing(SPACE_SM)
        .into()
    }

    fn view_context_files(&self) -> Element<'_, Message> {
        let mut context_list = column![].spacing(SPACE_XS);
        for (i, (path, content)) in self.attached_files.iter().enumerate() {
            let name = path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().to_string(),
            );
            let short_name = shorten_tail(&name, 24);
            let rel_path = path.strip_prefix(&self.cwd).unwrap_or(path.as_path());
            let short_path = shorten_tail(&rel_path.display().to_string(), 42);
            let size_label = fmt_bytes(content.len() as u64);
            let is_hovered = self.hovered_context_idx == Some(i);
            let item = container(
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
                    tooltip(
                        button(text("x").size(FS_MICRO))
                            .on_press(Message::RemoveAttachment(i))
                            .padding([PAD_XXS, PAD_XS])
                            .style(danger_btn),
                        text("컨텍스트에서 제거").size(FS_MICRO),
                        Position::Bottom,
                    ),
                ]
                .spacing(SPACE_XS)
                .align_y(Alignment::Center),
            )
            .padding([PAD_XXS, PAD_SM])
            .style(move |theme: &Theme| {
                let mut s = context_item_style(theme);
                if is_hovered {
                    let p = theme.extended_palette();
                    s.border.color = with_alpha(p.primary.base.color, 0.55);
                    s.border.width = 1.5;
                    s.shadow = iced::Shadow {
                        color: with_alpha(p.primary.base.color, 0.12),
                        offset: iced::Vector { x: 0.0, y: 1.0 },
                        blur_radius: 6.0,
                    };
                }
                s
            });
            context_list = context_list.push(
                mouse_area(item)
                    .on_enter(Message::ContextHovered(Some(i)))
                    .on_exit(Message::ContextHovered(None)),
            );
        }
        column![
            self.view_context_header(self.attached_files.len()),
            self.view_context_actions(self.attached_files.len()),
            scrollable(context_list)
                .direction(Direction::Vertical(app_vscrollbar()))
                .style(dark_scrollable)
                .height(Length::Fixed(CONTEXT_LIST_HEIGHT)),
        ]
        .spacing(SPACE_SM)
        .into()
    }

    pub(super) fn view_sidebar_context_area(&self) -> Element<'_, Message> {
        if self.attached_files.is_empty() {
            self.view_context_empty()
        } else {
            self.view_context_files()
        }
    }
}
