use crate::Message;
use iced::widget::{column, container, text};
use iced::{Color, Element, Length};

pub(crate) fn pulse_opacity(phase: u8) -> f32 {
    match phase {
        0 | 3 => 0.5,
        _ => 1.0,
    }
}

pub(crate) fn view_skeleton_block(phase: u8) -> Element<'static, Message> {
    let op = pulse_opacity(phase);

    let title_bar = container(text(""))
        .height(Length::Fixed(16.0))
        .width(Length::Fixed(130.0))
        .style(move |_: &iced::Theme| {
            let c = Color::from_rgba(1.0, 1.0, 1.0, 0.08 * op);
            container::Style {
                background: Some(c.into()),
                border: iced::Border {
                    radius: 10.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let line1 = container(text(""))
        .height(Length::Fixed(12.0))
        .width(Length::Fill)
        .style(move |_: &iced::Theme| {
            let c = Color::from_rgba(1.0, 1.0, 1.0, 0.08 * op);
            container::Style {
                background: Some(c.into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let line2 = container(text(""))
        .height(Length::Fixed(12.0))
        .width(Length::Fixed(380.0))
        .style(move |_: &iced::Theme| {
            let c = Color::from_rgba(1.0, 1.0, 1.0, 0.15 * op);
            container::Style {
                background: Some(c.into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    let line3 = container(text(""))
        .height(Length::Fixed(12.0))
        .width(Length::Fixed(260.0))
        .style(move |_: &iced::Theme| {
            let c = Color::from_rgba(1.0, 1.0, 1.0, 0.08 * op);
            container::Style {
                background: Some(c.into()),
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        });

    column![title_bar, line1, line2, line3]
        .spacing(8)
        .width(Length::Fill)
        .into()
}
