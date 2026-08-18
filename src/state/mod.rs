// state.rs — Core application state types (child module of main)
use super::{
    AgentMode, Arc, AuthKeyData, Block, ChatMessage, HashSet, InferenceEngine, LlmProvider,
    Message, ModelOption, OpenRouterModel, PathBuf, PendingToolCall, RuntimeStopHandle,
    SIDEBAR_WIDTH, ScrollId, SettingsTab, SortMode, TABBY_API_DEFAULT_PORT, Task, Theme, combo_box,
    default_tabbyapi_runtime_dir, find_tabbyapi_launcher, keystore, mcp, persisted_to_block, pty,
    session, task,
};
use crate::util::file_tree;
use iced::widget::text_editor;

mod state_new;
mod state_restore;
mod state_types;
pub(crate) use state_types::*;

#[derive(Debug)]
pub(crate) struct InactiveSession {
    pub(crate) id: u64,
    pub(crate) title: String,
    pub(crate) conversation: Arc<Vec<ChatMessage>>,
    pub(crate) blocks: Vec<session::PersistedBlock>,
    pub(crate) next_block_id: u64,
    pub(crate) scroll_y: f32,
}

pub(crate) struct App {
    pub(crate) has_key: bool,
    pub(crate) key_input: String,
    pub(crate) tabby_url_input: String,
    pub(crate) tabby_token_input: String,
    pub(crate) openai_compat_label: String,
    pub(crate) inference_command_input: String,
    pub(crate) inference_engine: InferenceEngine,
    pub(crate) inference_binary_path: String,
    pub(crate) inference_selected_model: String,
    pub(crate) inference_port_input: String,
    pub(crate) inference_pid: Option<u32>,
    pub(crate) inference_stop: Option<RuntimeStopHandle>,
    pub(crate) inference_generation: u64,
    pub(crate) inference_stopping: bool,
    pub(crate) inference_log: std::collections::VecDeque<String>,
    pub(crate) tabby_status: Option<Result<String, String>>,
    pub(crate) tabby_connect_retry_left: u8,
    pub(crate) tabby_retry_generation: u64,
    pub(crate) status: String,
    pub(crate) busy: bool,
    pub(crate) sidebar_width: f32,
    pub(crate) window_width: f32,
    pub(crate) models: Vec<OpenRouterModel>,
    pub(crate) model_ids: Vec<String>,
    pub(crate) models_request_generation: u64,
    pub(crate) account_request_generation: u64,
    pub(crate) selected_model: Option<String>,
    pub(crate) selected_model_provider: Option<LlmProvider>,
    pub(crate) blocks: Vec<Block>,
    pub(crate) next_block_id: u64,
    pub(crate) input: String,
    pub(crate) editor_content: text_editor::Content,
    pub(crate) streaming_block_id: Option<u64>,
    pub(crate) streaming_block_idx: Option<usize>,
    pub(crate) stream_generation: u64,
    pub(crate) streaming_raw: String,
    pub(crate) abort_handle: Option<task::Handle>,
    pub(crate) mcp_abort_handle: Option<task::Handle>,
    pub(crate) mcp_request_generation: u64,
    pub(crate) mcp_pending_results: usize,
    pub(crate) mcp_pending_call_ids: HashSet<String>,
    pub(crate) hf_abort_handle: Option<task::Handle>,
    pub(crate) ui: UiState,
    pub(crate) hf_token_input: String,
    pub(crate) show_hf_token: bool,
    pub(crate) model_dir_input: String,
    pub(crate) hf_repo_input: String,
    pub(crate) hf_revision: Option<String>,
    pub(crate) hf_folder_name: Option<String>,
    pub(crate) hf_dl: Option<HfDownload>,
    pub(crate) stream_id: ScrollId,
    pub(crate) current_scroll_y: f32,
    pub(crate) conversation: Arc<Vec<ChatMessage>>,
    pub(crate) pending_tool_calls: Vec<PendingToolCall>,
    pub(crate) tool_round: u32,
    pub(crate) mid_stream_retries: u32,
    pub(crate) cwd: PathBuf,
    pub(crate) model_combo_state: combo_box::State<ModelOption>,
    pub(crate) model_options: Vec<ModelOption>,
    pub(crate) account: Option<AuthKeyData>,
    pub(crate) pending_write_calls: Vec<PendingToolCall>,
    pub(crate) model_filter: ModelFilterState,
    pub(crate) agent_mode: AgentMode,
    pub(crate) inactive_sessions: Vec<InactiveSession>,
    pub(crate) current_session_id: u64,
    pub(crate) current_session_title: String,
    pub(crate) next_session_id: u64,
    pub(crate) usage: session::UsageStore,
    pub(crate) generation_lookup_generation: u64,
    pub(crate) last_response_cost: Option<f64>,
    pub(crate) last_response_time_ms: Option<u64>,
    pub(crate) response_started_at: Option<std::time::Instant>,
    pub(crate) attached_files: Vec<(PathBuf, String)>,
    pub(crate) mention_query: String,
    pub(crate) mention_candidates: Vec<PathBuf>,
    pub(crate) mention_selected: usize,
    pub(crate) pty_output: std::collections::VecDeque<String>,
    pub(crate) pty_input: String,
    pub(crate) pty_session: Option<pty::PtySession>,
    pub(crate) pty_generation: u64,
    pub(crate) mcp_servers: Vec<mcp::McpServer>,
    pub(crate) mcp_tool_load_generations: std::collections::HashMap<String, u64>,
    pub(crate) mcp_tools: Vec<mcp::McpTool>,
    pub(crate) mcp_input: McpInputState,
    pub(crate) theme_config: session::ThemeConfig,
    pub(crate) theme_apply_msg: String,
    pub(crate) file_tree_items: Vec<crate::util::file_tree::FileTreeItem>,
    pub(crate) file_tree_expanded: std::collections::HashSet<std::path::PathBuf>,
    pub(crate) skeleton_phase: u8,
    pub(crate) tldr_expanded: std::collections::HashSet<u64>,
    pub(crate) tldr_data: std::collections::HashMap<u64, Vec<crate::update_tldr::TldrFileEntry>>,
    pub(crate) hovered_code_blocks: std::collections::HashSet<u64>,
    pub(crate) hovered_block: Option<u64>,
    pub(crate) hovered_session: Option<u64>,
    pub(crate) hovered_context_idx: Option<usize>,
    pub(crate) hovered_settings_tab: Option<SettingsTab>,
    pub(crate) hovered_palette_idx: Option<usize>,
    pub(crate) hovered_confirm_idx: Option<usize>,
    pub(crate) hovered_attach_idx: Option<usize>,
    pub(crate) hovered_shortcut_idx: Option<usize>,
    pub(crate) hovered_pty: bool,
    pub(crate) hovered_mcp_idx: Option<usize>,
    pub(crate) compare_old_text: Option<String>,
    pub(crate) compare_new_text: Option<String>,
    pub(crate) compare_block_ids: Option<(u64, u64)>,
    pub(crate) compare_generation: u64,
    pub(crate) toast: Option<String>,
    pub(crate) close_in_progress: bool,
    #[cfg(test)]
    pub(crate) close_lifecycle_events: Vec<CloseLifecycleEvent>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CloseLifecycleEvent {
    CancelStreamAndMcp,
    ReapInferenceAndPty,
    Persist,
    MarkClean,
    CloseWindow,
}

#[derive(Clone, Debug)]
pub(crate) struct CloseReapOutcome {
    pub(crate) result: Result<(), String>,
    pub(crate) inference: Option<crate::runtime_process::RuntimeStopHandle>,
    pub(crate) pty: Option<crate::pty::PtySession>,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(handle) = &self.inference_stop {
            let _requested = handle.request_stop();
        }
    }
}

impl App {
    pub(crate) fn try_persist(&mut self, result: Result<(), String>, context: &str) {
        if let Err(e) = result {
            self.status = format!("{context}: {e}");
        }
    }
}
