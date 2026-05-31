# Update Pipeline

CodeWarp의 UI 이벤트 처리 파이프라인.

## Architecture

Iced의 Elm 아키텍처 기반:

```
User Action → Message → App::update() → Task → App::view()
                ↓
        match message {
            simple  → helper method → Task::none()
            async   → Task::perform → Message(result) → update again
            stream  → Task::run     → Message(chunk)  → update again
        }
```

## Message Dispatch Flow

`src/update.rs`의 `App::update()`는 모든 `Message` 변형을 라우팅합니다.

### Arm 분류

| 분류 | 패턴 | 예시 |
|------|------|------|
| UI state helper | `self.helper()` | `OpenSettings`, `ToggleFavorite`, `CwdPicked` |
| Async trigger | `Task::perform(async, msg)` | `SaveKey`, `StartHfDownload` |
| Async result | `Result → status update` | `KeySaved`, `HfTokenSaved` |
| Stream management | `Task::run(stream, msg)` | `StartInference` |
| Session lifecycle | complex state mutation | `NewChat`, `SwitchSession` |

### Helper Extraction Pattern

순수 UI/state 변경 arms는 helper 메서드로 분리됩니다:

```rust
// Match arm (dispatch only)
Message::ToggleFavorite => self.toggle_favorite(),

// Helper (state transition logic)
fn toggle_favorite(&mut self) -> Task<Message> {
    if let Some(id) = &self.selected_model {
        // ... state mutation ...
    }
    Task::none()
}
```

**Extraction criteria:**
- Pure state mutation (no `Task::perform` or `Task::run`)
- No async I/O or runtime spawn
- No complex control flow with early returns from multiple branches
- Helper returns `Task<Message>` (usually `Task::none()`)

### Async Pattern

Async operations follow a two-arm pattern:

```rust
// Arm 1: Start async work
Message::SaveKey => {
    let key = self.key_input.clone();
    self.busy = true;
    Task::perform(
        async move { keystore::write_api_key(&key) },
        Message::KeySaved,
    )
}

// Arm 2: Handle result
Message::KeySaved(result) => {
    self.busy = false;
    match result {
        Ok(()) => { /* success state */ }
        Err(e) => { self.status = format!("Error: {}", e); }
    }
    Task::none()
}
```

### Stream Pattern

Chat streams use `Task::run` for continuous token delivery:

```rust
Message::Send => {
    // ... setup conversation, blocks ...
    Task::run(
        provider.chat_stream(conversation, model, tools),
        Message::ChatChunk,
    )
}

Message::ChatChunk(event) => {
    match event {
        ChatEvent::Token(text) => { /* append to assistant block */ }
        ChatEvent::Done => { /* finalize */ }
        ChatEvent::Error(msg) => { /* handle */ }
    }
    Task::none()
}
```

## Helper Organization

Helpers in `src/update.rs` are grouped by domain with section comments:

```
// ── Settings helpers ──────────────────────
open_settings_overlay, close_settings_overlay, set_settings_tab

// ── MCP input helpers ─────────────────────
update_mcp_name_input, update_mcp_command_input

// ── Model filter helpers ──────────────────
set_filter_coding, set_filter_reasoning, cycle_model_sort_mode

// ── Key persistence helpers ───────────────
save_api_key, on_key_saved, clear_api_key, on_key_cleared

// ── Tabby connection helpers ──────────────
save_tabby_settings, on_tabby_saved, clear_tabby_settings

// ── Inference lifecycle helpers ───────────
stop_inference, on_inference_log_line, on_inference_exited

// ── PTY helpers ───────────────────────────
toggle_pty, send_pty_input, pty_ctrl_c, pty_clear

// ── Attachment helpers ────────────────────
remove_attachment, clear_attachments, on_file_read_done

// ── Mention helpers ───────────────────────
move_mention_selection, confirm_mention, load_mention_candidates

// ── Write confirm helpers ─────────────────
approve_pending_writes, deny_pending_writes, discard_write_call

// ── Usage helpers ─────────────────────────
on_generation_loaded
```

## Refactoring Rounds

Incremental behavior-preserving extraction:

| Round | Commit | Scope |
|-------|--------|-------|
| 1 | `a93f32e` | App state separation (UiState, ModelFilterState, McpInputState) |
| 2 | `f6e7eb3` | UI/control arm extraction (settings, MCP, filters, commands, sessions) |
| 3 | `3945794` | Simple input handlers (fields, toggles, PTY input, file drag) |
| 4 | `72ea2a0` | Handler groups (HF token, Tabby presets, file read, model select, MCP tools) |
| 5 | `6d6b0cd` | Result/config handlers (HF token save, model preset, account, MCP server) |
| 6 | `a319f6c` | Inference lifecycle and usage handlers |

### Remaining Heavy Arms

These arms stay inline because they involve async spawn, runtime management, or complex control flow:

- `StartInference` (~200 lines): runtime spawn, Tabby ping retry, model loading
- `NewChat`, `SwitchSession`: session lifecycle, scroll restore
- `Send`: slash commands, compare mode, stream setup
- `StartHfDownload`, `HfDownloadEvent`: async download progress
- `SelectInferenceEngine`: complex match with keystore writes

Future extraction for these may require moving to separate modules (`update_inference.rs`, `update_chat.rs`) rather than helper methods.

## Anti-Patterns

- **Do not** extract arms that spawn `Task::perform` or `Task::run` into helpers unless they are pure async triggers with no complex pre/post logic
- **Do not** extract session lifecycle arms (`NewChat`, `SwitchSession`) that call `save_session`, `snapshot_current_to_inactive`, and Iced scroll operations
- **Do not** add helper methods for arms that are already single-line delegates (`Task::perform(...)`)
- **Do not** create helper methods that return hardcoded `Task::none()` unless they genuinely encapsulate meaningful state transitions
