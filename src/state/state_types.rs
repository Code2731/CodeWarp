use super::{HashSet, SettingsTab, SortMode, session};
use crate::ModelCategory;

#[derive(Debug)]
pub(crate) struct HfDownload {
    pub(crate) folder_name: String,
    pub(crate) total_files: usize,
    pub(crate) file_idx: usize,
    pub(crate) file_name: String,
    pub(crate) file_bytes_done: u64,
    pub(crate) file_bytes_total: Option<u64>,
}

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct UiState {
    pub(crate) show_settings: bool,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) show_command_palette: bool,
    pub(crate) command_palette_input: String,
    pub(crate) active_palette_idx: Option<usize>,
    pub(crate) pending_delete_session: Option<u64>,
    pub(crate) expanded_confirm_idx: Option<usize>,
    pub(crate) collapsed_blocks: std::collections::HashSet<u64>,
    pub(crate) theme_hex_inputs: Vec<String>,
    pub(crate) renaming_session_id: Option<u64>,
    pub(crate) rename_input: String,
    pub(crate) session_search: String,
    pub(crate) show_shortcut_guide: bool,
    pub(crate) show_tabby_token: bool,
    pub(crate) compare_both: bool,
    pub(crate) compare_pending: bool,
    pub(crate) follow_bottom: bool,
    pub(crate) show_write_confirm: bool,
    pub(crate) show_mention: bool,
    pub(crate) pty_visible: bool,
}

impl UiState {
    pub(crate) fn new(show_settings: bool, show_tabby_token: bool) -> Self {
        let default_theme = session::ThemeConfig::default_dark();
        Self {
            show_settings,
            settings_tab: SettingsTab::Provider,
            show_command_palette: false,
            command_palette_input: String::new(),
            active_palette_idx: None,
            pending_delete_session: None,
            expanded_confirm_idx: None,
            collapsed_blocks: std::collections::HashSet::new(),
            theme_hex_inputs: Self::theme_hex_vec(&default_theme),
            renaming_session_id: None,
            rename_input: String::new(),
            session_search: String::new(),
            show_shortcut_guide: false,
            show_tabby_token,
            compare_both: false,
            compare_pending: false,
            follow_bottom: true,
            show_write_confirm: false,
            show_mention: false,
            pty_visible: false,
        }
    }

    fn theme_hex_vec(cfg: &session::ThemeConfig) -> Vec<String> {
        vec![
            cfg.hex("background"),
            cfg.hex("text"),
            cfg.hex("primary"),
            cfg.hex("success"),
            cfg.hex("warning"),
            cfg.hex("danger"),
            cfg.hex("accent_user"),
            cfg.hex("accent_assistant"),
            cfg.hex("accent_error"),
        ]
    }

    pub(crate) fn sync_theme_inputs(&mut self, cfg: &session::ThemeConfig) {
        self.theme_hex_inputs = Self::theme_hex_vec(cfg);
    }
}

#[derive(Debug)]
pub(crate) struct ModelFilterState {
    pub(crate) filter_categories: HashSet<ModelCategory>,
    pub(crate) filter_favorites_only: bool,
    pub(crate) favorites: HashSet<String>,
    pub(crate) sort_mode: SortMode,
}

impl ModelFilterState {
    pub(crate) fn new() -> Self {
        Self {
            filter_categories: HashSet::from([
                ModelCategory::Coding,
                ModelCategory::Reasoning,
                ModelCategory::General,
            ]),
            filter_favorites_only: false,
            favorites: session::read_favorites().into_iter().collect(),
            sort_mode: SortMode::Default,
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct McpInputState {
    pub(crate) name_input: String,
    pub(crate) command_input: String,
}
