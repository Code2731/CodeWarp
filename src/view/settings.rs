use super::ui::*;
use crate::*;
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Alignment, Element, Length, Theme};

impl App {
    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        #[derive(Clone, Copy)]
        enum TabHealth {
            Good,
            Warn,
            Bad,
        }

        let provider_health = match &self.tabby_status {
            Some(Err(_)) => TabHealth::Bad,
            _ if self.has_key || !self.tabby_url_input.trim().is_empty() => TabHealth::Good,
            _ => TabHealth::Warn,
        };
        let runtime_health = if self.inference_pid.is_some() {
            TabHealth::Good
        } else {
            TabHealth::Warn
        };
        let local_model_count =
            list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len();
        let model_health = if local_model_count > 0 {
            TabHealth::Good
        } else {
            TabHealth::Warn
        };
        let mcp_health = if self.mcp_servers.is_empty() || self.mcp_tools.is_empty() {
            TabHealth::Warn
        } else {
            TabHealth::Good
        };

        let tab_btn = |icon: &'static str,
                       label: &'static str,
                       note: String,
                       health: TabHealth,
                       tab: SettingsTab| {
            let dot = text("●").size(FS_MICRO).style(move |theme: &Theme| {
                let p = theme.extended_palette();
                let color = match health {
                    TabHealth::Good => p.success.base.color,
                    TabHealth::Warn => p.primary.base.color,
                    TabHealth::Bad => p.danger.base.color,
                };
                iced::widget::text::Style { color: Some(color) }
            });
            let btn = button(
                column![
                    row![
                        text(icon).size(FS_LABEL),
                        text(label).size(FS_LABEL).font(semibold_font()),
                        dot,
                    ]
                    .spacing(5)
                    .align_y(Alignment::Center),
                    text(note).size(FS_MICRO),
                ]
                .spacing(2),
            )
            .on_press(Message::SetSettingsTab(tab))
            .padding([8, 8])
            .width(Length::FillPortion(1));
            if self.ui.settings_tab == tab {
                btn.style(primary_btn)
            } else {
                btn.style(secondary_btn)
            }
        };

        let header = row![
            text("Settings").size(18).font(bold_font()),
            Space::new().width(Length::Fill),
            button(text("닫기").size(FS_BODY))
                .on_press(Message::CloseSettings)
                .padding([4, 12])
                .style(secondary_btn),
        ]
        .align_y(Alignment::Center);

        let key_status = if self.has_key {
            text("OpenRouter 키: 저장됨").size(FS_SUBTITLE)
        } else {
            text("OpenRouter 키 미등록").size(FS_SUBTITLE)
        };

        let key_input = text_input("sk-or-v1-...", &self.key_input)
            .on_input(Message::KeyInputChanged)
            .on_submit(Message::SaveKey)
            .padding(10)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fill);

        let actions = row![
            button(text("저장").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy || self.key_input.trim().is_empty() {
                    None
                } else {
                    Some(Message::SaveKey)
                })
                .style(primary_btn),
            button(text("삭제").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy || !self.has_key {
                    None
                } else {
                    Some(Message::ClearKey)
                })
                .style(danger_btn),
        ]
        .spacing(8);

        let tabby_header = row![
            text("OpenAI 호환 endpoint")
                .size(FS_SUBTITLE)
                .font(semibold_font()),
            text("(선택)").size(FS_LABEL).font(semibold_font()),
        ]
        .spacing(SPACE_XS)
        .align_y(Alignment::Center);
        let label_input: Element<Message> = text_input(
            "라벨 — 모델 셀렉터에 [xLLM] / [Tabby] / [Local] 같이 표시",
            &self.openai_compat_label,
        )
        .on_input(Message::OpenAICompatLabelChanged)
        .padding(8)
        .size(FS_BODY)
        .style(field_input)
        .width(Length::Fill)
        .into();
        let tabby_url = text_input(
            "예: http://localhost:9000 (xLLM) 또는 http://localhost:8080 (Tabby)",
            &self.tabby_url_input,
        )
        .on_input(Message::TabbyUrlChanged)
        .padding(10)
        .size(FS_BODY)
        .style(field_input)
        .width(Length::Fill);
        let tabby_token_toggle: Element<Message> = button(
            text(if self.show_tabby_token {
                "토큰 숨기기"
            } else {
                "토큰 입력 (선택)"
            })
            .size(FS_LABEL),
        )
        .on_press(Message::ToggleTabbyTokenVisible)
        .padding([4, 10])
        .style(secondary_btn)
        .into();
        let tabby_token: Element<Message> = if self.show_tabby_token {
            text_input("token (인증 강제 시에만)", &self.tabby_token_input)
                .on_input(Message::TabbyTokenChanged)
                .padding(10)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fill)
                .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };
        let tabby_actions = row![
            button(text("저장").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy {
                    None
                } else {
                    Some(Message::SaveTabby)
                })
                .style(primary_btn),
            button(text("연결 테스트").size(FS_SUBTITLE))
                .on_press_maybe(if self.busy || self.tabby_url_input.trim().is_empty() {
                    None
                } else {
                    Some(Message::FetchTabbyModels)
                })
                .style(secondary_btn),
            button(text("삭제").size(FS_SUBTITLE))
                .on_press_maybe(
                    if self.busy
                        || (self.tabby_url_input.is_empty() && self.tabby_token_input.is_empty())
                    {
                        None
                    } else {
                        Some(Message::ClearTabby)
                    }
                )
                .style(danger_btn),
        ]
        .spacing(8);
        let tabby_status_label: Element<Message> = self.endpoint_indicator(FS_LABEL);
        let mut tabby_presets = column![text("Tabby 추천 프리셋 (클릭 시 즉시 다운로드)")
            .size(FS_LABEL)
            .font(semibold_font())]
        .spacing(4);
        for (i, p) in EXL2_PRESETS.iter().take(4).enumerate() {
            let downloaded_folder = downloaded_exl2_preset_folder(&self.model_dir_input, p);
            let is_downloaded = downloaded_folder.is_some();
            let label = if is_downloaded {
                format!("✓ {} · {} · 다운로드됨", p.label, p.vram)
            } else {
                format!("{} · {}", p.label, p.vram)
            };
            tabby_presets = tabby_presets.push(
                button(text(label).size(FS_LABEL))
                    .on_press_maybe(if self.hf_dl.is_none() {
                        if let Some(folder_name) = downloaded_folder.clone() {
                            Some(Message::SelectDownloadedModel(folder_name))
                        } else {
                            Some(Message::DownloadExl2Preset(i))
                        }
                    } else {
                        None
                    })
                    .padding([4, 10])
                    .width(Length::Fill)
                    .style(secondary_btn),
            );
        }
        tabby_presets = tabby_presets
            .push(text("저장 위치는 Models 탭의 다운로드 경로를 사용합니다.").size(FS_MICRO));

        let provider_intro = column![
            text("AI Provider").size(FS_SUBTITLE).font(bold_font()),
            text("최소 1개 이상의 provider를 설정하세요").size(FS_LABEL),
        ]
        .spacing(SPACE_XXS);

        let provider_section = column![
            provider_intro,
            container(
                column![
                    row![
                        text("OpenRouter (클라우드)")
                            .size(FS_SUBTITLE)
                            .font(semibold_font()),
                        text("(필수)").size(FS_LABEL).font(semibold_font()),
                    ]
                    .spacing(SPACE_XS)
                    .align_y(Alignment::Center),
                    key_status,
                    key_input,
                    actions,
                    text("1. https://openrouter.ai 가입").size(FS_LABEL),
                    text("2. /keys 에서 키 발급 후 붙여넣기").size(FS_LABEL),
                ]
                .spacing(8),
            )
            .padding([12, 14])
            .style(panel_style),
            container(
                column![
                    tabby_header,
                    label_input,
                    tabby_url,
                    tabby_token_toggle,
                    tabby_token,
                    tabby_actions,
                    tabby_status_label,
                    Space::new().height(Length::Fixed(6.0)),
                    tabby_presets,
                ]
                .spacing(8),
            )
            .padding([12, 14])
            .style(panel_style),
        ]
        .spacing(10);

        let active_section: Element<Message> = match self.ui.settings_tab {
            SettingsTab::Provider => container(provider_section)
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match provider_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            SettingsTab::Runtime => container(self.view_inference_runner())
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match runtime_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            SettingsTab::Models => container(self.view_model_manager())
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match model_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
            SettingsTab::Mcp => container(self.view_mcp_settings())
                .padding([4, 4])
                .width(Length::Fill)
                .style(move |theme: &Theme| {
                    let p = theme.extended_palette();
                    let accent = match mcp_health {
                        TabHealth::Good => p.success.base.color,
                        TabHealth::Warn => p.primary.base.color,
                        TabHealth::Bad => p.danger.base.color,
                    };
                    container::Style {
                        background: Some(
                            iced::Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into(),
                        ),
                        border: iced::Border {
                            color: accent,
                            width: 1.0,
                            radius: 12.0.into(),
                        },
                        ..Default::default()
                    }
                })
                .into(),
        };

        let tabs = row![
            tab_btn(
                "◎",
                "Provider",
                if self.has_key || !self.tabby_url_input.trim().is_empty() {
                    "configured".to_string()
                } else {
                    "not set".to_string()
                },
                provider_health,
                SettingsTab::Provider
            ),
            tab_btn(
                "▶",
                "Runtime",
                if self.inference_pid.is_some() {
                    "running".to_string()
                } else {
                    "stopped".to_string()
                },
                runtime_health,
                SettingsTab::Runtime
            ),
            tab_btn(
                "□",
                "Models",
                format!("{local_model_count} local"),
                model_health,
                SettingsTab::Models
            ),
            tab_btn(
                "◇",
                "MCP",
                format!(
                    "{} srv / {} tools",
                    self.mcp_servers.len(),
                    self.mcp_tools.len()
                ),
                mcp_health,
                SettingsTab::Mcp
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

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
        .size(FS_LABEL),];

        let runtime_can_start = self.can_attempt_start_inference();

        let (active_tab_title, active_health, active_action, quick_label, quick_action) = match self
            .ui
            .settings_tab
        {
            SettingsTab::Provider => (
                "Provider",
                provider_health,
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
            SettingsTab::Runtime => (
                "Runtime",
                runtime_health,
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
            ),
            SettingsTab::Models => (
                "Models",
                model_health,
                if local_model_count > 0 {
                    "권장 액션: 다운로드된 모델을 Runtime 탭에서 선택해 실행해 보세요.".to_string()
                } else {
                    "권장 액션: 추천 프리셋에서 1개를 선택해 먼저 다운로드해 주세요.".to_string()
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
                mcp_health,
                if self.mcp_servers.is_empty() {
                    "권장 액션: 서버 이름과 명령을 입력해 MCP 서버를 하나 추가해 주세요."
                        .to_string()
                } else if self.mcp_tools.is_empty() {
                    "권장 액션: 서버 명령이 유효한지 확인하고 tools 로드를 기다려 주세요."
                        .to_string()
                } else {
                    "권장 액션: 채팅에서 MCP 도구 호출이 정상 동작하는지 점검해 주세요.".to_string()
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

        // 스크롤바가 콘텐츠를 덮지 않도록 우측 gutter를 확보한다.
        let scroll_body = container(
            column![
                Space::new().height(Length::Fixed(8.0)),
                tabs,
                summary,
                Space::new().height(Length::Fixed(8.0)),
                active_header,
                active_action_hint,
                active_section,
            ]
            .spacing(10)
            .max_width(560),
        )
        .padding([0, 14])
        .width(Length::Fill);

        let body = column![
            header,
            scrollable(scroll_body)
                .direction(Direction::Vertical(app_vscrollbar()))
                .height(Length::Fill)
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::Fixed(620.0))
        .spacing(8);

        container(body)
            .padding(20)
            .width(Length::Shrink)
            .height(Length::Fill)
            .into()
    }
}
