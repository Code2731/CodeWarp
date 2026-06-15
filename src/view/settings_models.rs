use super::ui::*;
use crate::*;
use iced::widget::{button, column, container, row, text, text_input, Space};
use iced::{Alignment, Element, Font, Length};

impl App {
    pub(crate) fn view_model_manager(&self) -> Element<'_, Message> {
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
