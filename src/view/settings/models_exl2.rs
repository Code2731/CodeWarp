use crate::view::ui::{
    primary_btn, secondary_btn, semibold_font, FS_BODY, FS_LABEL, FS_MICRO, FS_SUBTITLE,
};
use crate::{downloaded_exl2_preset_folder, App, Message, EXL2_PRESETS};
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Font, Length};

impl App {
    pub(crate) fn view_exl2_presets(&self) -> Element<'_, Message> {
        let is_downloading = self.hf_dl.is_some();
        let mut col =
            column![text("EXL2 프리셋 (TabbyAPI용) — 클릭하면 바로 다운로드").size(FS_BODY)]
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
            col = col.push(if is_downloading {
                btn.style(secondary_btn)
            } else if let Some(folder_name) = downloaded_folder {
                btn.on_press(Message::SelectDownloadedModel(folder_name))
                    .style(primary_btn)
            } else {
                btn.on_press(Message::DownloadExl2Preset(i))
                    .style(secondary_btn)
            });
        }
        col.into()
    }
}
