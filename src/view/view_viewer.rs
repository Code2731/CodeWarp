use super::{Color, FS_LABEL, Message, hscrollbar};
use crate::view::ui::{dark_scrollable, secondary_btn};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, text};
use iced::{Element, Font, Length, Theme};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};

pub(super) struct CodewarpViewer<'a> {
    pub(super) hovered_set: &'a HashSet<u64>,
    pub(super) on_hover: fn(u64, bool) -> Message,
}

static NEXT_CODE_BLOCK_ID: AtomicU64 = AtomicU64::new(0);

impl<'a> iced::widget::markdown::Viewer<'a, Message> for CodewarpViewer<'a> {
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
                    if let Some(font) = s.font.as_mut()
                        && !matches!(font.style, iced::font::Style::Normal)
                    {
                        font.style = iced::font::Style::Normal;
                        if matches!(font.weight, iced::font::Weight::Normal) {
                            font.weight = iced::font::Weight::Semibold;
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
        let id = NEXT_CODE_BLOCK_ID.fetch_add(1, Ordering::Relaxed);
        let is_hovered = self.hovered_set.contains(&id);
        let language_label = language.unwrap_or("text").to_ascii_lowercase();

        let copy_btn: Element<Message> = if is_hovered {
            button(
                text("⎘")
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono")),
            )
            .on_press(Message::CopyText(code.to_string()))
            .padding([3, 8])
            .style(secondary_btn)
            .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        };

        let header = row![
            container(
                text(language_label)
                    .size(FS_LABEL)
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
            copy_btn,
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
            .style(dark_scrollable)
            .width(Length::Fill);

        let inner = container(column![header, code_body].spacing(0))
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
            });

        mouse_area(inner)
            .on_enter((self.on_hover)(id, true))
            .on_exit((self.on_hover)(id, false))
            .into()
    }
}
