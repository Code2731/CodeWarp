use iced::Color;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ThemeConfig {
    pub(crate) background: [u8; 3],
    pub(crate) text: [u8; 3],
    pub(crate) primary: [u8; 3],
    pub(crate) success: [u8; 3],
    pub(crate) warning: [u8; 3],
    pub(crate) danger: [u8; 3],
    pub(crate) accent_user: [u8; 3],
    pub(crate) accent_assistant: [u8; 3],
    pub(crate) accent_error: [u8; 3],
}

impl ThemeConfig {
    pub(crate) fn default_dark() -> Self {
        Self {
            background: [0x03, 0x07, 0x12],
            text: [0xe6, 0xec, 0xf8],
            primary: [0x0e, 0xa5, 0xe9],
            success: [0x10, 0xb9, 0x81],
            warning: [0xf5, 0x9e, 0x0b],
            danger: [0xf4, 0x71, 0x74],
            accent_user: [0x0e, 0xa5, 0xe9],
            accent_assistant: [0xa7, 0x8b, 0xfa],
            accent_error: [0xf0, 0x5b, 0x6f],
        }
    }

    pub(crate) fn to_palette(&self) -> iced::theme::Palette {
        iced::theme::Palette {
            background: Color::from_rgb8(
                self.background[0],
                self.background[1],
                self.background[2],
            ),
            text: Color::from_rgb8(self.text[0], self.text[1], self.text[2]),
            primary: Color::from_rgb8(self.primary[0], self.primary[1], self.primary[2]),
            success: Color::from_rgb8(self.success[0], self.success[1], self.success[2]),
            warning: Color::from_rgb8(self.warning[0], self.warning[1], self.warning[2]),
            danger: Color::from_rgb8(self.danger[0], self.danger[1], self.danger[2]),
        }
    }

    pub(crate) fn accent_user(&self) -> Color {
        Color::from_rgb8(
            self.accent_user[0],
            self.accent_user[1],
            self.accent_user[2],
        )
    }

    pub(crate) fn accent_assistant(&self) -> Color {
        Color::from_rgb8(
            self.accent_assistant[0],
            self.accent_assistant[1],
            self.accent_assistant[2],
        )
    }

    pub(crate) fn accent_error(&self) -> Color {
        Color::from_rgb8(
            self.accent_error[0],
            self.accent_error[1],
            self.accent_error[2],
        )
    }

    pub(crate) fn update_hex(&mut self, field: &str, hex: &str) -> Result<(), String> {
        let hex = hex.trim_start_matches('#');
        if hex.len() != 6 {
            return Err("6자리 HEX가 필요합니다".to_string());
        }
        let r = u8::from_str_radix(&hex[0..2], 16).map_err(|_| "잘못된 HEX".to_string())?;
        let g = u8::from_str_radix(&hex[2..4], 16).map_err(|_| "잘못된 HEX".to_string())?;
        let b = u8::from_str_radix(&hex[4..6], 16).map_err(|_| "잘못된 HEX".to_string())?;
        *self.field_mut(field) = [r, g, b];
        Ok(())
    }

    fn field_mut(&mut self, field: &str) -> &mut [u8; 3] {
        match field {
            "background" => &mut self.background,
            "text" => &mut self.text,
            "primary" => &mut self.primary,
            "success" => &mut self.success,
            "warning" => &mut self.warning,
            "danger" => &mut self.danger,
            "accent_user" => &mut self.accent_user,
            "accent_assistant" => &mut self.accent_assistant,
            "accent_error" => &mut self.accent_error,
            _ => panic!("unknown field: {field}"),
        }
    }

    pub(crate) fn hex(&self, field: &str) -> String {
        let c = self.field_val(field);
        format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
    }

    fn field_val(&self, field: &str) -> &[u8; 3] {
        match field {
            "background" => &self.background,
            "text" => &self.text,
            "primary" => &self.primary,
            "success" => &self.success,
            "warning" => &self.warning,
            "danger" => &self.danger,
            "accent_user" => &self.accent_user,
            "accent_assistant" => &self.accent_assistant,
            "accent_error" => &self.accent_error,
            _ => panic!("unknown field: {field}"),
        }
    }
}

fn theme_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("codewarp").join("theme.json"))
}

pub(crate) fn read_theme() -> ThemeConfig {
    let Some(path) = theme_path() else {
        return ThemeConfig::default_dark();
    };
    let Ok(json) = std::fs::read_to_string(&path) else {
        return ThemeConfig::default_dark();
    };
    serde_json::from_str(&json).unwrap_or_default()
}

pub(crate) fn write_theme(config: &ThemeConfig) -> Result<(), String> {
    let path = theme_path().ok_or("data_local_dir 없음".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::default_dark()
    }
}

pub(crate) struct ThemePreset {
    pub(crate) name: &'static str,
    pub(crate) config: ThemeConfig,
}

pub(crate) fn theme_presets() -> Vec<ThemePreset> {
    vec![
        ThemePreset {
            name: "Default Dark",
            config: ThemeConfig::default_dark(),
        },
        ThemePreset {
            name: "Nord",
            config: ThemeConfig {
                background: [0x2e, 0x34, 0x40],
                text: [0xec, 0xef, 0xf4],
                primary: [0x88, 0xc0, 0xd0],
                success: [0xa3, 0xbe, 0x8c],
                warning: [0xeb, 0xcb, 0x8b],
                danger: [0xbf, 0x61, 0x6a],
                accent_user: [0x88, 0xc0, 0xd0],
                accent_assistant: [0xb4, 0x8e, 0xad],
                accent_error: [0xbf, 0x61, 0x6a],
            },
        },
        ThemePreset {
            name: "Dracula",
            config: ThemeConfig {
                background: [0x28, 0x2a, 0x36],
                text: [0xf8, 0xf8, 0xf2],
                primary: [0xbd, 0x93, 0xf9],
                success: [0x50, 0xfa, 0x7b],
                warning: [0xf1, 0xfa, 0x8c],
                danger: [0xff, 0x55, 0x55],
                accent_user: [0xbd, 0x93, 0xf9],
                accent_assistant: [0xff, 0x79, 0xc6],
                accent_error: [0xff, 0x55, 0x55],
            },
        },
        ThemePreset {
            name: "Monokai",
            config: ThemeConfig {
                background: [0x27, 0x28, 0x22],
                text: [0xf8, 0xf8, 0xf2],
                primary: [0x66, 0xd9, 0xef],
                success: [0xa6, 0xe2, 0x2e],
                warning: [0xe6, 0xdb, 0x74],
                danger: [0xf9, 0x26, 0x72],
                accent_user: [0x66, 0xd9, 0xef],
                accent_assistant: [0xae, 0x81, 0xff],
                accent_error: [0xf9, 0x26, 0x72],
            },
        },
        ThemePreset {
            name: "Catppuccin",
            config: ThemeConfig {
                background: [0x1e, 0x1e, 0x2e],
                text: [0xcd, 0xd6, 0xf4],
                primary: [0x89, 0xb4, 0xfa],
                success: [0xa6, 0xe3, 0xa1],
                warning: [0xf9, 0xe2, 0xaf],
                danger: [0xf3, 0x8b, 0xa8],
                accent_user: [0x89, 0xb4, 0xfa],
                accent_assistant: [0xcb, 0xa6, 0xf7],
                accent_error: [0xf3, 0x8b, 0xa8],
            },
        },
    ]
}
