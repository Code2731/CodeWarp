use crate::view::ui::*;
use crate::*;
use iced::widget::{button, column, text};
use iced::{Element, Font, Length};

impl App {
    pub(crate) fn view_model_presets(&self) -> Element<'_, Message> {
        let mut col = column![text("추천 프리셋 (클릭 → 입력란에 채움)").size(12)].spacing(4);
        for (i, p) in MODEL_PRESETS.iter().enumerate() {
            col = col.push(
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
        col.into()
    }
}
