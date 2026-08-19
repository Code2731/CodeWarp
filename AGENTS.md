# CodeWarp — Agent Context

## Project
- Rust MSRV 1.90.0, Edition 2024, Iced 0.14, serde 1.0.228 (derive + rc)
- `cargo clippy` before any commit; pre-commit `cargo fmt --check`; pre-push `cargo fmt --check && cargo check && cargo test --all-targets -- --test-threads=1`
- Test baseline: run `cargo test --all-targets -- --test-threads=1`; the 2026-08-19 verification passed 679 unit tests, ignored 11 (including the Linux PTY Ctrl+C manual-QA case), plus 1 external integration smoke test. Zero clippy warnings (strict).

## Key Conventions
- `Message` derives `Clone` (required for `key_binding` Fn closure)
- `mouse_area` with `.on_enter(Message)` / `.on_exit(Message)` for hover effects
- `on_press` takes `Message` directly (not closure) — closures create unique types breaking `row!` homogeneity
- Style closures that capture `self` must extract a `bool`/value first (`let is_hovered = ...`) to avoid `Renderer` inference failures
- `update_dispatch_*.rs` files are `include!()`'d from `update.rs`
- `Padding` uses builder pattern (`.top()`, `.bottom()`, `.left()`, `.right()`); no `[u16; 4]` array
- `PersistedSessionData.conversation` is `Arc<Vec<ChatMessage>>`
- `App::try_persist(result, context)` sets `self.status` on error
- `SkeletonTick` fires every 600ms during streaming; drives cursor blink + accent bar pulse

## Hover Effects (mouse_area + Option<T> pattern)
| Feature | Message | State | Style |
|---|---|---|---|
| Block hover | `BlockHovered(Option<u64>)` | `hovered_block` | primary border glow |
| Session hover | `SessionHovered(Option<u64>)` | `hovered_session` | primary bg 8% |
| Context file hover | `ContextHovered(Option<usize>)` | `hovered_context_idx` | primary border + shadow |
| Settings tab hover | `SettingsTabHovered(Option<SettingsTab>)` | `hovered_settings_tab` | primary bg 6% |
| Palette hover | `PaletteHovered(Option<usize>)` | `hovered_palette_idx` | primary bg 6% |
| Confirm card hover | `ConfirmCardHovered(Option<usize>)` | `hovered_confirm_idx` | primary bg 8% + border |
| Attach chip hover | `AttachChipHovered(Option<usize>)` | `hovered_attach_idx` | primary bg 12% + border |
| Shortcut hint hover | `ShortcutHintHovered(Option<usize>)` | `hovered_shortcut_idx` | primary bg 8% |
| PTY header hover | `PtyPanelHovered(bool)` | `hovered_pty` | primary bg 6% |
| Code block hover | `CodeBlockHovered(u64, bool)` | `hovered_code_blocks: HashSet<u64>` | show copy button |

## Refactoring: Functions Extracted (all #[allow(clippy::too_many_lines)] removed)
- `src/view/settings/provider.rs`: `view_provider_tab` → `view_openrouter_section` + `view_tabby_endpoint_section` + `view_tabby_presets`
- `src/view/chat/stream.rs`: `view_stream` → `view_mode_label` + `view_slash_hint` + `view_input_action_btn` + `view_chat_editor` + `view_input_hint` + `view_compare_diff`
- `src/view/sidebar/context.rs`: `view_sidebar_context_area` → `view_context_quota_label` + `view_context_actions` + `view_context_header` + `view_context_empty` + `view_context_files`
- `src/view/sidebar/mod.rs`: `view_sidebar` → `view_active_session_label` + `view_session_list_empty` + `view_session_trailing` + `view_session_row_content` + `view_session_row` + `view_sidebar_body` + `view_resize_row`
- `src/view/view_confirm.rs`: `view_inline_confirm` → `view_confirm_card` + `confirm_panel_style` free fn
- `src/view/settings/mod.rs`: `view_settings` → `settings_tab_health` + `settings_health_for_tab` + `view_settings_header` + `view_active_section`
- `src/view/mod.rs`: `view` → `view_main_row` + `view_overlay` + `view_toast`
- `src/view/settings/models.rs`: `view_model_manager` → `view_model_local_state` + `view_model_dir_row` + `view_model_token_section` + `view_model_download_row` + `view_model_download_progress` (cast_precision_loss remains)
- `src/hf/mod.rs`: `download_repo` → `init_download` async fn + `DownloadSetup` struct

## Theme Presets
- 5 presets in `src/session/theme.rs`: Default Dark, Nord, Dracula, Monokai, Catppuccin
- `ThemePreset { name, background, primary, text }` struct + `theme_presets()` free fn
- `ApplyThemePreset(usize)` message writes to both `theme_config` and `theme_hex_inputs`
- 3-color swatch preview per preset in `src/view/settings/theme.rs`

## Relevant Files
- `src/message.rs` — all Message variants
- `src/state/mod.rs` — UiState struct with all hover fields
- `src/state/state_new.rs` — UiState initialization
- `src/update_dispatch_ui.rs` — hover handlers
- `src/view/ui/styles.rs` — `context_item_style`, `with_alpha`
- `src/view/settings/health.rs` — settings tab bar
- `src/view/view_palette.rs` — command palette
- `src/view/sidebar/context.rs` — context files
- `src/view/sidebar/mod.rs` — session list
- `src/view/chat/block_item.rs` — block cards
- `src/view/chat/mention.rs` — mention popup + attach chips
- `src/view/chat/empty.rs` — empty state + shortcut hints
- `src/view/view_confirm.rs` — write confirm panel
- `src/view/pty.rs` — terminal panel
- `src/view/view_viewer.rs` — code block markdown viewer

## Hover追加 방법 (recipe)
1. Add variant to `Message` enum in `src/message.rs` (e.g. `FooHovered(Option<usize>)`)
2. Add field to `UiState` in `src/state/mod.rs` (e.g. `hovered_foo: Option<usize>`)
3. Initialize to `None` in `src/state/state_new.rs`
4. Add handler arm in `src/update_dispatch_ui.rs` (set field, `Some(Task::none())`)
5. In view function: wrap element in `mouse_area(container(...).style(move |t| {...}))`
   - Extract `is_hovered` bool BEFORE the style closure to avoid `Renderer` inference issues
   - Use `Color::from_rgba(p.primary.base.color.r, ..., ..., alpha)` for consistent primary tint
