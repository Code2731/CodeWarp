use super::settings::TabHealth;
use super::ui::{app_vscrollbar, bold_font, secondary_btn, FS_BODY};
use crate::{list_downloaded_models, App, Message, SettingsTab};
use iced::widget::scrollable::Direction;
use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{Alignment, Element, Length, Theme};

impl App {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn view_settings(&self) -> Element<'_, Message> {
        let provider_health = self.compute_provider_health();
        let runtime_health = self.compute_runtime_health();
        let model_health = self.compute_model_health();
        let mcp_health = self.compute_mcp_health();
        let local_model_count =
            list_downloaded_models(std::path::Path::new(&self.model_dir_input)).len();
        let active_health = match self.ui.settings_tab {
            SettingsTab::Provider => provider_health,
            SettingsTab::Runtime => runtime_health,
            SettingsTab::Models => model_health,
            SettingsTab::Mcp => mcp_health,
        };

        let header = row![
            text("Settings").size(18).font(bold_font()),
            Space::new().width(Length::Fill),
            button(text("?リ린").size(FS_BODY))
                .on_press(Message::CloseSettings)
                .padding([4, 12])
                .style(secondary_btn),
        ]
        .align_y(Alignment::Center);

        let active_section: Element<Message> = match self.ui.settings_tab {
            SettingsTab::Provider => container(self.view_provider_tab())
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

        let tabs = self.view_settings_tab_bar(
            provider_health,
            runtime_health,
            model_health,
            mcp_health,
            local_model_count,
        );
        let (summary, active_panel) =
            self.view_settings_status_panel(active_health, local_model_count);

        let scroll_body = container(
            column![
                Space::new().height(Length::Fixed(8.0)),
                tabs,
                summary,
                Space::new().height(Length::Fixed(8.0)),
                active_panel,
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
