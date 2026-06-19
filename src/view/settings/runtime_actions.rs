use crate::view::ui::*;
use crate::*;
use iced::widget::{button, column, container, row, text, Space};
use iced::{Alignment, Element, Font, Length, Theme};

impl App {
    pub(crate) fn view_inference_actions(&self) -> Element<'_, Message> {
        let running = self.inference_pid.is_some();
        let can_start = self.can_attempt_start_inference();

        if running {
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
        }
    }

    pub(crate) fn view_inference_log_section(&self) -> Element<'_, Message> {
        if self.inference_log.is_empty() {
            return Space::new().height(Length::Shrink).into();
        }
        let mut col = column![text("로그 (최근)").size(FS_MICRO).font(semibold_font())].spacing(1);
        for line in &self.inference_log {
            col = col.push(
                text(line.clone())
                    .size(FS_MICRO)
                    .font(Font::with_name("JetBrains Mono")),
            );
        }
        container(col).padding([6, 10]).style(panel_style).into()
    }
}
