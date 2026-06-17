pub(crate) use super::settings_health_tab::TabHealth;

use super::ui::*;
use crate::*;
use iced::widget::{button, column, row, text};
use iced::{Alignment, Element, Length, Theme};

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
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    }
}
