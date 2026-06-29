use super::{
    AgentMode, App, Arc, Color, InferenceEngine, McpInputState, Message, ModelFilterState, PathBuf,
    SIDEBAR_WIDTH, ScrollId, Task, Theme, UiState, combo_box, keystore, mcp, session, text_editor,
};

impl App {
    #[allow(clippy::unused_self)]
    pub(crate) fn title(&self) -> String {
        "CodeWarp".to_string()
    }

    #[allow(clippy::unused_self)]
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
            window_width: 1280.0,
            models: Vec::new(),
            model_ids: Vec::new(),
            selected_model: saved_model,
            selected_model_provider: None,
            compare_both: false,
            compare_pending: false,
            blocks: Vec::new(),
            next_block_id: 0,
            input: String::new(),
            editor_content: text_editor::Content::new(),
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

        app.auto_attach_tabbyapi();
        let scroll_off = app.restore_sessions();
        let task = app.build_startup_tasks(scroll_off);
        (app, task)
    }
}
