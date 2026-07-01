use super::ui::{
    FS_LABEL, STATUSBAR_PAD_X, STATUSBAR_PAD_Y, STATUSBAR_ROW_SPACING, semibold_font, topbar_style,
};
use crate::{App, Message};
use iced::widget::{Space, container, row, text};
use iced::{Alignment, Color, Element, Font, Length, Theme};

impl App {
    pub(super) fn view_statusbar(&self) -> Element<'_, Message> {
        let model_label = self
            .selected_model
            .clone()
            .unwrap_or_else(|| "(없음)".into());
        let credit_label = match &self.account {
            Some(a) => match (a.usage, a.limit) {
                (Some(u), Some(l)) => format!("잔액: ${:.2} / ${:.2}", (l - u).max(0.0), l),
                (Some(u), None) => format!("사용: ${u:.4}"),
                _ => "잔액: -".into(),
            },
            None => "잔액: -".into(),
        };
        let last_cost_label = match self.last_response_cost {
            Some(c) if c > 0.0 => format!("최근: ${c:.4}"),
            _ => String::new(),
        };
        let time_label = match self.last_response_time_ms {
            Some(ms) if ms > 0 => {
                if ms < 1000 {
                    format!("{ms}ms")
                } else {
                    format!("{:.1}s", ms as f64 / 1000.0)
                }
            }
            _ => String::new(),
        };
        let streaming_indicator: Element<Message> = if self.streaming_block_id.is_some() {
            container(
                text("▶ 응답 생성 중...")
                    .size(FS_LABEL)
                    .font(semibold_font())
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.extended_palette().primary.base.text),
                    }),
            )
            .padding([2, 8])
            .style(|theme: &Theme| {
                let p = theme.extended_palette();
                container::Style {
                    background: Some(
                        Color::from_rgba(
                            p.primary.base.color.r,
                            p.primary.base.color.g,
                            p.primary.base.color.b,
                            0.15,
                        )
                        .into(),
                    ),
                    border: iced::Border {
                        color: Color::from_rgba(
                            p.primary.base.color.r,
                            p.primary.base.color.g,
                            p.primary.base.color.b,
                            0.30,
                        ),
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
        } else {
            Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        };
        let status_text: Element<Message> = if self.status.starts_with("[WARN]") {
            text(&self.status)
                .size(FS_LABEL)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().warning.base.color),
                })
                .into()
        } else if self.status.starts_with("[ERROR]") {
            text(&self.status)
                .size(FS_LABEL)
                .style(|theme: &Theme| iced::widget::text::Style {
                    color: Some(theme.extended_palette().danger.base.color),
                })
                .into()
        } else {
            text(&self.status).size(FS_LABEL).into()
        };
        let mut bar = row![
            streaming_indicator,
            status_text,
            Space::new().width(Length::Fill),
        ]
        .spacing(STATUSBAR_ROW_SPACING)
        .align_y(Alignment::Center);
        if !last_cost_label.is_empty() {
            bar = bar.push(
                text(last_cost_label)
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono")),
            );
        }
        if !time_label.is_empty() {
            bar = bar.push(
                text(time_label)
                    .size(FS_LABEL)
                    .font(Font::with_name("JetBrains Mono")),
            );
        }
        bar = bar
            .push(text(credit_label).size(FS_LABEL))
            .push(text(format!("모델: {model_label}")).size(FS_LABEL))
            .push(
                text(if self.has_key {
                    "키: 등록됨"
                } else {
                    "키: 미등록"
                })
                .size(FS_LABEL),
            )
            .push(self.endpoint_indicator(FS_LABEL));

        container(bar)
            .padding([STATUSBAR_PAD_Y, STATUSBAR_PAD_X])
            .style(topbar_style)
            .width(Length::Fill)
            .into()
    }
}
