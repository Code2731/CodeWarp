use super::*;

pub(crate) struct HfDownload {
    pub(crate) folder_name: String,
    pub(crate) total_files: usize,
    pub(crate) file_idx: usize,
    pub(crate) file_name: String,
    pub(crate) file_bytes_done: u64,
    pub(crate) file_bytes_total: Option<u64>,
}

pub(crate) struct UiState {
    pub(crate) show_settings: bool,
    pub(crate) settings_tab: SettingsTab,
    pub(crate) show_command_palette: bool,
    pub(crate) command_palette_input: String,
    pub(crate) pending_delete_session: Option<u64>,
    pub(crate) expanded_confirm_idx: Option<usize>,
}

impl UiState {
    pub(crate) fn new(show_settings: bool) -> Self {
        Self {
            show_settings,
            settings_tab: SettingsTab::Provider,
            show_command_palette: false,
            command_palette_input: String::new(),
            pending_delete_session: None,
            expanded_confirm_idx: None,
        }
    }
}

pub(crate) struct ModelFilterState {
    pub(crate) filter_coding: bool,
    pub(crate) filter_reasoning: bool,
    pub(crate) filter_general: bool,
    pub(crate) filter_favorites_only: bool,
    pub(crate) favorites: HashSet<String>,
    pub(crate) sort_mode: SortMode,
}

impl ModelFilterState {
    pub(crate) fn new() -> Self {
        Self {
            filter_coding: true,
            filter_reasoning: true,
            filter_general: true,
            filter_favorites_only: false,
            favorites: session::read_favorites().into_iter().collect(),
            sort_mode: SortMode::Default,
        }
    }
}

#[derive(Default)]
pub(crate) struct McpInputState {
    pub(crate) name_input: String,
    pub(crate) command_input: String,
}
