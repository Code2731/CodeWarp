use super::settings_health::TabHealth;
use super::ui::*;
use crate::*;
use iced::widget::{row, text};
use iced::Element;

impl App {
    pub(crate) fn view_settings_status_panel(
        &self,
        active_health: TabHealth,
        local_model_count: usize,
    ) -> (Element<'_, Message>, Element<'_, Message>) {
        let summary = row![text(format!(
            "Providers: {}  •  Runtime: {}  •  Models: {}  •  MCP: {}",
            if self.has_key || !self.tabby_url_input.trim().is_empty() {
                "configured"
            } else {
                "empty"
            },
            if self.inference_pid.is_some() {
                "running"
            } else {
                "stopped"
            },
            local_model_count,
            self.mcp_servers.len()
        ))
        .size(FS_LABEL)];

        let (active_tab_title, active_action, quick_label, quick_msg) =
            self.settings_tab_data(local_model_count);

        let hint = self.view_settings_active_action_hint(
            active_health,
            active_tab_title,
            active_action,
            quick_label,
            quick_msg,
        );

        (summary.into(), hint)
    }
}
