use super::super::ui::{FS_LABEL, semibold_font, shorten_tail};
use crate::{App, Message, session};
use iced::widget::{Space, column, row, text};
use iced::{Element, Font, Length};

impl App {
    pub(super) fn view_usage_summary(&self) -> Element<'_, Message> {
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
                text(format!("${total:.4}"))
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono")),
            ]
            .spacing(6),
        );
        col.into()
    }
}
