use super::settings_health::TabHealth;
use super::ui::*;
use crate::*;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Length, Theme};

impl App {
    pub(crate) fn view_settings_status_panel(
        &self,
        active_health: TabHealth,
        local_model_count: usize,
    ) -> (
        Element<'_, Message>,
        Element<'_, Message>,
        Element<'_, Message>,
    ) {
        let summary = row![text(format!(
            "Providers: {}  •  Runtime: {}  •  Models: {}  •  MCP: {}",
            if self.has_key || !self.tabby_url_input.trim().is_empty() {
                "configured"
            } else {
                "empty"
            },
            if self.inference_pid.is_some() {
                "running"
            } else {
                "stopped"
            },
            local_model_count,
            self.mcp_servers.len()
        ))
        .size(FS_LABEL)];

        let (active_tab_title, _active_health, active_action, quick_label, quick_action) =
            match self.ui.settings_tab {
                SettingsTab::Provider => (
                    "Provider",
                    active_health,
                    if self.has_key || !self.tabby_url_input.trim().is_empty() {
                        "권장 액션: 현재 설정 유지 후 연결 테스트를 가끔 실행해 주세요.".to_string()
                    } else {
                        "권장 액션: OpenRouter 키 저장 또는 로컬 endpoint URL을 먼저 등록해 주세요."
                            .to_string()
                    },
                    if !self.tabby_url_input.trim().is_empty() {
                        "연결 테스트"
                    } else {
                        "키 저장"
                    },
                    if !self.tabby_url_input.trim().is_empty() {
                        Some(Message::FetchTabbyModels)
                    } else if !self.key_input.trim().is_empty() {
                        Some(Message::SaveKey)
                    } else {
                        None
                    },
                ),
                SettingsTab::Runtime => {
                    let runtime_can_start = self.can_attempt_start_inference();
                    (
                        "Runtime",
                        active_health,
                        if self.inference_pid.is_some() {
                            "권장 액션: 현재 로그를 확인하고 필요한 경우 중지 후 모델을 교체하세요."
                                .to_string()
                        } else {
                            "권장 액션: 엔진/모델(또는 커스텀 명령) 입력 후 시작 버튼을 눌러주세요."
                                .to_string()
                        },
                        if self.inference_pid.is_some() {
                            "중지"
                        } else {
                            "시작"
                        },
                        if self.inference_pid.is_some() {
                            Some(Message::StopInference)
                        } else if runtime_can_start {
                            Some(Message::StartInference)
                        } else {
                            None
                        },
                    )
                }
                SettingsTab::Models => (
                    "Models",
                    active_health,
                    if local_model_count > 0 {
                        "권장 액션: 다운로드된 모델을 Runtime 탭에서 선택해 실행해 보세요."
                            .to_string()
                    } else {
                        "권장 액션: 추천 프리셋에서 1개를 선택해 먼저 다운로드해 주세요."
                            .to_string()
                    },
                    if local_model_count > 0 {
                        "Runtime 탭으로"
                    } else {
                        "기본 EXL2 다운로드"
                    },
                    if local_model_count > 0 {
                        Some(Message::SetSettingsTab(SettingsTab::Runtime))
                    } else if self.hf_dl.is_none() {
                        Some(Message::DownloadExl2Preset(0))
                    } else {
                        None
                    },
                ),
                SettingsTab::Mcp => (
                    "MCP",
                    active_health,
                    if self.mcp_servers.is_empty() {
                        "권장 액션: 서버 이름과 명령을 입력해 MCP 서버를 하나 추가해 주세요."
                            .to_string()
                    } else if self.mcp_tools.is_empty() {
                        "권장 액션: 서버 명령이 유효한지 확인하고 tools 로드를 기다려 주세요."
                            .to_string()
                    } else {
                        "권장 액션: 채팅에서 MCP 도구 호출이 정상 동작하는지 점검해 주세요."
                            .to_string()
                    },
                    "서버 추가",
                    if !self.mcp_input.name_input.trim().is_empty()
                        && !self.mcp_input.command_input.trim().is_empty()
                    {
                        Some(Message::AddMcpServer)
                    } else {
                        None
                    },
                ),
            };
        let badge_label = match active_health {
            TabHealth::Good => "정상",
            TabHealth::Warn => "설정 필요",
            TabHealth::Bad => "오류",
        };
        let status_badge = container(
            row![
                text("●").size(FS_MICRO).style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let color = match active_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    iced::widget::text::Style { color: Some(color) }
                }),
                text(badge_label).size(FS_MICRO).font(semibold_font()),
            ]
            .spacing(4)
            .align_y(Alignment::Center),
        )
        .padding([3, 8])
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match active_health {
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
        });
        let active_header = row![
            text(format!("{active_tab_title} 상세"))
                .size(FS_SUBTITLE)
                .font(semibold_font()),
            Space::new().width(Length::Fill),
            status_badge,
        ]
        .align_y(Alignment::Center);
        let quick_btn: Element<Message> = button(text(quick_label).size(FS_BODY))
            .on_press_maybe(quick_action)
            .padding([6, 12])
            .style(primary_btn)
            .into();
        let active_action_hint = container(
            row![
                text(active_action).size(FS_LABEL).width(Length::Fill),
                quick_btn,
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match active_health {
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

        (
            summary.into(),
            active_header.into(),
            active_action_hint.into(),
        )
    }
}
