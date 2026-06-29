use super::ui::{
    CONTROL_PAD_X, CONTROL_PAD_Y, FS_BODY, FS_HEADING, TOPBAR_PAD_X, TOPBAR_PAD_Y,
    TOPBAR_ROW_SPACING, panel_style, secondary_btn, topbar_style,
};
use crate::{App, Message};
use iced::widget::{Space, button, checkbox, combo_box, container, row, text};
use iced::{Alignment, Element, Length};

impl App {
    pub(super) fn view_topbar(&self) -> Element<'_, Message> {
        let model_picker: Element<Message> = if self.model_ids.is_empty() {
            container(text("모델 없음").size(FS_BODY))
                .padding([6, 10])
                .style(panel_style)
                .into()
        } else {
            {
                let selected_opt = self
                    .selected_model
                    .as_ref()
                    .and_then(|id| self.model_options.iter().find(|o| &o.id == id));
                iced::widget::container(
                    combo_box(
                        &self.model_combo_state,
                        "모델 검색…",
                        selected_opt,
                        Message::SelectModel,
                    )
                    .size(FS_BODY),
                )
                .width(Length::FillPortion(2))
                .max_width(420.0)
                .into()
            }
        };

        let is_fav = self
            .selected_model
            .as_ref()
            .is_some_and(|id| self.model_filter.favorites.contains(id));
        let fav_btn = button(text(if is_fav { "★" } else { "☆" }).size(FS_HEADING))
            .on_press(Message::ToggleFavorite)
            .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
            .style(secondary_btn);

        let filters = row![
            checkbox(self.model_filter.filter_coding)
                .label("코딩")
                .on_toggle(Message::ToggleFilterCoding)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.model_filter.filter_reasoning)
                .label("추론")
                .on_toggle(Message::ToggleFilterReasoning)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.model_filter.filter_general)
                .label("범용")
                .on_toggle(Message::ToggleFilterGeneral)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.model_filter.filter_favorites_only)
                .label("⭐만")
                .on_toggle(Message::ToggleFilterFavorites)
                .size(16)
                .text_size(FS_BODY),
            checkbox(self.compare_both)
                .label("둘 다 답변")
                .on_toggle(Message::ToggleCompareBoth)
                .size(16)
                .text_size(FS_BODY),
        ]
        .spacing(TOPBAR_ROW_SPACING)
        .align_y(Alignment::Center);

        let sort_btn = button(text(self.model_filter.sort_mode.label()).size(FS_BODY))
            .on_press(Message::CycleSortMode)
            .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
            .style(secondary_btn);

        let bar = row![
            filters,
            Space::new().width(Length::Fill),
            sort_btn,
            model_picker,
            fav_btn,
            button(text("⚙").size(FS_HEADING).align_y(Alignment::Center))
                .on_press(Message::OpenSettings)
                .padding([CONTROL_PAD_Y, CONTROL_PAD_X])
                .style(secondary_btn),
        ]
        .spacing(TOPBAR_ROW_SPACING)
        .align_y(Alignment::Center);

        container(bar)
            .padding([TOPBAR_PAD_Y, TOPBAR_PAD_X])
            .style(topbar_style)
            .width(Length::Fill)
            .into()
    }
}
