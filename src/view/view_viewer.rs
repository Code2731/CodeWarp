use super::*;
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Element, Font, Length, Theme};

/// markdown::view_with용 커스텀 Viewer.
#[derive(Debug)]
pub(super) struct CodewarpViewer;

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
