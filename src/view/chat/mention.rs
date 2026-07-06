// chat_mention.rs — Mention popup + attachment row (view child module)
use crate::view::ui::{
    FS_BODY, FS_LABEL, FS_MICRO, app_vscrollbar, dark_scrollable, primary_btn, secondary_btn,
    shorten_tail,
};
use crate::{App, Message, fuzzy_match_paths, hscrollbar};
use iced::widget::scrollable::Direction;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text, tooltip};
use iced::{Alignment, Color, Element, Length, Shadow, Theme, Vector};

impl App {
    pub(crate) fn view_mention_popup(&self) -> Element<'_, Message> {
        if self.ui.show_mention {
            let filtered = fuzzy_match_paths(&self.mention_candidates, &self.mention_query, 8);
            if filtered.is_empty() {
                Space::new().height(Length::Shrink).into()
            } else {
                let mut list = column![].spacing(2);
                for (i, path) in filtered.iter().enumerate() {
                    let label = path.to_string_lossy().to_string();
                    let is_selected = i == self.mention_selected;
                    list = list.push(
                        button(text(label).size(FS_BODY))
                            .on_press(Message::MentionConfirm)
                            .padding([6, 10])
                            .width(Length::Fill)
                            .style(if is_selected {
                                primary_btn
                            } else {
                                secondary_btn
                            }),
                    );
                }
                container(
                    scrollable(list)
                        .direction(Direction::Vertical(app_vscrollbar()))
                        .style(dark_scrollable)
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
                        shadow: Shadow {
                            color: Color::BLACK,
                            offset: Vector::new(0.0, 4.0),
                            blur_radius: 12.0,
                        },
                        ..Default::default()
                    }
                })
                .into()
            }
        } else {
            Space::new().height(Length::Shrink).into()
        }
    }

    pub(crate) fn view_attach_row(&self) -> Element<'_, Message> {
        if self.attached_files.is_empty() {
            Space::new().height(Length::Shrink).into()
        } else {
            let mut chips = row![].spacing(6).align_y(Alignment::Center);
            for (i, (path, _)) in self.attached_files.iter().enumerate() {
                let rel_path = path.strip_prefix(&self.cwd).unwrap_or(path.as_path());
                let name = shorten_tail(&rel_path.display().to_string(), 36);
                let chip = container(
                    row![
                        text(format!("📄 {name}")).size(FS_LABEL),
                        tooltip(
                            button(text("✕").size(FS_MICRO))
                                .on_press(Message::RemoveAttachment(i))
                                .padding([1, 4])
                                .style(secondary_btn),
                            text("파일 제거").size(FS_MICRO),
                            Position::Bottom,
                        ),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center),
                )
                .padding([3, 8])
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    container::Style {
                        background: Some(
                            (if self.hovered_attach_idx == Some(i) {
                                Color::from_rgba(
                                    p.primary.base.color.r,
                                    p.primary.base.color.g,
                                    p.primary.base.color.b,
                                    0.12,
                                )
                            } else {
                                p.background.strong.color
                            })
                            .into(),
                        ),
                        border: iced::Border {
                            color: if self.hovered_attach_idx == Some(i) {
                                p.primary.base.color
                            } else {
                                p.primary.weak.color
                            },
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                });
                chips = chips.push(
                    mouse_area(chip)
                        .on_enter(Message::AttachChipHovered(Some(i)))
                        .on_exit(Message::AttachChipHovered(None)),
                );
            }
            container(
                scrollable(chips)
                    .direction(Direction::Horizontal(hscrollbar()))
                    .style(dark_scrollable)
                    .width(Length::Fill),
            )
            .padding([4, 0])
            .into()
        }
    }
}
