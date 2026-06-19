use crate::*;
use iced::widget::{row, text};
use iced::{Alignment, Element, Theme};

impl App {
    pub(crate) fn endpoint_indicator(&self, size: f32) -> Element<'_, Message> {
        #[derive(Clone, Copy)]
        enum Kind {
            Ok,
            Err,
            Unknown,
        }
        let (kind, label): (Kind, String) = match &self.tabby_status {
            Some(Ok(s)) => (Kind::Ok, format!("연결됨 — {}", s)),
            Some(Err(e)) => (Kind::Err, format!("끊김 — {}", e)),
            None => (Kind::Unknown, "endpoint 미시도".into()),
        };
        let dot = text("●").size(size).style(move |theme: &Theme| {
            let p = theme.extended_palette();
            iced::widget::text::Style {
                color: Some(match kind {
                    Kind::Ok => p.success.base.color,
                    Kind::Err => p.danger.base.color,
                    Kind::Unknown => p.background.strong.color,
                }),
            }
        });
        row![dot, text(label).size(size)]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
    }
}
