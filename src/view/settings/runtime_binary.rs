use crate::view::ui::*;
use crate::*;
use iced::widget::{button, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

impl App {
    pub(crate) fn view_inference_binary_section(&self) -> Element<'_, Message> {
        if matches!(
            self.inference_engine,
            InferenceEngine::Ollama | InferenceEngine::Custom
        ) {
            return Space::new().height(Length::Shrink).into();
        }
        let binary_label = if self.inference_engine == InferenceEngine::TabbyApi {
            "TabbyAPI script"
        } else {
            "바이너리"
        };
        let binary_placeholder = if self.inference_engine == InferenceEngine::TabbyApi {
            "Windows: Start.bat/Start.cmd / macOS: start.sh 또는 main.py"
        } else {
            "PATH 기본값 사용, 필요 시 실제 실행 파일 경로"
        };
        let pick_label = if self.inference_engine == InferenceEngine::TabbyApi {
            "script 선택"
        } else {
            "📁"
        };
        let install_btn: Element<Message> = if self.inference_engine == InferenceEngine::TabbyApi {
            button(text("TabbyAPI 설치").size(FS_LABEL))
                .on_press_maybe(if self.busy {
                    None
                } else {
                    Some(Message::InstallTabbyApiRuntime)
                })
                .padding([4, 8])
                .style(secondary_btn)
                .into()
        } else {
            Space::new().width(Length::Shrink).into()
        };
        row![
            text(binary_label).size(FS_LABEL).font(semibold_font()),
            text_input(binary_placeholder, &self.inference_binary_path,)
                .on_input(Message::InferenceBinaryChanged)
                .padding(6)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fixed(300.0)),
            button(text(pick_label).size(FS_LABEL))
                .on_press(Message::PickInferenceBinary)
                .padding([4, 8])
                .style(secondary_btn),
            install_btn,
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }

    pub(crate) fn view_tabbyapi_launcher_hint(&self) -> Element<'_, Message> {
        if self.inference_engine == InferenceEngine::TabbyApi
            && self.inference_binary_path.trim().is_empty()
        {
            container(
                text("EXL2 모델 다운로드만으로는 서버가 실행되지 않습니다. TabbyAPI 프로젝트의 Start.bat/Start.cmd/start.sh/main.py를 script로 지정해야 합니다.")
                    .size(FS_LABEL),
            )
                .padding([6, 10])
                .style(panel_style)
                .into()
        } else {
            Space::new().height(Length::Shrink).into()
        }
    }
}
