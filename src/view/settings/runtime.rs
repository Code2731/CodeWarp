use crate::view::ui::{FS_BODY, FS_LABEL, FS_SUBTITLE, field_input, panel_style, semibold_font};
use crate::{App, InferenceEngine, Message};
use iced::widget::{Space, column, container, pick_list, row, text, text_input};
use iced::{Alignment, Element, Length};

impl App {
    pub(crate) fn view_inference_runner(&self) -> Element<'_, Message> {
        let header = text("inference 서버 (CodeWarp가 spawn 관리)")
            .size(FS_SUBTITLE)
            .font(semibold_font());

        let engine_pick: Element<Message> = pick_list(
            InferenceEngine::ALL,
            Some(self.inference_engine),
            Message::SelectInferenceEngine,
        )
        .placeholder("엔진 선택")
        .text_size(FS_BODY)
        .into();

        let port_section: Element<Message> = match self.inference_engine {
            InferenceEngine::Custom => Space::new().height(Length::Shrink).into(),
            _ => row![
                text("포트").size(FS_LABEL).font(semibold_font()),
                text_input("9000", &self.inference_port_input)
                    .on_input(Message::InferencePortChanged)
                    .padding(6)
                    .size(FS_BODY)
                    .style(field_input)
                    .width(Length::Fixed(100.0)),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into(),
        };

        container(
            column![
                header,
                row![
                    text("엔진").size(FS_LABEL).font(semibold_font()),
                    engine_pick
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                self.view_inference_binary_section(),
                self.view_tabbyapi_launcher_hint(),
                self.view_inference_model_section(),
                port_section,
                self.view_inference_actions(),
                self.view_inference_log_section(),
            ]
            .spacing(8),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }
}
