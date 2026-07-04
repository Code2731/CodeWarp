use super::TabHealth;

use crate::view::ui::{FS_LABEL, FS_MICRO, primary_btn, secondary_btn, semibold_font};
use crate::{App, Message, SettingsTab};
use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Alignment, Color, Element, Length, Theme};

impl App {
    pub(crate) fn view_settings_tab_bar(
        &self,
        provider_health: TabHealth,
        runtime_health: TabHealth,
        model_health: TabHealth,
        mcp_health: TabHealth,
        local_model_count: usize,
    ) -> Element<'_, Message> {
        let tab_btn = |icon: &'static str,
                       label: &'static str,
                       note: String,
                       health: TabHealth,
                       tab: SettingsTab|
         -> Element<'_, Message> {
            let dot = text("●").size(FS_MICRO).style(move |theme: &Theme| {
                let p = theme.extended_palette();
                let color = match health {
                    TabHealth::Good => p.success.base.color,
                    TabHealth::Warn => p.primary.base.color,
                    TabHealth::Bad => p.danger.base.color,
                };
                iced::widget::text::Style { color: Some(color) }
            });
            let btn: Element<Message> = {
                let b = button(
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
                    b.style(primary_btn)
                } else {
                    b.style(secondary_btn)
                }
                .into()
            };
            let is_hovered = self.hovered_settings_tab == Some(tab);
            container(
                mouse_area(btn)
                    .on_enter(Message::SettingsTabHovered(Some(tab)))
                    .on_exit(Message::SettingsTabHovered(None)),
            )
            .style(move |theme: &Theme| {
                if is_hovered && self.ui.settings_tab != tab {
                    let p = theme.extended_palette();
                    container::Style {
                        background: Some(
                            Color::from_rgba(
                                p.primary.base.color.r,
                                p.primary.base.color.g,
                                p.primary.base.color.b,
                                0.06,
                            )
                            .into(),
                        ),
                        border: iced::Border {
                            color: Color::from_rgba(
                                p.primary.base.color.r,
                                p.primary.base.color.g,
                                p.primary.base.color.b,
                                0.35,
                            ),
                            width: 0.0,
                            radius: 10.0.into(),
                        },
                        ..Default::default()
                    }
                } else {
                    container::Style::default()
                }
            })
            .into()
        };

        row![
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
            tab_btn(
                "🎨",
                "Theme",
                "custom".to_string(),
                TabHealth::Good,
                SettingsTab::Theme
            ),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }
}
