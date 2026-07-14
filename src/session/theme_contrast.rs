use super::theme::ThemeConfig;

pub(crate) const NORMAL_TEXT_CONTRAST_MINIMUM: f64 = 4.5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NormalTextColor {
    Text,
    Primary,
    Success,
    Warning,
    Danger,
}

impl NormalTextColor {
    const ALL: [Self; 5] = [
        Self::Text,
        Self::Primary,
        Self::Success,
        Self::Warning,
        Self::Danger,
    ];

    fn field_name(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Primary => "primary",
            Self::Success => "success",
            Self::Warning => "warning",
            Self::Danger => "danger",
        }
    }

    fn color(self, config: &ThemeConfig) -> [u8; 3] {
        match self {
            Self::Text => config.text,
            Self::Primary => config.primary,
            Self::Success => config.success,
            Self::Warning => config.warning,
            Self::Danger => config.danger,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ContrastViolation {
    color: NormalTextColor,
    ratio: f64,
}

impl ContrastViolation {
    pub(crate) fn korean_message(self) -> String {
        format!(
            "{} 색상이 배경과 WCAG AA 4.5:1 대비를 만족하지 않습니다 (현재 {:.2}:1). 더 밝거나 어두운 HEX 색상을 입력하세요.",
            self.color.field_name(),
            self.ratio
        )
    }
}

pub(crate) fn normal_text_contrast_violation(config: &ThemeConfig) -> Option<ContrastViolation> {
    NormalTextColor::ALL.into_iter().find_map(|color| {
        let ratio = contrast_ratio(color.color(config), config.background);
        (!meets_normal_text_contrast(ratio)).then_some(ContrastViolation { color, ratio })
    })
}

fn srgb_channel_to_linear(channel: u8) -> f64 {
    let srgb = f64::from(channel) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

fn relative_luminance(color: [u8; 3]) -> f64 {
    0.2126 * srgb_channel_to_linear(color[0])
        + 0.7152 * srgb_channel_to_linear(color[1])
        + 0.0722 * srgb_channel_to_linear(color[2])
}

pub(crate) fn contrast_ratio(first: [u8; 3], second: [u8; 3]) -> f64 {
    let first_luminance = relative_luminance(first);
    let second_luminance = relative_luminance(second);
    let lighter = first_luminance.max(second_luminance);
    let darker = first_luminance.min(second_luminance);
    (lighter + 0.05) / (darker + 0.05)
}

fn meets_normal_text_contrast(ratio: f64) -> bool {
    ratio >= NORMAL_TEXT_CONTRAST_MINIMUM
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::theme_presets;

    const FLOAT_EPSILON: f64 = 1e-12;

    #[test]
    fn srgb_channel_to_linear_uses_wcag_transfer_function_edge_values() {
        let low_edge = srgb_channel_to_linear(10);
        let high_edge = srgb_channel_to_linear(11);

        assert!((low_edge - (10.0 / 255.0 / 12.92)).abs() < FLOAT_EPSILON);
        assert!(
            (high_edge - (((11.0_f64 / 255.0 + 0.055) / 1.055).powf(2.4))).abs() < FLOAT_EPSILON
        );
    }

    #[test]
    fn contrast_ratio_returns_wcag_black_and_white_extremes() {
        assert!((contrast_ratio([0, 0, 0], [255, 255, 255]) - 21.0).abs() < FLOAT_EPSILON);
        assert!((contrast_ratio([255, 255, 255], [0, 0, 0]) - 21.0).abs() < FLOAT_EPSILON);
    }

    #[test]
    fn normal_text_contrast_threshold_uses_the_unrounded_wcag_boundary() {
        assert!(meets_normal_text_contrast(NORMAL_TEXT_CONTRAST_MINIMUM));
        assert!(!meets_normal_text_contrast(
            NORMAL_TEXT_CONTRAST_MINIMUM - FLOAT_EPSILON
        ));
    }

    #[test]
    fn every_theme_preset_meets_normal_text_contrast_policy() {
        for preset in theme_presets() {
            assert!(
                normal_text_contrast_violation(&preset.config).is_none(),
                "{} preset failed normal-text contrast validation",
                preset.name
            );
        }
    }
}
