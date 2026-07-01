use super::settings::TabHealth;
use super::ui::{FS_BODY, app_vscrollbar, bold_font, dark_scrollable, divider, secondary_btn};
use crate::{App, Message, SettingsTab, list_downloaded_models};
use iced::widget::scrollable::Direction;
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Color, Element, Length, Theme};

fn active_section_box<'a>(
    content: Element<'a, Message>,
    health: TabHealth,
) -> Element<'a, Message> {
    container(content)
        .padding([4, 4])
        .width(Length::Fill)
        .style(move |theme: &Theme| {
            let p = theme.extended_palette();
            let accent = match health {
                TabHealth::Good => p.success.base.color,
                TabHealth::Warn => p.primary.base.color,
                TabHealth::Bad => p.danger.base.color,
            };
            container::Style {
                background: Some(Color::from_rgba(accent.r, accent.g, accent.b, 0.06).into()),
                border: iced::Border {
                    color: accent,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            }
        })
        .into()
}

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
            SettingsTab::Theme => TabHealth::Good,
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
            SettingsTab::Provider => active_section_box(self.view_provider_tab(), provider_health),
            SettingsTab::Runtime => {
                active_section_box(self.view_inference_runner(), runtime_health)
            }
            SettingsTab::Models => active_section_box(self.view_model_manager(), model_health),
            SettingsTab::Mcp => active_section_box(self.view_mcp_settings(), mcp_health),
            SettingsTab::Theme => container(self.view_theme_tab())
                .padding([4, 4])
                .width(Length::Fill)
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
                divider(),
                Space::new().height(Length::Fixed(4.0)),
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
                .style(dark_scrollable)
                .height(Length::Fill)
                .width(Length::Fill),
        ]
        .height(Length::Fill)
        .width(Length::FillPortion(3))
        .max_width(660.0)
        .spacing(8);

        container(body)
            .padding(20)
            .width(Length::Shrink)
            .height(Length::Fill)
            .into()
    }
}
