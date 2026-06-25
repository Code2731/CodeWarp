use super::TabHealth;
use crate::view::ui::{FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE, primary_btn, semibold_font};
use crate::{App, Message, SettingsTab};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};

impl App {
    pub(crate) fn settings_tab_data(
        &self,
        local_model_count: usize,
    ) -> (&str, String, String, Option<Message>) {
        match self.ui.settings_tab {
            SettingsTab::Provider => {
                let action = if self.has_key || !self.tabby_url_input.trim().is_empty() {
                    "권장 액션: 현재 설정 유지 후 연결 테스트를 가끔 실행해 주세요.".to_string()
                } else {
                    "권장 액션: OpenRouter 키 저장 또는 로컬 endpoint URL을 먼저 등록해 주세요."
                        .to_string()
                };
                let (label, msg) = if !self.tabby_url_input.trim().is_empty() {
                    ("연결 테스트", Some(Message::FetchTabbyModels))
                } else if !self.key_input.trim().is_empty() {
                    ("키 저장", Some(Message::SaveKey))
                } else {
                    ("키 저장", None)
                };
                ("Provider", action, label.to_string(), msg)
            }
            SettingsTab::Runtime => {
                let runtime_can_start = self.can_attempt_start_inference();
                let action = if self.inference_pid.is_some() {
                    "권장 액션: 현재 로그를 확인하고 필요한 경우 중지 후 모델을 교체하세요."
                        .to_string()
                } else {
                    "권장 액션: 엔진/모델(또는 커스텀 명령) 입력 후 시작 버튼을 눌러주세요."
                        .to_string()
                };
                let (label, msg) = if self.inference_pid.is_some() {
                    ("중지", Some(Message::StopInference))
                } else if runtime_can_start {
                    ("시작", Some(Message::StartInference))
                } else {
                    ("시작", None)
                };
                ("Runtime", action, label.to_string(), msg)
            }
            SettingsTab::Models => {
                let action = if local_model_count > 0 {
                    "권장 액션: 다운로드된 모델을 Runtime 탭에서 선택해 실행해 보세요.".to_string()
                } else {
                    "권장 액션: 추천 프리셋에서 1개를 선택해 먼저 다운로드해 주세요.".to_string()
                };
                let (label, msg) = if local_model_count > 0 {
                    (
                        "Runtime 탭으로",
                        Some(Message::SetSettingsTab(SettingsTab::Runtime)),
                    )
                } else if self.hf_dl.is_none() {
                    ("기본 EXL2 다운로드", Some(Message::DownloadExl2Preset(0)))
                } else {
                    ("기본 EXL2 다운로드", None)
                };
                ("Models", action, label.to_string(), msg)
            }
            SettingsTab::Mcp => {
                let action = if self.mcp_servers.is_empty() {
                    "권장 액션: 서버 이름과 명령을 입력해 MCP 서버를 하나 추가해 주세요."
                        .to_string()
                } else if self.mcp_tools.is_empty() {
                    "권장 액션: 서버 명령이 유효한지 확인하고 tools 로드를 기다려 주세요."
                        .to_string()
                } else {
                    "권장 액션: 채팅에서 MCP 도구 호출이 정상 동작하는지 점검해 주세요.".to_string()
                };
                let (label, msg) = if !self.mcp_input.name_input.trim().is_empty()
                    && !self.mcp_input.command_input.trim().is_empty()
                {
                    ("서버 추가", Some(Message::AddMcpServer))
                } else {
                    ("서버 추가", None)
                };
                ("MCP", action, label.to_string(), msg)
            }
        }
    }

    pub(crate) fn view_settings_status_badge(health: TabHealth) -> Element<'static, Message> {
        let label = match health {
            TabHealth::Good => "정상",
            TabHealth::Warn => "설정 필요",
            TabHealth::Bad => "오류",
        };
        container(
            row![
                text("●").size(FS_MICRO).style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let color = match health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    iced::widget::text::Style { color: Some(color) }
                }),
                text(label).size(FS_MICRO).font(semibold_font()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([3, 8])
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match health {
                TabHealth::Good => p.success.base.color,
                TabHealth::Warn => p.primary.base.color,
                TabHealth::Bad => p.danger.base.color,
            };
            container::Style {
                background: Some(iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.12).into()),
                border: iced::Border {
                    color: accent,
                    width: 1.0,
                    radius: 999.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
    }
    pub(crate) fn view_settings_active_action_hint(
        health: TabHealth,
        title: &str,
        action_text: String,
        quick_label: String,
        quick_msg: Option<Message>,
    ) -> Element<'_, Message> {
        let header = row![
            text(format!("{title} 상세"))
                .size(FS_SUBTITLE)
                .font(semibold_font()),
            Space::new().width(Length::Fill),
            Self::view_settings_status_badge(health),
        ]
        .align_y(Alignment::Center);
        let quick_btn: Element<Message> = button(text(quick_label).size(FS_BODY))
            .on_press_maybe(quick_msg)
            .padding([6, 12])
            .style(primary_btn)
            .into();
        let action_hint = container(
            row![
                text(action_text).size(FS_LABEL).width(Length::Fill),
                quick_btn,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match health {
                TabHealth::Good => p.success.base.color,
                TabHealth::Warn => p.primary.base.color,
                TabHealth::Bad => p.danger.base.color,
            };
            container::Style {
                background: Some(iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.08).into()),
                border: iced::Border {
                    color: accent,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            }
        });
        container(column![header, action_hint].spacing(6))
            .padding([8, 10])
            .width(Length::Fill)
            .into()
    }
}
