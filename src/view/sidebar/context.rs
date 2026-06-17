use super::super::ui::*;
use crate::*;
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Font, Length};

impl App {
    pub(crate) fn view_sidebar_context_area(&self) -> Element<'_, Message> {
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

        if self.attached_files.is_empty() {
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
            .into()
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
            .into()
        }
    }
}
