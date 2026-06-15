use super::ui::*;
use crate::*;
use iced::widget::{button, column, container, pick_list, row, text, text_input, Space};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    /// inference 서버 (xLLM/vLLM/llama-server/TabbyML/TabbyAPI/Ollama/Custom) — dropdown 기반.
    /// CodeWarp가 child process로 spawn 관리.
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

        let binary_section: Element<Message> = if matches!(
            self.inference_engine,
            InferenceEngine::Ollama | InferenceEngine::Custom
        ) {
            Space::new().height(Length::Shrink).into()
        } else {
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
            let install_btn: Element<Message> =
                if self.inference_engine == InferenceEngine::TabbyApi {
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
        };

        let tabbyapi_launcher_hint: Element<Message> = if self.inference_engine
            == InferenceEngine::TabbyApi
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
        };

        let model_section: Element<Message> = match self.inference_engine {
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
        };

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

        let running = self.inference_pid.is_some();
        let can_start = self.can_attempt_start_inference();

        let actions: Element<Message> = if running {
            let running_label = if let Some(pid) = self.inference_pid {
                format!("● 실행 중 (pid {})", pid)
            } else {
                "● 실행 중".to_string()
            };
            row![
                text(running_label).size(FS_LABEL).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.extended_palette().success.base.color),
                    }
                }),
                Space::new().width(Length::Fill),
                button(text("중지").size(FS_LABEL))
                    .on_press(Message::StopInference)
                    .padding([4, 12])
                    .style(danger_btn),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        } else {
            let btn_label = if self.inference_engine == InferenceEngine::Ollama {
                "Ollama 연결"
            } else {
                "시작"
            };
            row![
                text("● 미실행")
                    .size(FS_LABEL)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().background.strong.color),
                    }),
                Space::new().width(Length::Fill),
                button(text(btn_label).size(FS_LABEL))
                    .on_press_maybe(if can_start {
                        Some(Message::StartInference)
                    } else {
                        None
                    })
                    .padding([4, 12])
                    .style(primary_btn),
            ]
            .spacing(8)
            .align_y(Alignment::Center)
            .into()
        };

        let log_section: Element<Message> = if self.inference_log.is_empty() {
            Space::new().height(Length::Shrink).into()
        } else {
            let mut col =
                column![text("로그 (최근)").size(FS_MICRO).font(semibold_font())].spacing(1);
            for line in &self.inference_log {
                col = col.push(
                    text(line.clone())
                        .size(FS_MICRO)
                        .font(Font::with_name("JetBrains Mono")),
                );
            }
            container(col).padding([6, 10]).style(panel_style).into()
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
                binary_section,
                tabbyapi_launcher_hint,
                model_section,
                port_section,
                actions,
                log_section
            ]
            .spacing(8),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }
}
