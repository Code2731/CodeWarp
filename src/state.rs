// state.rs — Core application state types (child module of main)
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

#[derive(Debug, Clone)]
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
    pub(crate) show_tabby_token: bool,
    pub(crate) openai_compat_label: String,
    pub(crate) inference_command_input: String,
    pub(crate) inference_engine: InferenceEngine,
    pub(crate) inference_binary_path: String,
    pub(crate) inference_selected_model: String,
    pub(crate) inference_port_input: String,
    pub(crate) inference_pid: Option<u32>,
    pub(crate) inference_log: std::collections::VecDeque<String>,
    pub(crate) tabby_status: Option<Result<String, String>>,
    pub(crate) tabby_connect_retry_left: u8,
    pub(crate) tabby_retry_generation: u64,
    pub(crate) status: String,
    pub(crate) busy: bool,
    pub(crate) sidebar_width: f32,
    pub(crate) models: Vec<OpenRouterModel>,
    pub(crate) model_ids: Vec<String>,
    pub(crate) selected_model: Option<String>,
    pub(crate) selected_model_provider: Option<LlmProvider>,
    pub(crate) compare_both: bool,
    pub(crate) compare_pending: bool,
    pub(crate) blocks: Vec<Block>,
    pub(crate) next_block_id: u64,
    pub(crate) input: String,
    pub(crate) streaming_block_id: Option<u64>,
    pub(crate) streaming_block_idx: Option<usize>,
    pub(crate) streaming_raw: String,
    pub(crate) abort_handle: Option<task::Handle>,
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
    pub(crate) follow_bottom: bool,
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
    pub(crate) show_write_confirm: bool,
    pub(crate) model_filter: ModelFilterState,
    pub(crate) agent_mode: AgentMode,
    pub(crate) inactive_sessions: Vec<InactiveSession>,
    pub(crate) current_session_id: u64,
    pub(crate) current_session_title: String,
    pub(crate) next_session_id: u64,
    pub(crate) usage: session::UsageStore,
    pub(crate) last_response_cost: Option<f64>,
    pub(crate) attached_files: Vec<(PathBuf, String)>,
    pub(crate) show_mention: bool,
    pub(crate) mention_query: String,
    pub(crate) mention_candidates: Vec<PathBuf>,
    pub(crate) mention_selected: usize,
    pub(crate) pty_visible: bool,
    pub(crate) pty_output: std::collections::VecDeque<String>,
    pub(crate) pty_input: String,
    pub(crate) pty_session: Option<pty::PtySession>,
    pub(crate) mcp_servers: Vec<mcp::McpServer>,
    pub(crate) mcp_tools: Vec<mcp::McpTool>,
    pub(crate) mcp_input: McpInputState,
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(pid) = self.inference_pid {
            kill_pid(pid);
        }
    }
}

impl App {
    pub(crate) fn title(&self) -> String {
        "CodeWarp".to_string()
    }

    pub(crate) fn theme(&self) -> Theme {
        Theme::custom(
            "CodeWarp Dark".to_string(),
            iced::theme::Palette {
                background: Color::from_rgb8(0x03, 0x07, 0x12),
                text: Color::from_rgb8(0xe6, 0xec, 0xf8),
                primary: Color::from_rgb8(0x0e, 0xa5, 0xe9),
                success: Color::from_rgb8(0x10, 0xb9, 0x81),
                warning: Color::from_rgb8(0xf5, 0x9e, 0x0b),
                danger: Color::from_rgb8(0xf4, 0x71, 0x74),
            },
        )
    }

    pub(crate) fn new() -> (Self, Task<Message>) {
        let has_key = keystore::has_api_key();
        let saved_model = keystore::read_selected_model();
        let status = if has_key {
            "준비됨".into()
        } else {
            "OpenRouter API 키 미등록".into()
        };
        let saved_tabby_url = keystore::read_tabby_base_url().unwrap_or_default();
        let saved_tabby_token = keystore::read_tabby_token().unwrap_or_default();
        let mut app = Self {
            has_key,
            key_input: String::new(),
            tabby_url_input: saved_tabby_url,
            tabby_token_input: saved_tabby_token.clone(),
            show_tabby_token: !saved_tabby_token.trim().is_empty(),
            openai_compat_label: keystore::read_openai_compat_label().unwrap_or_default(),
            inference_command_input: keystore::read_inference_command().unwrap_or_default(),
            inference_engine: InferenceEngine::XLlm,
            inference_binary_path: keystore::read_inference_binary().unwrap_or_default(),
            inference_selected_model: String::new(),
            inference_port_input: "9000".into(),
            inference_pid: None,
            inference_log: std::collections::VecDeque::new(),
            tabby_status: None,
            tabby_connect_retry_left: 0,
            tabby_retry_generation: 0,
            status,
            busy: false,
            sidebar_width: SIDEBAR_WIDTH,
            models: Vec::new(),
            model_ids: Vec::new(),
            selected_model: saved_model,
            selected_model_provider: None,
            compare_both: false,
            compare_pending: false,
            blocks: Vec::new(),
            next_block_id: 0,
            input: String::new(),
            streaming_block_id: None,
            streaming_block_idx: None,
            streaming_raw: String::new(),
            abort_handle: None,
            hf_abort_handle: None,
            ui: UiState::new(!has_key),
            hf_token_input: keystore::read_hf_token().unwrap_or_default(),
            show_hf_token: false,
            model_dir_input: keystore::read_model_dir().unwrap_or_else(|| {
                dirs::data_local_dir()
                    .map(|d| d.join("codewarp").join("models").display().to_string())
                    .unwrap_or_default()
            }),
            hf_repo_input: String::new(),
            hf_revision: None,
            hf_folder_name: None,
            hf_dl: None,
            stream_id: ScrollId::new("stream"),
            follow_bottom: true,
            current_scroll_y: 0.0,
            conversation: Arc::new(Vec::new()),
            pending_tool_calls: Vec::new(),
            tool_round: 0,
            mid_stream_retries: 0,
            cwd: keystore::read_cwd()
                .map(PathBuf::from)
                .filter(|p| p.is_dir())
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from(".")),
            model_combo_state: combo_box::State::new(Vec::new()),
            model_options: Vec::new(),
            account: None,
            pending_write_calls: Vec::new(),
            show_write_confirm: false,
            model_filter: ModelFilterState::new(),
            agent_mode: AgentMode::Plan,
            inactive_sessions: Vec::new(),
            current_session_id: 1,
            current_session_title: String::new(),
            next_session_id: 1,
            usage: session::load_usage(),
            last_response_cost: None,
            attached_files: Vec::new(),
            show_mention: false,
            pty_visible: false,
            pty_output: std::collections::VecDeque::new(),
            pty_input: String::new(),
            pty_session: None,
            mcp_servers: mcp::load_servers(),
            mcp_tools: Vec::new(),
            mcp_input: McpInputState::default(),
            mention_query: String::new(),
            mention_candidates: Vec::new(),
            mention_selected: 0,
        };

        let should_auto_attach_tabbyapi = app.openai_compat_label.eq_ignore_ascii_case("TabbyAPI")
            || app.tabby_url_input.contains(":5000");
        if should_auto_attach_tabbyapi && app.inference_binary_path.trim().is_empty() {
            if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir()) {
                app.inference_engine = InferenceEngine::TabbyApi;
                app.inference_port_input = TABBY_API_DEFAULT_PORT.to_string();
                if app.tabby_url_input.trim().is_empty() {
                    app.tabby_url_input = format!("http://localhost:{}", TABBY_API_DEFAULT_PORT);
                }
                app.inference_binary_path = launcher.display().to_string();
                let _ = keystore::write_inference_binary(&app.inference_binary_path);
            }
        }

        let mut persisted = session::load_all();
        if persisted.sessions.is_empty() {
            persisted = session::load_all();
        }
        let active_idx = persisted
            .active_idx
            .min(persisted.sessions.len().saturating_sub(1));
        let active = persisted.sessions[active_idx].clone();
        let inactive: Vec<InactiveSession> = persisted
            .sessions
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != active_idx)
            .map(|(_, s)| InactiveSession {
                id: s.id,
                title: s.title.clone(),
                conversation: Arc::new(s.conversation.clone()),
                blocks: s.blocks.clone(),
                next_block_id: s.next_block_id,
                scroll_y: s.scroll_y,
            })
            .collect();

        app.current_session_id = active.id;
        app.current_session_title = active.title;
        app.conversation = Arc::new(active.conversation);
        app.next_block_id = active.next_block_id;
        app.blocks = active.blocks.into_iter().map(persisted_to_block).collect();
        app.current_scroll_y = active.scroll_y;
        app.inactive_sessions = inactive;
        app.next_session_id = persisted.sessions.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        if !session::was_clean_shutdown() && !app.blocks.is_empty() {
            app.status = format!("[복구됨] {}", app.status);
        }
        let restore_scroll = if app.current_scroll_y > 0.0 {
            Some(iced::widget::operation::scroll_to(
                app.stream_id.clone(),
                iced::widget::scrollable::AbsoluteOffset {
                    x: 0.0,
                    y: app.current_scroll_y,
                },
            ))
        } else {
            None
        };

        let mut tasks: Vec<Task<Message>> = Vec::new();
        if has_key {
            tasks.push(Task::done(Message::FetchModels));
            tasks.push(Task::done(Message::FetchAccount));
        }
        if !app.tabby_url_input.trim().is_empty() {
            tasks.push(Task::done(Message::FetchTabbyModels));
        }
        if !app.inference_command_input.trim().is_empty() {
            tasks.push(Task::done(Message::StartInference));
        }
        for server in app.mcp_servers.clone() {
            let name = server.name.clone();
            tasks.push(Task::perform(
                async move {
                    mcp::list_tools(&server)
                        .await
                        .map(|tools| (name.clone(), tools))
                        .map_err(|e| format!("[{name}] {e}"))
                },
                |r| match r {
                    Ok((name, tools)) => Message::McpToolsLoaded(name, tools),
                    Err(msg) => Message::McpToolsFailed(msg),
                },
            ));
        }
        if let Some(t) = restore_scroll {
            tasks.push(t);
        }
        let task = if tasks.is_empty() {
            Task::none()
        } else {
            Task::batch(tasks)
        };
        (app, task)
    }
}
