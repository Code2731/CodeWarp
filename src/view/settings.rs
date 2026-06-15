use super::ui::*;
use crate::*;
use iced::widget::scrollable::Direction;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Alignment, Element, Font, Length, Theme};

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

    /// inference 서버 (xLLM/vLLM/llama-server/TabbyML/TabbyAPI/Ollama/Custom) — dropdown 기반.
    /// CodeWarp가 child process로 spawn 관리.
    fn view_inference_runner(&self) -> Element<'_, Message> {
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

        // 바이너리 경로 — 비어있으면 PATH default, 채워져 있으면 절대 경로 사용
        let binary_section: Element<Message> = if matches!(
            self.inference_engine,
            InferenceEngine::Ollama | InferenceEngine::Custom
        ) {
            // Ollama는 spawn 안 함, Custom은 명령에 포함 → 별도 binary 입력 불필요
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

        // 엔진별 모델 입력 분기
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

        // 포트 (Ollama는 항상 11434, Custom은 명령에 포함되므로 hide)
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

        // 로그 마지막 N줄 (있을 때만)
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

    fn view_mcp_settings(&self) -> Element<'_, Message> {
        let header = text("MCP 서버 (Model Context Protocol)")
            .size(14)
            .font(semibold_font());
        let hint = text("stdio MCP 서버를 등록해 AI tool을 동적으로 확장합니다.").size(FS_LABEL);

        // 등록된 서버 목록
        let mut server_list = column![].spacing(4);
        for (i, s) in self.mcp_servers.iter().enumerate() {
            let tool_count = self
                .mcp_tools
                .iter()
                .filter(|t| t.server_name == s.name)
                .count();
            let label = format!("{} — {} (tool {}개)", s.name, s.command, tool_count);
            server_list = server_list.push(
                row![
                    text(shorten_tail(&label, 72))
                        .size(FS_BODY)
                        .width(Length::Fill),
                    button(text("✕").size(FS_LABEL))
                        .on_press(Message::RemoveMcpServer(i))
                        .padding([2, 6])
                        .style(danger_btn),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        // 추가 입력 행
        let add_row = row![
            text_input("서버 이름 (예: filesystem)", &self.mcp_input.name_input)
                .on_input(Message::McpNameChanged)
                .padding(6)
                .size(FS_BODY)
                .style(field_input)
                .width(Length::Fixed(140.0)),
            text_input(
                "명령 (예: npx -y @modelcontextprotocol/server-filesystem /tmp)",
                &self.mcp_input.command_input
            )
            .on_input(Message::McpCommandChanged)
            .on_submit(Message::AddMcpServer)
            .padding(6)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fill),
            button(text("추가").size(FS_BODY))
                .on_press(Message::AddMcpServer)
                .padding([6, 12])
                .style(primary_btn),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        let empty_state: Element<Message> = if self.mcp_servers.is_empty() {
            container(text("등록된 MCP 서버가 없습니다. 먼저 서버를 추가해 주세요.").size(FS_LABEL))
                .padding([8, 10])
                .style(panel_style)
                .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        container(column![header, hint, empty_state, server_list, add_row].spacing(8))
            .padding([14, 16])
            .width(Length::Fill)
            .style(panel_style)
            .into()
    }

    fn view_model_manager(&self) -> Element<'_, Message> {
        let header = text("모델 매니저 (HuggingFace 다운로드)")
            .size(14)
            .font(semibold_font());
        let local_model_count =
            list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len();
        let local_state: Element<Message> = if local_model_count == 0 {
            container(
                text("로컬 모델이 비어 있습니다. 프리셋 1개를 먼저 내려받으세요.").size(FS_LABEL),
            )
            .padding([8, 10])
            .style(panel_style)
            .into()
        } else {
            container(
                text(format!(
                    "로컬 모델 {}개가 준비되어 있습니다.",
                    local_model_count
                ))
                .size(FS_LABEL),
            )
            .padding([8, 10])
            .style(panel_style)
            .into()
        };

        // 다운로드 경로 — picker 버튼을 명확하게
        let dir_input = text_input("예: C:\\models 또는 ~/models", &self.model_dir_input)
            .on_input(Message::ModelDirChanged)
            .padding(8)
            .size(FS_BODY)
            .style(field_input)
            .width(Length::Fixed(360.0));
        let dir_row = row![
            dir_input,
            button(text("📁 찾아보기").size(FS_LABEL))
                .on_press(Message::PickModelDir)
                .padding([6, 12])
                .style(secondary_btn),
        ]
        .spacing(6)
        .align_y(Alignment::Center);

        // HF 토큰 toggle
        let token_toggle = button(
            text(if self.show_hf_token {
                "토큰 숨기기"
            } else {
                "토큰 입력"
            })
            .size(FS_LABEL),
        )
        .on_press(Message::ToggleHfTokenVisible)
        .padding([4, 10])
        .style(secondary_btn);
        let token_section: Element<Message> = if self.show_hf_token {
            row![
                text_input("hf_xxx... (gated repo용, 선택)", &self.hf_token_input)
                    .on_input(Message::HfTokenChanged)
                    .on_submit(Message::SaveHfToken)
                    .padding(8)
                    .size(FS_BODY)
                    .style(field_input)
                    .width(Length::Fixed(360.0)),
                button(text("저장").size(FS_LABEL))
                    .on_press(Message::SaveHfToken)
                    .padding([4, 10])
                    .style(primary_btn),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        // 추천 프리셋 — 카드를 두드러지게 (가장 많이 쓰이는 진입점)
        let mut presets_col =
            column![text("추천 프리셋 (클릭 → 입력란에 채움)").size(12)].spacing(4);
        for (i, p) in MODEL_PRESETS.iter().enumerate() {
            presets_col = presets_col.push(
                button(
                    column![
                        text(p.label).size(FS_SUBTITLE).font(semibold_font()),
                        text(p.note).size(FS_MICRO),
                        text(p.repo_id)
                            .size(FS_MICRO)
                            .font(Font::with_name("JetBrains Mono")),
                    ]
                    .spacing(2),
                )
                .on_press(Message::UsePreset(i))
                .padding([6, 12])
                .width(Length::Fill)
                .style(secondary_btn),
            );
        }

        // repo 입력 + 다운로드 시작
        let repo_input = text_input(
            "HF repo (예: Qwen/Qwen2.5-Coder-7B-Instruct)",
            &self.hf_repo_input,
        )
        .on_input(Message::HfRepoChanged)
        .on_submit(Message::StartHfDownload)
        .padding(8)
        .size(FS_BODY)
        .style(field_input)
        .width(Length::Fixed(360.0));
        let action_btn: Element<Message> = if self.hf_dl.is_some() {
            button(text("취소").size(FS_LABEL))
                .on_press(Message::CancelHfDownload)
                .padding([4, 10])
                .style(danger_btn)
                .into()
        } else {
            button(text("다운로드").size(FS_LABEL))
                .on_press(Message::StartHfDownload)
                .padding([4, 10])
                .style(primary_btn)
                .into()
        };
        let dl_row = row![repo_input, action_btn]
            .spacing(6)
            .align_y(Alignment::Center);

        // 진행률
        let progress: Element<Message> = if let Some(dl) = &self.hf_dl {
            let pct_text = match dl.file_bytes_total {
                Some(t) if t > 0 => {
                    format!("{:.0}%", (dl.file_bytes_done as f64 / t as f64) * 100.0)
                }
                _ => fmt_bytes(dl.file_bytes_done).to_string(),
            };
            column![
                text(format!(
                    "[{}/{}] {}",
                    dl.file_idx + 1,
                    dl.total_files.max(1),
                    dl.file_name
                ))
                .size(FS_LABEL)
                .font(Font::with_name("JetBrains Mono")),
                text(pct_text).size(FS_LABEL),
            ]
            .spacing(2)
            .into()
        } else {
            Space::new().height(Length::Shrink).into()
        };

        // EXL2 프리셋 섹션 (TabbyAPI용, 클릭 → 즉시 다운로드)
        let is_downloading = self.hf_dl.is_some();
        let mut exl2_col =
            column![text("EXL2 프리셋 (TabbyAPI용) — 클릭하면 바로 다운로드").size(FS_BODY),]
                .spacing(4);
        for (i, p) in EXL2_PRESETS.iter().enumerate() {
            let downloaded_folder = downloaded_exl2_preset_folder(&self.model_dir_input, p);
            let is_downloaded = downloaded_folder.is_some();
            let title = if is_downloaded {
                format!("✓ {} · 다운로드됨", p.label)
            } else {
                p.label.to_string()
            };
            let btn = button(
                row![
                    column![
                        text(title).size(FS_SUBTITLE).font(semibold_font()),
                        text(p.note).size(FS_MICRO),
                        text(p.repo_id)
                            .size(FS_MICRO)
                            .font(Font::with_name("JetBrains Mono")),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    text(p.vram).size(FS_LABEL),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([6, 12]);
            exl2_col = exl2_col.push(if is_downloading {
                btn.style(secondary_btn)
            } else if let Some(folder_name) = downloaded_folder {
                btn.on_press(Message::SelectDownloadedModel(folder_name))
                    .style(primary_btn)
            } else {
                btn.on_press(Message::DownloadExl2Preset(i))
                    .style(secondary_btn)
            });
        }

        container(
            column![
                header,
                local_state,
                text("HuggingFace에서 모델 받아 디스크에 저장.").size(FS_LABEL),
                Space::new().height(Length::Fixed(4.0)),
                text("저장 경로 (변경 가능)")
                    .size(FS_LABEL)
                    .font(semibold_font()),
                dir_row,
                Space::new().height(Length::Fixed(12.0)),
                exl2_col,
                Space::new().height(Length::Fixed(12.0)),
                text("HF 일반 모델 (safetensors · xLLM/vLLM용) — 클릭 → 입력란에 채움")
                    .size(FS_BODY),
                presets_col,
                Space::new().height(Length::Fixed(8.0)),
                text("또는 직접 입력").size(FS_LABEL).font(semibold_font()),
                dl_row,
                progress,
                Space::new().height(Length::Fixed(12.0)),
                // gated repo (Llama 등) 받을 때만 필요
                token_toggle,
                token_section,
            ]
            .spacing(6),
        )
        .padding([14, 16])
        .width(Length::Fill)
        .style(panel_style)
        .into()
    }
}
