use super::*;

// ── fmt_bytes ───────────────────────────────────────────────────

#[test]
fn fmt_bytes_units() {
    assert_eq!(fmt_bytes(0), "0 B");
    assert_eq!(fmt_bytes(512), "512 B");
    assert_eq!(fmt_bytes(1024), "1 KB");
    assert_eq!(fmt_bytes(1024 * 1024), "1.0 MB");
    assert_eq!(fmt_bytes(1024 * 1024 * 1024), "1.00 GB");
}

#[test]
fn fmt_bytes_large_gb() {
    let n = 5_500_000_000u64; // 5.5GB 모델 정도
    let s = fmt_bytes(n);
    assert!(s.ends_with(" GB"), "got: {}", s);
    assert!(s.starts_with("5.1"), "got: {}", s); // 5.5e9 / 2^30 ≈ 5.12
}

// ── fmt_context_length ──────────────────────────────────────────

#[test]
fn fmt_context_length_units() {
    assert_eq!(fmt_context_length(500), "500");
    assert_eq!(fmt_context_length(8000), "8k");
    assert_eq!(fmt_context_length(128_000), "128k");
    assert_eq!(fmt_context_length(1_000_000), "1.0M");
    assert_eq!(fmt_context_length(2_500_000), "2.5M");
}

// ── parse_price_per_million ─────────────────────────────────────

#[test]
fn parse_price_per_million_typical() {
    // OpenRouter는 토큰당 USD를 문자열로 줌 (e.g. "0.000005" = $5/M)
    let p = parse_price_per_million(Some("0.000005"));
    assert!(matches!(p, Some(v) if (v - 5.0).abs() < 1e-9));
}

#[test]
fn parse_price_per_million_free() {
    let p = parse_price_per_million(Some("0"));
    assert_eq!(p, Some(0.0));
}

#[test]
fn parse_price_per_million_invalid() {
    assert_eq!(parse_price_per_million(None), None);
    assert_eq!(parse_price_per_million(Some("")), None);
    assert_eq!(parse_price_per_million(Some("abc")), None);
}
#[test]
fn humanize_inference_spawn_error_explains_missing_xllm_binary() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("xllm", &err);
    assert!(msg.contains("xllm"), "got: {}", msg);
    assert!(msg.to_ascii_lowercase().contains("path"), "got: {}", msg);
    assert!(
        msg.to_ascii_lowercase().contains("binary path"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_falls_back_for_other_errors() {
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let msg = humanize_inference_spawn_error("xllm", &err);
    assert!(msg.starts_with("xllm: "), "got: {}", msg);
}

#[test]
fn humanize_inference_spawn_error_handles_tabby_cmd_alias() {
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Access is denied");
    let msg = humanize_inference_spawn_error("tabby.cmd", &err);
    assert!(
        msg.contains("Tabby executable could not be started"),
        "got: {}",
        msg
    );
    assert!(msg.contains("tabby.cmd"), "got: {}", msg);
}

#[test]
fn humanize_inference_spawn_error_vllm_not_found() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("vllm", &err);
    assert!(msg.contains("vllm"), "got: {}", msg);
    assert!(
        msg.to_ascii_lowercase().contains("binary path"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_llama_server_not_found() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("llama-server", &err);
    assert!(msg.contains("llama-server"), "got: {}", msg);
    assert!(
        msg.to_ascii_lowercase().contains("binary path"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_tabby_not_found_falls_back() {
    let err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let msg = humanize_inference_spawn_error("tabby.exe", &err);
    assert!(msg.starts_with("tabby.exe:"), "got: {}", msg);
}

#[test]
fn humanize_inference_spawn_error_tabby_korean_access_denied() {
    let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "액세스가 거부됨");
    let msg = humanize_inference_spawn_error("tabby.bat", &err);
    assert!(
        msg.contains("Tabby executable could not be started"),
        "got: {}",
        msg
    );
}

#[test]
fn humanize_inference_spawn_error_generic_fallback() {
    let err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "connection refused");
    let msg = humanize_inference_spawn_error("my-tool", &err);
    assert_eq!(msg, "my-tool: connection refused");
}

// ── input::handle_key ──────────────────────────────────────────

#[test]
fn input_escape_closes_overlays() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(
        Key::Named(iced::keyboard::key::Named::Escape),
        Modifiers::default(),
    );
    assert!(matches!(result, Some(Message::CloseAllOverlays)));
}

#[test]
fn input_cmd_k_opens_palette() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(Key::Character("k".into()), Modifiers::COMMAND);
    assert!(matches!(result, Some(Message::OpenCommandPalette)));
}

#[test]
fn input_cmd_n_new_chat() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(Key::Character("n".into()), Modifiers::COMMAND);
    assert!(matches!(result, Some(Message::NewChat)));
}

#[test]
fn input_cmd_comma_settings() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(Key::Character(",".into()), Modifiers::COMMAND);
    assert!(matches!(result, Some(Message::OpenSettings)));
}

#[test]
fn input_arrow_keys_are_not_handle_key() {
    use iced::keyboard::{Key, Modifiers};
    assert!(input::handle_key(
        Key::Named(iced::keyboard::key::Named::ArrowUp),
        Modifiers::default()
    )
    .is_none());
    assert!(input::handle_key(
        Key::Named(iced::keyboard::key::Named::ArrowDown),
        Modifiers::default()
    )
    .is_none());
}

#[test]
fn input_cmd_backtick_toggles_terminal() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(Key::Character("`".into()), Modifiers::COMMAND);
    assert!(matches!(result, Some(Message::PtyToggle)));
}

#[test]
fn input_cmd_shift_p_sets_plan_mode() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(
        Key::Character("p".into()),
        Modifiers::COMMAND | Modifiers::SHIFT,
    );
    assert!(matches!(
        result,
        Some(Message::SetAgentMode(AgentMode::Plan))
    ));
}

#[test]
fn input_cmd_shift_b_sets_build_mode() {
    use iced::keyboard::{Key, Modifiers};
    let result = input::handle_key(
        Key::Character("b".into()),
        Modifiers::COMMAND | Modifiers::SHIFT,
    );
    assert!(matches!(
        result,
        Some(Message::SetAgentMode(AgentMode::Build))
    ));
}

#[test]
fn input_non_shortcut_returns_none() {
    use iced::keyboard::{Key, Modifiers};
    assert!(input::handle_key(Key::Character("z".into()), Modifiers::default()).is_none());
    assert!(input::handle_key(
        Key::Named(iced::keyboard::key::Named::Enter),
        Modifiers::default()
    )
    .is_none());
}

// ── is_korean_friendly ──────────────────────────────────────────

#[test]
fn ko_friendly_known_models() {
    assert!(is_korean_friendly("openai/gpt-4o"));
    assert!(is_korean_friendly("anthropic/claude-3.5-sonnet"));
    assert!(is_korean_friendly("google/gemini-1.5-pro"));
    assert!(is_korean_friendly("qwen/qwen2.5-coder-7b"));
    assert!(is_korean_friendly("meta-llama/llama-3.1-70b-instruct"));
    assert!(is_korean_friendly("upstage/solar-10.7b"));
    assert!(is_korean_friendly("LGAI-EXAONE/EXAONE-3.5-7.8B")); // 대문자도 매칭
    assert!(is_korean_friendly("deepseek/deepseek-v3"));
}

#[test]
fn ko_friendly_negative() {
    assert!(!is_korean_friendly("mistralai/mistral-7b"));
    assert!(!is_korean_friendly("openai/gpt-3.5-turbo"));
    assert!(!is_korean_friendly("starcoder2:7b"));
}

// ── categorize_model ────────────────────────────────────────────

#[test]
fn categorize_coding_models() {
    let cats = categorize_model("qwen/qwen2.5-coder-7b");
    assert!(cats.contains(&ModelCategory::Coding));
}

#[test]
fn categorize_reasoning_models() {
    let cats = categorize_model("deepseek/deepseek-r1");
    assert!(cats.contains(&ModelCategory::Reasoning));
}

#[test]
fn categorize_general_fallback() {
    let cats = categorize_model("mistralai/mistral-7b-instruct");
    assert!(cats.contains(&ModelCategory::General));
}

// ── summarize_tool_result ───────────────────────────────────────

#[test]
fn summarize_write_file_success() {
    let args = r#"{"path":"src/foo.rs","content":"hello"}"#;
    let (summary, success) = summarize_tool_result("write_file", args, "OK: wrote 5 bytes");
    assert!(summary.contains("src/foo.rs"));
    assert!(summary.contains("5 bytes"));
    assert!(success);
}

#[test]
fn summarize_write_file_error_result() {
    let args = r#"{"path":"src/foo.rs","content":"x"}"#;
    let (_, success) = summarize_tool_result("write_file", args, "Error: permission denied");
    assert!(!success);
}

#[test]
fn summarize_run_command_truncates_long_command() {
    let long = "x".repeat(200);
    let args = format!(r#"{{"command":"{}"}}"#, long);
    let (summary, _) = summarize_tool_result("run_command", &args, "OK");
    assert!(summary.starts_with("$ "));
    // command 부분만 60자 제한 (prefix "$ " 포함 62자 이내)
    assert!(summary.len() <= 62);
}

#[test]
fn summarize_unknown_tool() {
    let (summary, success) = summarize_tool_result("foo", "{}", "first line\nsecond line");
    assert_eq!(summary, "first line");
    assert!(success);
}

#[test]
fn summarize_err_marker() {
    let (_, success) = summarize_tool_result("foo", "{}", "[err] something broke");
    assert!(!success);
}

// ── truncate_after_last_user (P4-3 regenerate/edit) ────────────

fn cm(role: &str, content: &str) -> ChatMessage {
    ChatMessage {
        role: role.into(),
        content: Some(content.into()),
        ..Default::default()
    }
}

#[test]
fn truncate_empty_conv() {
    let mut conv: Vec<ChatMessage> = Vec::new();
    truncate_after_last_user(&mut conv);
    assert!(conv.is_empty());
}

#[test]
fn truncate_user_only() {
    let mut conv = vec![cm("user", "hi")];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 1);
    assert_eq!(conv[0].role, "user");
}

#[test]
fn truncate_user_assistant() {
    let mut conv = vec![cm("user", "hi"), cm("assistant", "hello")];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 1);
    assert_eq!(conv[0].content.as_deref(), Some("hi"));
}

#[test]
fn truncate_keeps_last_user_intact() {
    let mut conv = vec![
        cm("user", "first"),
        cm("assistant", "answer1"),
        cm("user", "second"),
    ];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 3); // 마지막이 user면 그대로
    assert_eq!(conv[2].content.as_deref(), Some("second"));
}

#[test]
fn truncate_user_tool_assistant_chain() {
    // user / assistant(tool_calls) / tool / assistant 시퀀스
    let mut conv = vec![
        cm("system", "sys"),
        cm("user", "hi"),
        cm("assistant", "let me check"),
        cm("tool", "result"),
        cm("assistant", "done"),
    ];
    truncate_after_last_user(&mut conv);
    assert_eq!(conv.len(), 2);
    assert_eq!(conv[0].role, "system");
    assert_eq!(conv[1].role, "user");
}

#[test]
fn truncate_no_user_drops_all() {
    let mut conv = vec![cm("system", "sys"), cm("assistant", "lone")];
    truncate_after_last_user(&mut conv);
    assert!(conv.is_empty());
}

// ── last_user_block_idx / last_assistant_block_idx ─────────────

fn ub(id: u64) -> Block {
    Block {
        id,
        body: BlockBody::User(format!("u{}", id)),
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    }
}
fn ab(id: u64) -> Block {
    Block {
        id,
        body: BlockBody::Assistant(iced::widget::text_editor::Content::with_text(&format!(
            "a{}",
            id
        ))),
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    }
}
fn tb(id: u64) -> Block {
    Block {
        id,
        body: BlockBody::ToolResult {
            name: "x".into(),
            summary: "y".into(),
            success: true,
        },
        view_mode: ViewMode::Rendered,
        md_items: Vec::new(),
        model: None,
        apply_candidates: Vec::new(),
    }
}

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
fn persisted_assistant_blocks_default_to_raw_for_selection() {
    let block = persisted_to_block(session::PersistedBlock {
        id: 1,
        role: "assistant".into(),
        content: "selectable answer".into(),
        model: "local".into(),
    });

    assert_eq!(block.view_mode, ViewMode::Raw);
    assert_eq!(block.body.to_text(), "selectable answer");
}

#[test]
fn persisted_user_blocks_keep_rendered_layout() {
    let block = persisted_to_block(session::PersistedBlock {
        id: 1,
        role: "user".into(),
        content: "hello".into(),
        model: String::new(),
    });

    assert_eq!(block.view_mode, ViewMode::Rendered);
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
fn last_user_idx_empty() {
    assert_eq!(last_user_block_idx(&[]), None);
}

#[test]
fn last_user_idx_only_user() {
    let blocks = vec![ub(1)];
    assert_eq!(last_user_block_idx(&blocks), Some(0));
}

#[test]
fn last_user_idx_picks_last() {
    let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
    assert_eq!(last_user_block_idx(&blocks), Some(2));
}

#[test]
fn last_user_idx_no_user() {
    let blocks = vec![ab(1), tb(2)];
    assert_eq!(last_user_block_idx(&blocks), None);
}

#[test]
fn last_assistant_idx_picks_last_assistant() {
    let blocks = vec![ub(1), ab(2), ub(3), ab(4), tb(5)];
    assert_eq!(last_assistant_block_idx(&blocks), Some(3));
}

#[test]
fn last_assistant_idx_no_assistant() {
    let blocks = vec![ub(1), ub(2)];
    assert_eq!(last_assistant_block_idx(&blocks), None);
}

// ── parse_apply_candidates (P4-1) ──────────────────────────────

#[test]
fn apply_empty_markdown() {
    assert!(parse_apply_candidates("").is_empty());
    assert!(parse_apply_candidates("just plain text\nno code blocks").is_empty());
}

#[test]
fn apply_no_path_comment_skipped() {
    let md = "```rust\nfn main() {}\n```\n";
    assert!(parse_apply_candidates(md).is_empty());
}

#[test]
fn apply_rust_path_comment() {
    let md = "```rust\n// path: src/foo.rs\nfn main() {}\n```\n";
    let candidates = parse_apply_candidates(md);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].path, "src/foo.rs");
    assert_eq!(candidates[0].language, "rust");
    assert_eq!(candidates[0].content, "fn main() {}\n");
}

#[test]
fn apply_python_hash_comment() {
    let md = "```python\n# path: scripts/build.py\nprint('hi')\n```\n";
    let candidates = parse_apply_candidates(md);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].path, "scripts/build.py");
    assert_eq!(candidates[0].language, "python");
}

#[test]
fn apply_multiple_blocks_filters_no_path() {
    let md = "intro\n\
                  ```rust\n// path: a.rs\nA\n```\n\
                  some text\n\
                  ```rust\nB without path\n```\n\
                  ```python\n# path: b.py\nB\n```\n";
    let candidates = parse_apply_candidates(md);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].path, "a.rs");
    assert_eq!(candidates[1].path, "b.py");
}

#[test]
fn apply_path_comment_with_extra_spaces() {
    let md = "```rust\n//    path:    src/x.rs   \nbody\n```\n";
    let candidates = parse_apply_candidates(md);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].path, "src/x.rs");
}

#[test]
fn apply_unclosed_fence_ignored() {
    // 닫는 ``` 없으면 후속 처리는 그래도 path만 매칭됐으면 채택할지 결정.
    // 정책: 닫는 fence 없으면 미완성 → skip.
    let md = "```rust\n// path: a.rs\nbody (no closing)\n";
    assert!(parse_apply_candidates(md).is_empty());
}

#[test]
fn apply_no_language_still_works() {
    let md = "```\n// path: x.txt\nhello\n```\n";
    let candidates = parse_apply_candidates(md);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].path, "x.txt");
    assert_eq!(candidates[0].language, "");
}

#[test]
fn apply_first_line_must_be_path() {
    // path 주석이 둘째 줄이면 채택 안 함 (정책: 첫 줄만)
    let md = "```rust\nfn main() {}\n// path: a.rs\n```\n";
    assert!(parse_apply_candidates(md).is_empty());
}

// ── ModelOption Display + provider_label (OpenAICompat rename) ──

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
fn display_openrouter_basic() {
    let m = or_opt("gpt-4o");
    let s = format!("{}", m);
    assert!(s.starts_with("[OR]"), "got: {}", s);
    assert!(s.contains("gpt-4o"));
}

#[test]
fn display_openai_compat_with_label() {
    let m = oai_opt("qwen2.5-coder", "xLLM");
    let s = format!("{}", m);
    assert!(s.starts_with("[xLLM]"), "got: {}", s);
    assert!(s.contains("qwen2.5-coder"));
}

#[test]
fn display_openai_compat_empty_label_defaults_to_local() {
    let m = oai_opt("starcoder", "");
    let s = format!("{}", m);
    assert!(s.starts_with("[Local]"), "got: {}", s);
}

#[test]
fn display_openai_compat_whitespace_label_defaults() {
    let m = oai_opt("foo", "   ");
    let s = format!("{}", m);
    assert!(s.starts_with("[Local]"), "got: {}", s);
}

#[test]
fn display_combined_tags() {
    let mut m = or_opt("claude-3.5-sonnet");
    m.ko_friendly = true;
    m.favorite = true;
    m.context_length = Some(200_000);
    m.prompt_per_million = Some(3.0);
    m.completion_per_million = Some(15.0);
    let s = format!("{}", m);
    assert!(s.contains("[OR]"));
    assert!(s.contains("[KO]"));
    assert!(s.contains("★"));
    assert!(s.contains("200k"));
    assert!(s.contains("$3.00/$15.00"));
}

#[test]
fn display_openai_compat_free_marker() {
    let mut m = oai_opt("local-model", "xLLM");
    m.prompt_per_million = Some(0.0);
    m.completion_per_million = Some(0.0);
    let s = format!("{}", m);
    assert!(s.contains("[xLLM]"));
    assert!(s.contains("free"));
}

// ── InferenceEngine ─────────────────────────────────────────────

#[test]
fn engine_default_ports() {
    assert_eq!(InferenceEngine::TabbyMl.default_port(), 8080);
    assert_eq!(
        InferenceEngine::TabbyApi.default_port(),
        TABBY_API_DEFAULT_PORT
    );
    assert_eq!(InferenceEngine::Ollama.default_port(), 11434);
    assert_eq!(InferenceEngine::XLlm.default_port(), 9000);
    assert_eq!(InferenceEngine::VLlm.default_port(), 9000);
    assert_eq!(InferenceEngine::LlamaServer.default_port(), 9000);
}

#[test]
fn engine_model_namespace_rules() {
    assert!(InferenceEngine::XLlm.shares_model_namespace(InferenceEngine::VLlm));
    assert!(InferenceEngine::VLlm.shares_model_namespace(InferenceEngine::LlamaServer));
    assert!(!InferenceEngine::TabbyMl.shares_model_namespace(InferenceEngine::XLlm));
    assert!(!InferenceEngine::Custom.shares_model_namespace(InferenceEngine::TabbyMl));
    assert!(!InferenceEngine::TabbyMl.shares_model_namespace(InferenceEngine::TabbyApi));
}

#[test]
fn engine_compose_xllm() {
    let cmd = InferenceEngine::XLlm
        .compose_command("C:\\models\\Qwen", 9000)
        .unwrap();
    assert_eq!(cmd[0], "xllm");
    assert_eq!(cmd[1], "serve");
    assert!(cmd.contains(&"--model".to_string()));
    assert!(cmd.contains(&"C:\\models\\Qwen".to_string()));
    assert!(cmd.contains(&"--port".to_string()));
    assert!(cmd.contains(&"9000".to_string()));
}

#[test]
fn engine_compose_vllm() {
    let cmd = InferenceEngine::VLlm
        .compose_command("/path/to/model", 9000)
        .unwrap();
    assert_eq!(cmd[0], "vllm");
    assert_eq!(cmd[1], "serve");
    assert!(cmd.contains(&"/path/to/model".to_string()));
}

#[test]
fn engine_compose_llama_server() {
    let cmd = InferenceEngine::LlamaServer
        .compose_command("/path/model.gguf", 9000)
        .unwrap();
    assert_eq!(cmd[0], "llama-server");
    assert!(cmd.contains(&"-m".to_string()));
    assert!(cmd.contains(&"/path/model.gguf".to_string()));
}

#[test]
fn engine_compose_tabby_uses_repo_id() {
    let cmd = InferenceEngine::TabbyMl
        .compose_command("TabbyML/Qwen2.5-Coder-7B", 8080)
        .unwrap();
    assert_eq!(cmd[0], "tabby");
    assert_eq!(cmd[1], "serve");
    assert!(cmd.contains(&"--chat-model".to_string()));
    assert!(cmd.contains(&"TabbyML/Qwen2.5-Coder-7B".to_string()));
}

#[test]
fn engine_compose_tabbyapi_uses_platform_launcher() {
    let cmd = InferenceEngine::TabbyApi
        .compose_command("C:\\models\\Local-EXL2", TABBY_API_DEFAULT_PORT)
        .unwrap();
    #[cfg(windows)]
    assert_eq!(cmd[0], "Start.bat");
    #[cfg(not(windows))]
    assert_eq!(cmd[0], "./start.sh");
    assert_eq!(cmd[1], "--config");
    assert_eq!(cmd[2], "config.yml");
}

#[test]
fn engine_ollama_no_spawn() {
    // Ollama는 daemon — 이미 떠있다고 가정, spawn 안 함
    assert!(InferenceEngine::Ollama
        .compose_command("any", 11434)
        .is_none());
}

#[test]
fn engine_custom_no_compose() {
    // Custom은 사용자가 직접 명령 입력
    assert!(InferenceEngine::Custom
        .compose_command("any", 9000)
        .is_none());
}

#[test]
fn select_inference_engine_keeps_selection_within_local_namespace() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::SelectInferenceEngine(InferenceEngine::VLlm));
    assert_eq!(app.inference_selected_model, "Qwen--7B");
}

#[test]
fn select_inference_engine_clears_selection_across_namespaces() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::SelectInferenceEngine(InferenceEngine::TabbyMl));
    assert!(app.inference_selected_model.is_empty());
}

#[test]
fn can_start_inference_local_engine_requires_existing_model() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();

    assert!(app.can_start_inference());
}

#[test]
fn can_start_inference_local_engine_rejects_missing_model() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::VLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "missing-model".into();

    assert!(!app.can_start_inference());
}

#[test]
fn start_inference_local_engine_rejects_missing_binary_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();
    let missing_binary = tmp.path().join("missing-xllm.exe");

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();
    app.inference_binary_path = missing_binary.display().to_string();

    let _ = app.update(Message::StartInference);

    assert!(
        app.status.contains("xLLM binary was not found"),
        "got: {}",
        app.status
    );
    assert!(app.inference_pid.is_none());
}

#[test]
fn start_inference_local_engine_reports_missing_binary_inside_directory_override() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Qwen--7B");
    let runtime_dir = tmp.path().join("runtime-dir");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::create_dir_all(&runtime_dir).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = tmp.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();
    app.inference_binary_path = runtime_dir.display().to_string();

    let _ = app.update(Message::StartInference);

    #[cfg(windows)]
    let expected_binary = "xllm.exe";
    #[cfg(not(windows))]
    let expected_binary = "xllm";

    assert!(app.status.contains("is a directory"), "got: {}", app.status);
    assert!(app.status.contains(expected_binary), "got: {}", app.status);
    assert!(app.status.contains(&runtime_dir.display().to_string()));
    assert!(app.inference_pid.is_none());
}

#[test]
fn can_start_inference_tabby_requires_model_id() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyMl;
    app.inference_selected_model = String::new();
    assert!(!app.can_start_inference());

    app.inference_selected_model = "TabbyML/Qwen2.5-Coder-7B".into();
    assert!(app.can_start_inference());
}

#[test]
fn select_downloaded_model_defaults_to_tabbyapi_port() {
    let tmp = tempfile::TempDir::new().unwrap();
    let (mut app, _) = App::new();
    app.model_dir_input = tmp.path().display().to_string();
    let model = tmp.path().join("Local-EXL2");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let _ = app.update(Message::SelectDownloadedModel("Local-EXL2".into()));

    assert_eq!(app.inference_engine, InferenceEngine::TabbyApi);
    assert_eq!(app.inference_port_input, TABBY_API_DEFAULT_PORT.to_string());
    assert_eq!(app.tabby_url_input, "http://localhost:5000");
    assert!(app.inference_selected_model.ends_with("Local-EXL2"));
    if let Some(launcher) = find_tabbyapi_launcher(&default_tabbyapi_runtime_dir()) {
        assert_eq!(app.inference_binary_path, launcher.display().to_string());
        assert!(app.can_start_inference());
    } else {
        assert!(app.inference_binary_path.is_empty());
        assert!(!app.can_start_inference());
    }
    assert!(app.can_attempt_start_inference());
}

#[cfg(windows)]
#[test]
fn find_tabbyapi_launcher_accepts_start_cmd() {
    let tmp = tempfile::TempDir::new().unwrap();
    let launcher = tmp.path().join("Start.cmd");
    std::fs::write(&launcher, "@echo off").unwrap();

    let found = find_tabbyapi_launcher(tmp.path());
    let found = found.expect("expected launcher");
    assert_eq!(found.parent(), Some(tmp.path()));
    let name = found
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    assert!(name.eq_ignore_ascii_case("start.cmd"), "got: {}", name);
}

#[test]
fn selecting_tabbyapi_runtime_sets_provider_endpoint() {
    let (mut app, _) = App::new();
    app.tabby_url_input = "http://localhost:8080".into();
    app.openai_compat_label = "TabbyML".into();

    let _ = app.update(Message::SelectInferenceEngine(InferenceEngine::TabbyApi));

    assert_eq!(app.inference_port_input, TABBY_API_DEFAULT_PORT.to_string());
    assert_eq!(app.tabby_url_input, "http://localhost:5000");
    assert_eq!(app.openai_compat_label, "TabbyAPI");
}

#[test]
fn tabbyapi_port_change_syncs_loopback_provider_url() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_port_input = "5000".into();
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::InferencePortChanged("5001".into()));

    assert_eq!(app.inference_port_input, "5001");
    assert_eq!(app.tabby_url_input, "http://localhost:5001");
}

#[test]
fn tabbyapi_port_change_does_not_override_non_loopback_provider_url() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_port_input = "5000".into();
    app.tabby_url_input = "http://192.168.0.20:5000".into();

    let _ = app.update(Message::InferencePortChanged("5001".into()));

    assert_eq!(app.inference_port_input, "5001");
    assert_eq!(app.tabby_url_input, "http://192.168.0.20:5000");
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
fn saved_shared_model_prefers_tabby_when_tabby_url_is_set() {
    let (mut app, _) = App::new();
    app.selected_model = Some("shared-model".into());
    app.selected_model_provider = None;
    app.tabby_url_input = "http://localhost:5000".into();
    app.model_options = vec![or_opt("shared-model")];

    let _ = app.update(Message::TabbyModelsLoaded(Ok(vec!["shared-model".into()])));

    assert_eq!(app.selected_model_provider, Some(LlmProvider::OpenAICompat));
}

#[test]
fn tabby_models_loaded_selects_first_local_model() {
    let (mut app, _) = App::new();
    app.model_options.clear();
    app.selected_model = Some("openrouter-model".into());
    app.openai_compat_label = "TabbyAPI".into();

    let _ = app.update(Message::TabbyModelsLoaded(Ok(vec![
        "tabby-a".into(),
        "tabby-b".into(),
    ])));

    assert_eq!(app.selected_model.as_deref(), Some("tabby-a"));
    assert!(app
        .model_options
        .iter()
        .any(|o| { o.id == "tabby-a" && o.provider == LlmProvider::OpenAICompat }));
}

#[test]
fn openrouter_models_loaded_preserves_existing_tabby_selection() {
    let (mut app, _) = App::new();
    app.model_options = vec![ModelOption {
        id: "tabby-a".into(),
        provider: LlmProvider::OpenAICompat,
        provider_label: "TabbyAPI".into(),
        ko_friendly: false,
        favorite: false,
        context_length: None,
        prompt_per_million: Some(0.0),
        completion_per_million: Some(0.0),
    }];
    app.selected_model = Some("tabby-a".into());
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::ModelsLoaded(Ok(vec![OpenRouterModel {
        id: "openrouter-a".into(),
        name: None,
        context_length: None,
        pricing: None,
    }])));

    assert_eq!(app.selected_model.as_deref(), Some("tabby-a"));
    assert!(app
        .model_options
        .iter()
        .any(|o| { o.id == "tabby-a" && o.provider == LlmProvider::OpenAICompat }));
}

#[test]
fn openrouter_models_loaded_waits_for_tabby_when_saved_selection_not_loaded_yet() {
    let (mut app, _) = App::new();
    app.model_options.clear();
    app.selected_model = Some("tabby-a".into());
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::ModelsLoaded(Ok(vec![OpenRouterModel {
        id: "openrouter-a".into(),
        name: None,
        context_length: None,
        pricing: None,
    }])));

    assert_eq!(app.selected_model.as_deref(), Some("tabby-a"));
}

#[test]
fn tabbyapi_start_button_can_show_missing_launcher_error() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_selected_model = r"C:\models\Local-EXL2".into();
    app.inference_binary_path.clear();

    assert!(!app.can_start_inference());
    assert!(app.can_attempt_start_inference());

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(
        app.status.contains("TabbyAPI 런타임"),
        "got: {}",
        app.status
    );
    assert!(app.status.contains("먼저 설치"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_start_rejects_tabbyml_binary_with_specific_guidance() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_selected_model = r"C:\models\Local-EXL2".into();
    app.inference_binary_path = r"C:\tools\tabby.exe".into();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
    assert!(app.status.contains("EXL2"), "got: {}", app.status);
    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_binary_picker_rejects_tabby_cli_cmd() {
    let tmp = tempfile::TempDir::new().unwrap();
    let picked = tmp.path().join("tabby.cmd");
    std::fs::write(&picked, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path.clear();

    let _ = app.update(Message::InferenceBinaryPicked(Some(picked)));

    assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
    assert!(app.inference_binary_path.is_empty());
}

#[test]
fn tabbyapi_binary_picker_rejects_tabby_cli_bat() {
    let tmp = tempfile::TempDir::new().unwrap();
    let picked = tmp.path().join("tabby.bat");
    std::fs::write(&picked, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path.clear();

    let _ = app.update(Message::InferenceBinaryPicked(Some(picked)));

    assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
    assert!(app.status.contains("tabby.bat"), "got: {}", app.status);
    assert!(app.inference_binary_path.is_empty());
}

#[test]
fn tabbyapi_binary_picker_rejects_wrong_script_name() {
    let tmp = tempfile::TempDir::new().unwrap();
    let picked = tmp.path().join("launcher.bat");
    std::fs::write(&picked, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path.clear();

    let _ = app.update(Message::InferenceBinaryPicked(Some(picked)));

    assert!(
        app.status.contains("파일명이 올바르지"),
        "got: {}",
        app.status
    );
    assert!(app.inference_binary_path.is_empty());
}

#[test]
fn tabbyapi_binary_picker_accepts_start_bat() {
    let tmp = tempfile::TempDir::new().unwrap();
    let picked = tmp.path().join("Start.bat");
    std::fs::write(&picked, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path.clear();

    let _ = app.update(Message::InferenceBinaryPicked(Some(picked.clone())));

    assert_eq!(app.inference_binary_path, picked.display().to_string());
    assert!(
        app.status.contains("script 경로 저장됨"),
        "got: {}",
        app.status
    );
}

#[cfg(windows)]
#[test]
fn tabbyapi_binary_picker_accepts_start_cmd() {
    let tmp = tempfile::TempDir::new().unwrap();
    let picked = tmp.path().join("Start.cmd");
    std::fs::write(&picked, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path.clear();

    let _ = app.update(Message::InferenceBinaryPicked(Some(picked.clone())));

    assert_eq!(app.inference_binary_path, picked.display().to_string());
    assert!(
        app.status.contains("script 경로 저장됨"),
        "got: {}",
        app.status
    );
}

#[test]
fn tabbyapi_start_rejects_tabbyml_cli_without_extension() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_selected_model = r"C:\models\Local-EXL2".into();
    app.inference_binary_path = r"C:\tools\tabby".into();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
    assert!(app.status.contains("EXL2"), "got: {}", app.status);
    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_start_rejects_tabbyml_cli_cmd_launcher() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_selected_model = r"C:\models\Local-EXL2".into();
    app.inference_binary_path = r"C:\tools\tabby.cmd".into();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
    assert!(app.status.contains("tabby.cmd"), "got: {}", app.status);
    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_start_rejects_tabbyml_cli_bat_launcher() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_selected_model = r"C:\models\Local-EXL2".into();
    app.inference_binary_path = r"C:\tools\tabby.bat".into();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("TabbyML CLI"), "got: {}", app.status);
    assert!(app.status.contains("tabby.bat"), "got: {}", app.status);
    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_start_rejects_missing_launcher_file_with_explicit_message() {
    let tmp = tempfile::TempDir::new().unwrap();
    let missing = tmp.path().join("Start.bat");

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path = missing.display().to_string();
    app.inference_selected_model.clear();

    let _ = app.update(Message::StartInference);

    assert!(
        app.status.contains("찾을 수 없습니다"),
        "got: {}",
        app.status
    );
    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_start_rejects_launcher_directory_path() {
    let tmp = tempfile::TempDir::new().unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path = tmp.path().display().to_string();
    app.inference_selected_model.clear();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("폴더입니다"), "got: {}", app.status);
    assert!(app.status.contains("Start.bat"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn tabbyapi_can_start_with_launcher_without_model_path() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_selected_model.clear();
    app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();

    assert!(app.can_start_inference());
    assert!(app.can_attempt_start_inference());
}

#[test]
fn tabbyapi_connection_error_prompts_for_launcher_when_missing() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.tabby_url_input = "http://localhost:5000".into();
    app.inference_binary_path.clear();

    let msg = app.compose_tabby_connection_error("operation timed out");

    assert!(msg.contains("TabbyAPI 서버"), "got: {}", msg);
    assert!(msg.contains("Start.bat"), "got: {}", msg);
    assert!(msg.contains("start.sh"), "got: {}", msg);
    assert!(msg.contains("main.py"), "got: {}", msg);
}

#[test]
fn tabbyapi_connection_error_points_to_runtime_logs_when_launcher_is_set() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.tabby_url_input = "http://localhost:5000".into();
    app.inference_port_input = "5000".into();
    app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();

    let msg = app.compose_tabby_connection_error("error sending request: Connection refused");

    assert!(msg.contains("TabbyAPI 서버"), "got: {}", msg);
    assert!(msg.contains("로그"), "got: {}", msg);
    assert!(msg.contains("http://localhost:5000"), "got: {}", msg);
}

#[test]
fn tabbyapi_connection_error_detects_runtime_port_mismatch() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.tabby_url_input = "http://localhost:8080".into();
    app.inference_port_input = "5000".into();
    app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();

    let msg = app.compose_tabby_connection_error("operation timed out");

    assert!(msg.contains("Provider URL"), "got: {}", msg);
    assert!(msg.contains("5000"), "got: {}", msg);
    assert!(msg.contains("http://localhost:5000"), "got: {}", msg);
}

#[test]
fn tabby_models_loaded_error_decrements_auto_retry_while_runtime_alive() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_pid = Some(42);
    app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();
    app.tabby_url_input = "http://localhost:5000".into();
    app.tabby_connect_retry_left = 2;

    let _ = app.update(Message::TabbyModelsLoaded(
        Err("operation timed out".into()),
    ));

    assert_eq!(app.tabby_connect_retry_left, 1);
    assert!(app.status.contains("자동 재시도"), "got: {}", app.status);
    app.inference_pid = None;
}

#[test]
fn tabby_models_loaded_error_without_retry_budget_reports_failure() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_pid = Some(42);
    app.inference_binary_path = r"C:\TabbyAPI\Start.bat".into();
    app.tabby_url_input = "http://localhost:5000".into();
    app.tabby_connect_retry_left = 0;

    let _ = app.update(Message::TabbyModelsLoaded(
        Err("operation timed out".into()),
    ));

    assert_eq!(app.tabby_connect_retry_left, 0);
    assert!(app.status.contains("연결 실패"), "got: {}", app.status);
    app.inference_pid = None;
}

#[test]
fn stale_tabby_retry_message_is_ignored() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 10;
    app.status = "keep".into();

    let _ = app.update(Message::FetchTabbyModelsRetry(9));

    assert_eq!(app.status, "keep");
    assert_eq!(app.tabby_retry_generation, 10);
}

#[test]
fn manual_tabby_fetch_bumps_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 7;
    app.tabby_url_input = "http://localhost:5000".into();

    let _ = app.update(Message::FetchTabbyModels);

    assert_eq!(app.tabby_retry_generation, 8);
}

#[test]
fn tabby_url_change_invalidates_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 11;
    app.tabby_connect_retry_left = 2;

    let _ = app.update(Message::TabbyUrlChanged(
        "http://localhost:5001".to_string(),
    ));

    assert_eq!(app.tabby_retry_generation, 12);
    assert_eq!(app.tabby_connect_retry_left, 0);
}

#[test]
fn tabby_token_change_invalidates_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 5;
    app.tabby_connect_retry_left = 1;

    let _ = app.update(Message::TabbyTokenChanged("secret".to_string()));

    assert_eq!(app.tabby_retry_generation, 6);
    assert_eq!(app.tabby_connect_retry_left, 0);
}

#[test]
fn clear_tabby_invalidates_retry_generation() {
    let (mut app, _) = App::new();
    app.tabby_retry_generation = 3;
    app.tabby_connect_retry_left = 2;
    app.tabby_url_input = "http://localhost:5000".into();
    app.tabby_token_input = "secret".into();

    let _ = app.update(Message::ClearTabby);

    assert_eq!(app.tabby_retry_generation, 4);
    assert_eq!(app.tabby_connect_retry_left, 0);
    assert!(app.tabby_url_input.is_empty());
    assert!(app.tabby_token_input.is_empty());
}

#[cfg(windows)]
#[test]
fn tabbyapi_bat_launcher_runs_via_cmd_in_script_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    let script = tmp.path().join("Start.bat");
    std::fs::write(&script, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyApi;
    app.inference_binary_path = script.display().to_string();

    let (program, args, work_dir) = app.resolve_runtime_spawn_command(
        "Start.bat".into(),
        vec!["--config".into(), "config.yml".into()],
    );

    assert_eq!(program, "cmd.exe");
    assert_eq!(
        args,
        vec![
            "/C".to_string(),
            "Start.bat".to_string(),
            "--config".to_string(),
            "config.yml".to_string()
        ]
    );
    assert_eq!(work_dir.as_deref(), Some(tmp.path()));
}

#[test]
fn non_tabby_runtime_ignores_tabbyapi_launcher_override() {
    let tmp = tempfile::TempDir::new().unwrap();
    let tabby_dir = tmp.path().join("tabbyAPI");
    std::fs::create_dir_all(&tabby_dir).unwrap();
    let launcher = tabby_dir.join("Start.bat");
    std::fs::write(&launcher, "@echo off").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_binary_path = launcher.display().to_string();

    let (program, args, work_dir) =
        app.resolve_runtime_spawn_command("xllm".into(), vec!["serve".into()]);

    assert_eq!(program, "xllm");
    assert_eq!(args, vec!["serve".to_string()]);
    assert!(work_dir.is_none());
}

#[test]
fn non_tabby_runtime_keeps_custom_binary_override() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_binary_path = r"C:\tools\xllm.exe".into();

    let (program, args, work_dir) =
        app.resolve_runtime_spawn_command("xllm".into(), vec!["serve".into()]);

    assert_eq!(program, r"C:\tools\xllm.exe");
    assert_eq!(args, vec!["serve".to_string()]);
    assert!(work_dir.is_none());
}

#[test]
fn non_tabby_runtime_directory_override_resolves_engine_binary() {
    let tmp = tempfile::TempDir::new().unwrap();
    let runtime_dir = tmp.path().join("runtime");
    std::fs::create_dir_all(&runtime_dir).unwrap();
    #[cfg(windows)]
    let bin = runtime_dir.join("xllm.exe");
    #[cfg(not(windows))]
    let bin = runtime_dir.join("xllm");
    std::fs::write(&bin, "bin").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_binary_path = runtime_dir.display().to_string();

    let (program, args, work_dir) =
        app.resolve_runtime_spawn_command("xllm".into(), vec!["serve".into()]);

    assert_eq!(program, bin.display().to_string());
    assert_eq!(args, vec!["serve".to_string()]);
    assert!(work_dir.is_none());
}

#[test]
fn tabbyapi_config_points_to_selected_model_and_local_port() {
    let runtime = tempfile::TempDir::new().unwrap();
    let launcher = runtime.path().join("start.bat");
    std::fs::write(&launcher, "@echo off").unwrap();
    let models = tempfile::TempDir::new().unwrap();
    let model = models.path().join("Local-EXL2");
    std::fs::create_dir_all(&model).unwrap();

    let config = write_tabbyapi_config_for_launcher(
        &launcher.display().to_string(),
        &model.display().to_string(),
        TABBY_API_DEFAULT_PORT,
    )
    .unwrap();
    let text = std::fs::read_to_string(config).unwrap();

    assert!(text.contains("port: 5000"), "got: {}", text);
    assert!(text.contains("disable_auth: true"), "got: {}", text);
    assert!(text.contains("model_name: 'Local-EXL2'"), "got: {}", text);
    assert!(
        text.contains(&format!("model_dir: '{}'", models.path().display())),
        "got: {}",
        text
    );
}

#[test]
fn start_inference_tabby_rejects_local_exl2_path() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Local-EXL2");
    std::fs::create_dir_all(&model).unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyMl;
    app.inference_selected_model = model.display().to_string();

    let _ = app.update(Message::StartInference);

    assert!(app.status.contains("TabbyAPI"), "got: {}", app.status);
    assert!(app.inference_pid.is_none());
}

#[test]
fn can_start_inference_custom_requires_non_empty_command() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::Custom;
    app.inference_command_input = "   ".into();
    assert!(!app.can_start_inference());

    app.inference_command_input = "xllm serve --model X".into();
    assert!(app.can_start_inference());
}

#[test]
fn can_start_inference_ollama_always_true() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::Ollama;
    app.inference_selected_model = String::new();
    app.inference_command_input = String::new();
    assert!(app.can_start_inference());
}

#[test]
fn model_dir_changed_clears_stale_local_model_selection() {
    let old_dir = tempfile::TempDir::new().unwrap();
    let new_dir = tempfile::TempDir::new().unwrap();
    let model = old_dir.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.model_dir_input = old_dir.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::ModelDirChanged(
        new_dir.path().display().to_string(),
    ));
    assert!(app.inference_selected_model.is_empty());
}

#[test]
fn model_dir_changed_keeps_selection_for_tabby_engine() {
    let new_dir = tempfile::TempDir::new().unwrap();
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::TabbyMl;
    app.inference_selected_model = "TabbyML/Qwen2.5-Coder-7B".into();

    let _ = app.update(Message::ModelDirChanged(
        new_dir.path().display().to_string(),
    ));
    assert_eq!(app.inference_selected_model, "TabbyML/Qwen2.5-Coder-7B");
}

#[test]
fn model_dir_picked_clears_stale_local_model_selection() {
    let old_dir = tempfile::TempDir::new().unwrap();
    let new_dir = tempfile::TempDir::new().unwrap();
    let model = old_dir.path().join("Qwen--7B");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::LlamaServer;
    app.model_dir_input = old_dir.path().display().to_string();
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::ModelDirPicked(Some(new_dir.path().to_path_buf())));
    assert!(app.inference_selected_model.is_empty());
}

#[test]
fn model_dir_picked_none_keeps_selection() {
    let (mut app, _) = App::new();
    app.inference_engine = InferenceEngine::XLlm;
    app.inference_selected_model = "Qwen--7B".into();

    let _ = app.update(Message::ModelDirPicked(None));
    assert_eq!(app.inference_selected_model, "Qwen--7B");
}

// ── list_downloaded_models ──────────────────────────────────────

#[test]
fn list_models_empty_dir() {
    let tmp = tempfile::TempDir::new().unwrap();
    assert!(list_downloaded_models(tmp.path()).is_empty());
}

#[test]
fn downloaded_exl2_preset_folder_accepts_same_model_bpw_variant() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("config.json"), "{}").unwrap();
    std::fs::write(model.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-4.0bpw")
    );
}

#[test]
fn resolve_tabbyapi_model_dir_accepts_single_nested_child() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Qwen2.5-Coder-7B-Instruct-exl2-4.0bpw");
    let nested = root.join("model");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("config.json"), "{}").unwrap();
    std::fs::write(nested.join("model.safetensors"), "x").unwrap();

    let resolved = resolve_tabbyapi_model_dir(&root).expect("expected nested model dir");
    assert_eq!(resolved, nested);
}

#[test]
fn resolve_tabbyapi_model_dir_for_folder_prefers_matching_bpw_child() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
    let child_35 = root.join("3.5bpw");
    let child_40 = root.join("4.0bpw");
    std::fs::create_dir_all(&child_35).unwrap();
    std::fs::create_dir_all(&child_40).unwrap();
    std::fs::write(child_35.join("config.json"), "{}").unwrap();
    std::fs::write(child_35.join("model.safetensors"), "x").unwrap();
    std::fs::write(child_40.join("config.json"), "{}").unwrap();
    std::fs::write(child_40.join("model.safetensors"), "x").unwrap();

    let resolved = resolve_tabbyapi_model_dir_for_folder(&root, "Llama-3.2-3B-Instruct-3.5bpw")
        .expect("expected bpw-matched nested model dir");
    assert_eq!(resolved, child_35);
}

#[test]
fn downloaded_exl2_preset_folder_accepts_nested_model_layout() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    let nested = root.join("weights");
    std::fs::create_dir_all(&nested).unwrap();
    std::fs::write(nested.join("config.json"), "{}").unwrap();
    std::fs::write(nested.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-4.0bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_accepts_root_with_multiple_nested_variants() {
    let tmp = tempfile::TempDir::new().unwrap();
    let root = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
    let child_35 = root.join("3.5bpw");
    let child_40 = root.join("4.0bpw");
    std::fs::create_dir_all(&child_35).unwrap();
    std::fs::create_dir_all(&child_40).unwrap();
    std::fs::write(child_35.join("config.json"), "{}").unwrap();
    std::fs::write(child_35.join("model.safetensors"), "x").unwrap();
    std::fs::write(child_40.join("config.json"), "{}").unwrap();
    std::fs::write(child_40.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-3.5bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_exact_match_wins() {
    let tmp = tempfile::TempDir::new().unwrap();
    let exact = tmp.path().join("Llama-3.2-3B-Instruct-3.5bpw");
    let other = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    std::fs::create_dir_all(&exact).unwrap();
    std::fs::create_dir_all(&other).unwrap();
    std::fs::write(exact.join("config.json"), "{}").unwrap();
    std::fs::write(exact.join("model.safetensors"), "x").unwrap();
    std::fs::write(other.join("config.json"), "{}").unwrap();
    std::fs::write(other.join("model.safetensors"), "x").unwrap();

    assert_eq!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .as_deref(),
        Some("Llama-3.2-3B-Instruct-3.5bpw")
    );
}

#[test]
fn downloaded_exl2_preset_folder_avoids_ambiguous_variants() {
    let tmp = tempfile::TempDir::new().unwrap();
    let a = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    let b = tmp.path().join("Llama-3.2-3B-Instruct-5.0bpw");
    std::fs::create_dir_all(&a).unwrap();
    std::fs::create_dir_all(&b).unwrap();
    std::fs::write(a.join("config.json"), "{}").unwrap();
    std::fs::write(a.join("model.safetensors"), "x").unwrap();
    std::fs::write(b.join("config.json"), "{}").unwrap();
    std::fs::write(b.join("model.safetensors"), "x").unwrap();

    assert!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .is_none()
    );
}

#[test]
fn list_models_empty_path_returns_empty() {
    assert!(list_downloaded_models(std::path::Path::new("")).is_empty());
}

#[test]
fn resolve_user_path_expands_tilde() {
    if let Some(home) = dirs::home_dir() {
        assert_eq!(resolve_user_path("~"), home);
        assert_eq!(resolve_user_path("~/models"), home.join("models"));
        assert_eq!(resolve_user_path("~\\models"), home.join("models"));
    }
}

#[test]
fn list_models_returns_subdirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let qwen = tmp.path().join("Qwen--Qwen2.5-Coder-7B");
    std::fs::create_dir_all(&qwen).unwrap();
    std::fs::write(qwen.join("config.json"), "{}").unwrap();
    std::fs::write(qwen.join("model.safetensors"), "x").unwrap();
    let solar = tmp.path().join("upstage--SOLAR-10.7B");
    std::fs::create_dir_all(&solar).unwrap();
    std::fs::write(solar.join("model.safetensors"), "x").unwrap();
    // 파일은 무시 (디렉토리 아님)
    std::fs::write(tmp.path().join("ignore.txt"), "x").unwrap();
    let mut models = list_downloaded_models(tmp.path());
    models.sort();
    assert_eq!(models.len(), 2);
    assert!(models[0].contains("Qwen") || models[1].contains("Qwen"));
}

#[test]
fn list_models_are_sorted() {
    let tmp = tempfile::TempDir::new().unwrap();
    let zulu = tmp.path().join("zulu-model");
    let alpha = tmp.path().join("alpha-model");
    std::fs::create_dir_all(&zulu).unwrap();
    std::fs::create_dir_all(&alpha).unwrap();
    std::fs::write(zulu.join("model.safetensors"), "x").unwrap();
    std::fs::write(alpha.join("model.safetensors"), "x").unwrap();

    let models = list_downloaded_models(tmp.path());
    assert_eq!(
        models,
        vec!["alpha-model".to_string(), "zulu-model".to_string()]
    );
}

#[test]
fn list_models_skips_empty_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    std::fs::create_dir_all(tmp.path().join("empty")).unwrap();
    // 빈 폴더는 모델 아님 — skip
    assert!(list_downloaded_models(tmp.path()).is_empty());
}

#[test]
fn list_models_skips_metadata_only_dirs() {
    let tmp = tempfile::TempDir::new().unwrap();
    let model = tmp.path().join("Llama-3.2-3B-Instruct-4.0bpw");
    std::fs::create_dir_all(&model).unwrap();
    std::fs::write(model.join("README.md"), "x").unwrap();
    std::fs::write(model.join(".gitattributes"), "x").unwrap();

    assert!(list_downloaded_models(tmp.path()).is_empty());
    assert!(
        downloaded_exl2_preset_folder(&tmp.path().display().to_string(), &EXL2_PRESETS[1])
            .is_none()
    );
}

// ── extract_mention_query ───────────────────────────────────────

#[test]
fn mention_query_basic() {
    // '@' 뒤에 공백 없으면 Some, 있으면 None
    assert_eq!(extract_mention_query("fix @main"), Some("main"));
    assert_eq!(extract_mention_query("fix @main "), None); // '@main' 이후 공백
    assert_eq!(extract_mention_query("@src/lib"), Some("src/lib"));
    assert_eq!(extract_mention_query("no at sign"), None);
    assert_eq!(extract_mention_query("@"), Some(""));
}

#[test]
fn mention_query_last_at_wins() {
    // 마지막 '@' 기준으로 query 추출
    assert_eq!(extract_mention_query("@foo @bar"), Some("bar")); // 마지막 '@bar' 뒤 공백 없음
    assert_eq!(extract_mention_query("@foo @bar "), None); // 마지막 '@bar' 뒤 공백 있음
    assert_eq!(extract_mention_query("email@ex.com @file"), Some("file")); // 마지막 '@file'
}

// ── fuzzy_match_paths ───────────────────────────────────────────

#[test]
fn fuzzy_match_empty_query_returns_all() {
    let paths: Vec<PathBuf> = vec![PathBuf::from("src/main.rs"), PathBuf::from("src/tools.rs")];
    let result = fuzzy_match_paths(&paths, "", 10);
    assert_eq!(result.len(), 2);
}

#[test]
fn fuzzy_match_filters_by_query() {
    let paths: Vec<PathBuf> = vec![
        PathBuf::from("src/main.rs"),
        PathBuf::from("src/tools.rs"),
        PathBuf::from("Cargo.toml"),
    ];
    let result = fuzzy_match_paths(&paths, "tool", 10);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0], PathBuf::from("src/tools.rs"));
}

#[test]
fn fuzzy_match_respects_max_results() {
    let paths: Vec<PathBuf> = (0..20)
        .map(|i| PathBuf::from(format!("file{i}.rs")))
        .collect();
    let result = fuzzy_match_paths(&paths, "file", 5);
    assert_eq!(result.len(), 5);
}

// ── build_file_context ──────────────────────────────────────────

#[test]
fn build_file_context_single() {
    let files = vec![(PathBuf::from("src/main.rs"), "fn main() {}".to_string())];
    let ctx = build_file_context(&files);
    assert!(ctx.contains("src/main.rs"));
    assert!(ctx.contains("fn main() {}"));
    assert!(ctx.starts_with("```"));
}

#[test]
fn build_file_context_multi_separator() {
    let files = vec![
        (PathBuf::from("a.rs"), "aaa".to_string()),
        (PathBuf::from("b.rs"), "bbb".to_string()),
    ];
    let ctx = build_file_context(&files);
    assert!(ctx.contains("\n\n"));
    assert!(ctx.contains("aaa"));
    assert!(ctx.contains("bbb"));
}

// ── ChatChunk Done / Error handler ─────────────────────────────

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
