use crate::view::ui::*;
use crate::*;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Length};

impl App {
    pub(crate) fn view_provider_tab(&self) -> Element<'_, Message> {
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

        let body = column![
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

        container(body)
            .padding([14, 16])
            .width(Length::Fill)
            .style(panel_style)
            .into()
    }
}
