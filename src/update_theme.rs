use super::{App, Message, Task, session};

const THEME_FIELDS: &[&str] = &[
    "background",
    "text",
    "primary",
    "success",
    "warning",
    "danger",
    "accent_user",
    "accent_assistant",
    "accent_error",
];

impl App {
    pub(crate) fn on_theme_hex_changed(&mut self, field: String, value: String) {
        if let Some(idx) = THEME_FIELDS.iter().position(|f| *f == field) {
            while self.ui.theme_hex_inputs.len() <= idx {
                self.ui.theme_hex_inputs.push(String::new());
            }
            self.ui.theme_hex_inputs[idx] = value;
        }
    }

    pub(crate) fn apply_theme(&mut self) -> Task<Message> {
        for (idx, field) in THEME_FIELDS.iter().enumerate() {
            let hex = self
                .ui
                .theme_hex_inputs
                .get(idx)
                .cloned()
                .unwrap_or_default();
            if let Err(e) = self.theme_config.update_hex(field, &hex) {
                self.theme_apply_msg = format!("{field}: {e}");
                return Task::none();
            }
        }
        self.theme_apply_msg = "테마가 적용되었습니다".to_string();
        self.ui.sync_theme_inputs(&self.theme_config);
        let cfg = self.theme_config.clone();
        Task::perform(
            async move { session::write_theme(&cfg) },
            Message::ThemeSaved,
        )
    }

    pub(crate) fn reset_theme(&mut self) -> Task<Message> {
        self.theme_config = session::ThemeConfig::default_dark();
        self.theme_apply_msg = "기본 테마로 리셋되었습니다".to_string();
        self.ui.sync_theme_inputs(&self.theme_config);
        let cfg = self.theme_config.clone();
        Task::perform(
            async move { session::write_theme(&cfg) },
            Message::ThemeSaved,
        )
    }

    pub(crate) fn on_theme_saved(&mut self, _result: Result<(), String>) -> Task<Message> {
        Task::none()
    }
}
