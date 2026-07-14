use super::{App, Message, Task, session};
use session::{ThemeConfig, theme_presets};

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

enum CustomThemeApplication {
    Apply(ThemeConfig),
    Reject(String),
}

impl CustomThemeApplication {
    #[cfg(test)]
    fn persisted_theme(&self) -> Option<&ThemeConfig> {
        match self {
            Self::Apply(config) => Some(config),
            Self::Reject(_) => None,
        }
    }
}

fn custom_theme_application(
    active_theme: &ThemeConfig,
    hex_inputs: &[String],
) -> CustomThemeApplication {
    let mut candidate = active_theme.clone();
    for (idx, field) in THEME_FIELDS.iter().enumerate() {
        let hex = hex_inputs.get(idx).map_or("", String::as_str);
        if let Err(error) = candidate.update_hex(field, hex) {
            return CustomThemeApplication::Reject(format!("{field}: {error}"));
        }
    }

    match candidate.normal_text_contrast_violation() {
        Some(violation) => CustomThemeApplication::Reject(violation.korean_message()),
        None => CustomThemeApplication::Apply(candidate),
    }
}

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
        match custom_theme_application(&self.theme_config, &self.ui.theme_hex_inputs) {
            CustomThemeApplication::Apply(config) => {
                self.theme_config = config;
                self.theme_apply_msg = "테마가 적용되었습니다".to_string();
                self.ui.sync_theme_inputs(&self.theme_config);
                let cfg = self.theme_config.clone();
                Task::perform(
                    async move { session::write_theme(&cfg) },
                    Message::ThemeSaved,
                )
            }
            CustomThemeApplication::Reject(message) => {
                self.theme_apply_msg = message;
                Task::none()
            }
        }
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

    pub(crate) fn apply_theme_preset(&mut self, idx: usize) -> Task<Message> {
        let presets = theme_presets();
        if let Some(preset) = presets.get(idx) {
            self.theme_config = preset.config.clone();
            self.ui.sync_theme_inputs(&self.theme_config);
            self.theme_apply_msg = format!("프리셋 적용됨: {}", preset.name);
            let cfg = self.theme_config.clone();
            Task::perform(
                async move { session::write_theme(&cfg) },
                Message::ThemeSaved,
            )
        } else {
            Task::none()
        }
    }

    pub(crate) fn on_theme_saved(&mut self, _result: Result<(), String>) -> Task<Message> {
        Task::none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_custom_theme_is_rejected_without_mutating_or_persisting() {
        let active_theme = session::ThemeConfig::default_dark();
        let persisted_theme = active_theme.clone();
        let mut inputs = THEME_FIELDS
            .iter()
            .map(|field| active_theme.hex(field))
            .collect::<Vec<_>>();
        inputs[5] = "#030712".to_string();

        let outcome = custom_theme_application(&active_theme, &inputs);

        assert!(matches!(outcome, CustomThemeApplication::Reject(_)));
        assert_eq!(active_theme, persisted_theme);
        assert!(outcome.persisted_theme().is_none());
    }

    #[test]
    fn invalid_custom_theme_rejection_explains_how_to_meet_wcag_aa() {
        let active_theme = session::ThemeConfig::default_dark();
        let mut inputs = THEME_FIELDS
            .iter()
            .map(|field| active_theme.hex(field))
            .collect::<Vec<_>>();
        inputs[5] = "#030712".to_string();

        let outcome = custom_theme_application(&active_theme, &inputs);

        let CustomThemeApplication::Reject(message) = outcome else {
            panic!("invalid custom theme must be rejected");
        };
        assert!(message.contains("danger"));
        assert!(message.contains("4.5:1"));
        assert!(message.contains("밝거나 어두운"));
    }

    #[test]
    fn rejected_custom_apply_keeps_active_theme_and_entered_hex_values() {
        let (mut app, _) = App::new();
        let active_theme = session::ThemeConfig::default_dark();
        app.theme_config = active_theme.clone();
        app.ui.sync_theme_inputs(&active_theme);
        app.ui.theme_hex_inputs[5] = "#030712".to_string();
        let entered_inputs = app.ui.theme_hex_inputs.clone();

        let _ = app.apply_theme();

        assert_eq!(app.theme_config, active_theme);
        assert_eq!(app.ui.theme_hex_inputs, entered_inputs);
        assert!(app.theme_apply_msg.contains("WCAG AA 4.5:1"));
    }
}
