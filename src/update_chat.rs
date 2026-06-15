// update_chat.rs — Chat-related App update methods (main.rs child module)
use super::*;
use iced::Task;

impl App {
    pub(crate) fn on_stream_scrolled(
        &mut self,
        viewport: &iced::widget::scrollable::Viewport,
    ) -> Task<Message> {
        let rel = viewport.relative_offset();
        self.follow_bottom = rel.y > 0.95;
        self.current_scroll_y = viewport.absolute_offset().y;
        Task::none()
    }
    pub(crate) fn on_editor_action(
        &mut self,
        id: u64,
        action: iced::widget::text_editor::Action,
    ) -> Task<Message> {
        if action.is_edit() {
            return Task::none();
        }
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            if let BlockBody::Assistant(content) = &mut b.body {
                content.perform(action);
            }
        }
        Task::none()
    }
    pub(crate) fn toggle_block_view(&mut self, id: u64) -> Task<Message> {
        if let Some(b) = self.blocks.iter_mut().find(|b| b.id == id) {
            b.view_mode = match b.view_mode {
                ViewMode::Rendered => ViewMode::Raw,
                ViewMode::Raw => {
                    if let BlockBody::Assistant(content) = &b.body {
                        b.md_items = markdown::parse(&content.text()).collect();
                    }
                    ViewMode::Rendered
                }
            };
        }
        Task::none()
    }
    pub(crate) fn on_link_clicked(&mut self, uri: &markdown::Uri) -> Task<Message> {
        let url = uri.to_string();
        let lower = url.to_ascii_lowercase();
        if lower.starts_with("javascript:") {
            self.status = "차단된 링크 스킴입니다.".into();
            return Task::none();
        }
        match webbrowser::open(&url) {
            Ok(_) => {
                self.status = format!("브라우저에서 열기: {}", url);
            }
            Err(e) => {
                self.status = format!("링크 열기 실패: {}", e);
            }
        }
        Task::none()
    }
    pub(crate) fn copy_block(&self, id: u64) -> Task<Message> {
        if self.streaming_block_id == Some(id) && !self.streaming_raw.is_empty() {
            return iced::clipboard::write(self.streaming_raw.clone());
        }
        if let Some(b) = self.blocks.iter().find(|b| b.id == id) {
            return iced::clipboard::write(b.body.to_text());
        }
        Task::none()
    }
    pub(crate) fn on_compare_responses_loaded(
        &mut self,
        openrouter_block_id: u64,
        tabby_block_id: u64,
        openrouter_result: Result<String, String>,
        tabby_result: Result<String, String>,
    ) -> Task<Message> {
        if !self.compare_pending {
            return Task::none();
        }
        let openrouter_text = match openrouter_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[OpenRouter] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", openrouter::humanize_error(&e)),
        };
        let tabby_text = match tabby_result {
            Ok(text) if !text.trim().is_empty() => text,
            Ok(_) => "[Tabby] 빈 응답".into(),
            Err(e) => format!("[ERROR] {}", tabby::humanize_error(&e)),
        };
        self.fill_assistant_block(openrouter_block_id, openrouter_text.clone());
        self.fill_assistant_block(tabby_block_id, tabby_text.clone());
        Arc::make_mut(&mut self.conversation).push(ChatMessage::assistant(format!(
            "[OpenRouter]\n{}\n\n[Tabby]\n{}",
            openrouter_text, tabby_text
        )));
        self.compare_pending = false;
        self.status = "Compare 응답 완료".into();
        self.maybe_update_title();
        self.save_session();
        if self.follow_bottom {
            snap_to_end(self.stream_id.clone())
        } else {
            Task::none()
        }
    }
    pub(crate) fn compare_routes(&self) -> Result<(ChatRoute, ChatRoute), String> {
        let selected = self.selected_option();
        let openrouter_model = selected
            .filter(|o| o.provider == LlmProvider::OpenRouter)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenRouter)
            })
            .ok_or_else(|| "Compare 모드: OpenRouter 모델이 없습니다. OpenRouter 키/모델 목록을 먼저 불러와 주세요.".to_string())?;
        let tabby_model = selected
            .filter(|o| o.provider == LlmProvider::OpenAICompat)
            .or_else(|| {
                self.model_options
                    .iter()
                    .find(|o| o.provider == LlmProvider::OpenAICompat)
            })
            .ok_or_else(|| "Compare 모드: Tabby 모델이 없습니다. Provider 연결 테스트로 Tabby 모델을 먼저 불러와 주세요.".to_string())?;

        let openrouter_key = keystore::read_api_key()?;
        let tabby_base = if self.tabby_url_input.trim().is_empty() {
            keystore::read_tabby_base_url()
        } else {
            Some(self.tabby_url_input.clone())
        }
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| "Compare 모드: Tabby URL 미설정".to_string())?;
        let tabby_token = if self.tabby_token_input.trim().is_empty() {
            keystore::read_tabby_token()
        } else {
            Some(self.tabby_token_input.clone())
        }
        .filter(|s| !s.trim().is_empty());

        Ok((
            ChatRoute {
                label: "OpenRouter".into(),
                base_url: openrouter::BASE_URL.to_string(),
                api_key: Some(openrouter_key),
                model: openrouter_model.id.clone(),
            },
            ChatRoute {
                label: if tabby_model.provider_label.trim().is_empty() {
                    "Local".into()
                } else {
                    tabby_model.provider_label.trim().to_string()
                },
                base_url: tabby::chat_base(&tabby_base),
                api_key: tabby_token,
                model: tabby_model.id.clone(),
            },
        ))
    }
    pub(crate) fn ensure_system_message(&mut self) {
        let mode_block = match self.agent_mode {
            AgentMode::Plan => {
                "현재 모드: Plan (분석/계획 전용)\n\
                Plan 모드에서는 read_file/glob/grep으로 코드를 조사하고 변경 계획만 \
                제시하세요. 실제 파일 변경이나 명령 실행은 Build 모드에서만 가능하므로, \
                계획에 '필요한 변경'을 명확히 적고 사용자가 Build로 전환하기를 기다리세요.\n\n"
            }
            AgentMode::Build => {
                "현재 모드: Build (실행 가능)\n\
                Build 모드에서는 write_file/run_command를 사용해 실제 변경을 적용할 수 \
                있습니다. 단, 두 도구 모두 사용자 승인을 거치므로 부담 없이 호출하세요.\n\n"
            }
        };
        let prompt = format!(
            "당신은 CodeWarp의 코딩 어시스턴트입니다.\n\n\
            작업 디렉토리: '{}'\n\n\
            {}\
            사용 가능한 도구 (적극적으로 호출하세요):\n\
            - read_file(path): 파일 내용 읽기 (즉시 실행)\n\
            - write_file(path, content): 파일 작성/덮어쓰기 (Build 모드 + 사용자 승인)\n\
            - run_command(command): 셸 명령 실행 (Build 모드 + 사용자 승인)\n\
            - glob(pattern): 패턴 매칭 파일 리스트 (예: '**/*.rs', 'examples/**/*')\n\
            - grep(pattern): 정규식으로 모든 파일 검색\n\n\
            규칙:\n\
            1. 파일 시스템을 살펴봐야 할 때는 '확인하겠습니다' 같은 말 없이 즉시 도구를 호출하세요.\n\
            2. 새 파일을 만들기 전에 glob으로 기존 구조를 먼저 확인하세요.\n\
            3. 모든 path 인자는 작업 디렉토리 기준 상대 경로 (절대 경로 거부).\n\
            4. 도구 결과를 받은 뒤 그것을 근거로 한국어로 답하세요.\n\
            5. **마크다운 형식 제약** (한국어 폰트 한계): italic(*text* 또는 _text_)은 \
            사용하지 마세요. 강조는 오직 **굵게**만 사용. 별표 한 개로 감싸지 말고, \
            정말 강조가 필요하면 두 개로 감싸세요.\n\
            6. **Apply 가능한 코드 블록**: 사용자가 그대로 파일에 적용할 수 있도록, \
            새 파일/덮어쓸 파일의 코드 블록은 첫 줄에 다음 주석을 포함하세요:\n\
            - Rust/JS/C 계열: `// path: 상대경로`\n\
            - Python/shell/yaml: `# path: 상대경로`\n\
            예) ```rust\\n// path: src/foo.rs\\nfn main() {{}}\\n```\n\
            그러면 코드 블록 옆에 'Apply' 버튼이 노출되어 사용자가 한 번에 적용할 수 있습니다. \
            단순 예시 코드(개념 설명용)에는 path 주석을 넣지 마세요.",
            self.cwd.display(),
            mode_block,
        );
        if let Some(first) = Arc::make_mut(&mut self.conversation).first_mut() {
            if first.role == "system" {
                first.content = Some(prompt);
                return;
            }
        }
        Arc::make_mut(&mut self.conversation).insert(
            0,
            ChatMessage {
                role: "system".into(),
                content: Some(prompt),
                ..Default::default()
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assistant_block_with_text(id: u64, text: &str) -> Block {
        Block {
            id,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(text)),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        }
    }

    fn or_opt(id: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenRouter,
            provider_label: String::new(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    fn oai_opt(id: &str, label: &str) -> ModelOption {
        ModelOption {
            id: id.into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: label.into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: None,
            completion_per_million: None,
        }
    }

    #[test]
    fn abort_stream_keeps_partial_assistant_when_requested() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.tool_round = 3;
        app.pending_tool_calls = vec![PendingToolCall {
            id: "tc-1".into(),
            name: "read_file".into(),
            arguments: "{}".into(),
        }];
        app.blocks
            .push(assistant_block_with_text(42, "partial response"));

        app.abort_active_chat_stream(true);

        assert!(app.streaming_block_id.is_none());
        assert!(app.streaming_block_idx.is_none());
        assert!(app.pending_tool_calls.is_empty());
        assert_eq!(app.tool_round, 0);
        assert_eq!(app.conversation.len(), 1);
        assert_eq!(app.conversation[0].role, "assistant");
        assert_eq!(
            app.conversation[0].content.as_deref(),
            Some("partial response")
        );
    }

    #[test]
    fn abort_stream_drops_partial_assistant_when_not_requested() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(7);
        app.streaming_block_idx = Some(0);
        app.tool_round = 2;
        app.pending_tool_calls = vec![PendingToolCall {
            id: "tc-2".into(),
            name: "glob".into(),
            arguments: "{}".into(),
        }];
        app.blocks
            .push(assistant_block_with_text(7, "to be discarded"));

        app.abort_active_chat_stream(false);

        assert!(app.streaming_block_id.is_none());
        assert!(app.streaming_block_idx.is_none());
        assert!(app.pending_tool_calls.is_empty());
        assert_eq!(app.tool_round, 0);
        assert!(app.conversation.is_empty());
    }

    #[test]
    fn abort_stream_handles_missing_assistant_block() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(999);
        app.tool_round = 1;
        app.pending_tool_calls = vec![PendingToolCall {
            id: "tc-3".into(),
            name: "grep".into(),
            arguments: "{}".into(),
        }];

        app.abort_active_chat_stream(true);

        assert!(app.streaming_block_id.is_none());
        assert!(app.pending_tool_calls.is_empty());
        assert_eq!(app.tool_round, 0);
        assert!(app.conversation.is_empty());
    }

    #[test]
    fn chat_chunk_tokens_append_to_assistant_block_without_editor_focus() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Rendered,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ChatChunk(ChatEvent::Token("hel".into())));
        let _ = app.update(Message::ChatChunk(ChatEvent::Token("lo".into())));

        assert_eq!(app.streaming_raw, "hello");
    }

    #[test]
    fn chat_chunk_does_not_reparse_markdown_during_streaming() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ChatChunk(ChatEvent::Token("**hello**".into())));
        assert!(
            app.blocks[0].md_items.is_empty(),
            "md_items should stay empty during streaming (F16 perf fix)"
        );
        assert_eq!(app.streaming_raw, "**hello**");
    }

    #[test]
    fn toggle_block_view_to_rendered_triggers_markdown_parse() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        let id = 42;
        app.blocks.push(Block {
            id,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(
                "**bold** text",
            )),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ToggleBlockView(id));
        assert_eq!(app.blocks[0].view_mode, ViewMode::Rendered);
        assert!(
            !app.blocks[0].md_items.is_empty(),
            "md_items should be populated after toggling to Rendered"
        );
    }

    #[test]
    fn toggle_block_view_to_raw_clears_no_md_items() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        let id = 42;
        app.blocks.push(Block {
            id,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text("hello")),
            view_mode: ViewMode::Rendered,
            md_items: vec![], // pretend it was parsed
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ToggleBlockView(id));
        assert_eq!(app.blocks[0].view_mode, ViewMode::Raw);
    }

    #[test]
    fn on_mcp_tools_loaded_removes_old_tools_and_updates_status() {
        let (mut app, _) = App::new();
        app.mcp_tools.push(mcp::McpTool {
            server_name: "fs".into(),
            name: "read".into(),
            description: "".into(),
            input_schema: serde_json::json!({}),
        });
        app.mcp_tools.push(mcp::McpTool {
            server_name: "old-server".into(),
            name: "list".into(),
            description: "".into(),
            input_schema: serde_json::json!({}),
        });

        let new_tools = vec![
            mcp::McpTool {
                server_name: "fs".into(),
                name: "read".into(),
                description: "read file".into(),
                input_schema: serde_json::json!({}),
            },
            mcp::McpTool {
                server_name: "fs".into(),
                name: "write".into(),
                description: "write file".into(),
                input_schema: serde_json::json!({}),
            },
        ];

        let _ = app.on_mcp_tools_loaded("fs".into(), new_tools);
        assert_eq!(app.mcp_tools.len(), 3);
        assert!(app.mcp_tools.iter().any(|t| t.server_name == "old-server"));
        assert!(app.mcp_tools.iter().any(|t| t.name == "write"));
        assert!(app.status.contains("MCP"));
    }

    #[test]
    fn on_mcp_tools_failed_shows_error_in_status() {
        let (mut app, _) = App::new();
        let _ = app.on_mcp_tools_failed("connection refused".into());
        assert!(app.status.contains("MCP tool 로드 실패"));
        assert!(app.status.contains("connection refused"));
    }

    #[test]
    fn compare_mode_send_requires_registered_providers() {
        let (mut app, _) = App::new();
        app.compare_both = true;
        app.input = "compare this".into();
        app.selected_model = None;
        app.model_options.clear();
        let before_blocks = app.blocks.len();

        let _ = app.update(Message::Send);

        assert!(
            app.status.contains("Compare 모드: OpenRouter 모델"),
            "got: {}",
            app.status
        );
        assert_eq!(app.blocks.len(), before_blocks);
    }

    #[test]
    fn send_message_returns_early_when_streaming() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.input = "hello".into();
        let before = app.conversation.len();

        let _ = app.update(Message::Send);

        assert_eq!(
            app.conversation.len(),
            before,
            "should not send while streaming"
        );
        assert_eq!(app.streaming_block_id, Some(42));
    }

    #[test]
    fn send_message_returns_early_when_input_empty() {
        let (mut app, _) = App::new();
        app.input.clear();

        let _ = app.update(Message::Send);

        assert!(
            app.status.is_empty() || app.status == "준비됨" || app.status.starts_with("[복구됨]"),
            "unexpected status: {}",
            app.status
        );
    }

    #[test]
    fn regenerate_last_returns_early_when_streaming() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).push(ChatMessage::user("hello"));
        app.streaming_block_id = Some(42);
        let before = app.conversation.len();

        let _ = app.update(Message::RegenerateLast);

        assert_eq!(app.conversation.len(), before);
    }

    #[test]
    fn regenerate_last_returns_early_when_no_user_message() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();

        let _ = app.update(Message::RegenerateLast);

        assert!(
            app.status.is_empty() || app.status == "준비됨" || app.status.starts_with("[복구됨]"),
            "unexpected status: {}",
            app.status
        );
    }

    #[test]
    fn toggle_compare_mode_updates_status() {
        let (mut app, _) = App::new();

        let _ = app.update(Message::ToggleCompareBoth(true));

        assert!(app.compare_both);
        assert!(app.status.contains("Compare 모드"), "got: {}", app.status);

        let _ = app.update(Message::ToggleCompareBoth(false));

        assert!(!app.compare_both);
        assert!(app.status.contains("Single 모드"), "got: {}", app.status);
    }

    #[test]
    fn local_openai_compat_models_do_not_send_tool_definitions() {
        let (mut app, _) = App::new();
        app.model_options = vec![ModelOption {
            id: "local-model".into(),
            provider: LlmProvider::OpenAICompat,
            provider_label: "TabbyAPI".into(),
            ko_friendly: false,
            favorite: false,
            context_length: None,
            prompt_per_million: Some(0.0),
            completion_per_million: Some(0.0),
        }];
        app.selected_model = Some("local-model".into());

        assert!(app.tool_definitions_for_selected_model().is_none());
    }

    #[test]
    fn selected_model_with_same_id_uses_explicit_provider_choice() {
        let (mut app, _) = App::new();
        app.model_options = vec![or_opt("shared-model"), oai_opt("shared-model", "TabbyAPI")];
        app.tabby_url_input = "http://localhost:5000".into();

        let _ = app.update(Message::SelectModel(oai_opt("shared-model", "TabbyAPI")));
        assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenAICompat));
        assert!(app.tool_definitions_for_selected_model().is_none());

        let _ = app.update(Message::SelectModel(or_opt("shared-model")));
        assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenRouter));
        assert!(app.tool_definitions_for_selected_model().is_some());
    }

    #[test]
    fn resolve_provider_prefers_current_tabby_inputs_over_keystore() {
        let (mut app, _) = App::new();
        app.model_options = vec![oai_opt("local-model", "TabbyAPI")];
        app.selected_model = Some("local-model".into());
        app.selected_model_provider = Some(LlmProvider::OpenAICompat);
        app.tabby_url_input = "http://localhost:5001".into();
        app.tabby_token_input = "live-token".into();

        let (base_url, api_key) = app.resolve_provider().expect("provider resolves");

        assert_eq!(base_url, "http://localhost:5001/v1");
        assert_eq!(api_key.as_deref(), Some("live-token"));
    }

    #[test]
    fn chat_chunk_done_builds_content_from_streaming_raw() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app.streaming_raw = "hello world".into();

        let _ = app.update(Message::ChatChunk(ChatEvent::Done {
            finish_reason: Some("stop".into()),
            generation_id: None,
        }));

        assert_eq!(app.blocks[0].body.to_text(), "hello world");
        assert!(!app.blocks[0].md_items.is_empty());
        assert!(app.streaming_raw.is_empty());
        assert!(app.streaming_block_id.is_none());
        assert_eq!(app.conversation.len(), 1);
        assert_eq!(app.conversation[0].content.as_deref(), Some("hello world"));
    }

    #[test]
    fn chat_chunk_done_empty_streaming_raw_shows_warning() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app.streaming_raw = String::new();

        let _ = app.update(Message::ChatChunk(ChatEvent::Done {
            finish_reason: Some("stop".into()),
            generation_id: None,
        }));

        assert_eq!(app.blocks[0].body.to_text(), "[WARN] empty response");
        assert!(app.conversation.is_empty());
    }

    #[test]
    fn chat_chunk_error_appends_to_streaming_raw() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = MAX_MID_STREAM_RETRIES; // prevent mid-stream retry from clearing content

        let _ = app.update(Message::ChatChunk(ChatEvent::Error("server error".into())));

        assert!(app.blocks[0].body.to_text().contains("partial text"));
        assert!(app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] server error"));
        assert!(app.streaming_block_id.is_none());
    }

    #[test]
    fn chat_chunk_error_empty_streaming_raw() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });

        let _ = app.update(Message::ChatChunk(ChatEvent::Error("server error".into())));

        assert!(app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] server error"));
        assert!(app.streaming_block_id.is_none());
    }

    #[test]
    fn mid_stream_error_triggers_retry() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = 0;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error(
            "connection dropped".into(),
        )));

        assert_eq!(app.mid_stream_retries, 1);
        assert!(app.blocks[0].body.to_text().is_empty());
        assert!(app.streaming_raw.is_empty());
    }

    #[test]
    fn mid_stream_error_retries_exhausted() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = MAX_MID_STREAM_RETRIES;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error(
            "connection dropped".into(),
        )));

        assert!(app.blocks[0]
            .body
            .to_text()
            .contains("[ERROR] connection dropped"));
        assert_eq!(app.mid_stream_retries, MAX_MID_STREAM_RETRIES);
        assert!(app.streaming_block_id.is_none());
    }

    #[test]
    fn mid_stream_error_401_not_retried() {
        let (mut app, _) = App::new();
        Arc::make_mut(&mut app.conversation).clear();
        app.blocks.clear();
        app.streaming_block_id = Some(42);
        app.streaming_block_idx = Some(0);
        app.blocks.push(Block {
            id: 42,
            body: BlockBody::Assistant(iced::widget::text_editor::Content::new()),
            view_mode: ViewMode::Raw,
            md_items: Vec::new(),
            model: None,
            apply_candidates: Vec::new(),
        });
        app.streaming_raw = "partial text".into();
        app.mid_stream_retries = 0;

        let _ = app.update(Message::ChatChunk(ChatEvent::Error(
            "OpenRouter 401 unauthorized".into(),
        )));

        assert!(app.blocks[0].body.to_text().contains("[ERROR]"));
        assert_eq!(app.mid_stream_retries, 0);
    }
}
