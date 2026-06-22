use crate::view::ui::{field_input, panel_style, FS_BODY, FS_LABEL};
use crate::{list_downloaded_models, App, InferenceEngine, Message};
use iced::widget::{container, pick_list, text, text_input};
use iced::{Element, Length};

impl App {
    pub(crate) fn view_inference_model_section(&self) -> Element<'_, Message> {
        match self.inference_engine {
            InferenceEngine::XLlm | InferenceEngine::VLlm | InferenceEngine::LlamaServer => {
                let dl_dir = std::path::PathBuf::from(&self.model_dir_input);
                let models = list_downloaded_models(&dl_dir);
                if models.is_empty() {
                    container(text("받은 모델 없음 — Models 탭에서 먼저 다운로드").size(FS_LABEL))
                        .padding([8, 10])
                        .style(panel_style)
                        .into()
                } else {
                    let selected = if self.inference_selected_model.is_empty() {
                        None
                    } else {
                        Some(self.inference_selected_model.clone())
                    };
                    pick_list(models, selected, Message::SelectInferenceModel)
                        .placeholder("받은 모델 선택")
                        .text_size(FS_BODY)
                        .into()
                }
            }
            InferenceEngine::TabbyMl => text_input(
                "Tabby 카탈로그 (예: TabbyML/Qwen2.5-Coder-7B — Tabby가 자체 다운로드)",
                &self.inference_selected_model,
            )
            .on_input(Message::SelectInferenceModel)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
            InferenceEngine::TabbyApi => text_input(
                "EXL2 model folder path (TabbyAPI Start.bat/start.sh)",
                &self.inference_selected_model,
            )
            .on_input(Message::SelectInferenceModel)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
            InferenceEngine::Ollama => text_input(
                "Ollama 모델 (예: qwen2.5-coder:7b) — daemon은 별도로 떠있어야",
                &self.inference_selected_model,
            )
            .on_input(Message::SelectInferenceModel)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
            InferenceEngine::Custom => text_input(
                "직접 명령 (예: xllm serve --model ... --port 9000)",
                &self.inference_command_input,
            )
            .on_input(Message::InferenceCommandChanged)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(420.0))
            .into(),
        }
    }
}
