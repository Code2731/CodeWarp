# CodeWarp Coding Convention

This document describes **actual coding patterns currently used in this repository** (not generic Rust style advice).

Source basis:
- `src/main.rs`
- `src/block/mod.rs`
- `src/model/mod.rs`
- `src/palette.rs`
- `src/runtime_process/mod.rs`
- `src/update.rs`
- `src/view/mod.rs`
- `src/view/ui/mod.rs`
- `src/session/mod.rs`
- `Cargo.toml`
- `scripts/harness.ps1`, `scripts/harness.sh`
- recent `git log --oneline`
- test listing from `cargo test --all-targets -- --list`

## 1) Project structure and module organization

- Main entry point is `src/main.rs`.
- Feature modules are declared at top-level in `main.rs` using lowercase module names:
  - `mod block;`, `mod bootstrap;`, `mod hf;`, `mod input;`, `mod keystore;`, `mod mcp;`, `mod model;`, `mod openrouter;`, `mod palette;`, `mod pty;`, `mod runtime_process;`, `mod session;`, `mod tabby;`, `mod tools;`, `mod update;`, `mod util;`, `mod view;`
- `src/update.rs` is implemented as a child module of `main.rs` and uses `use super::*;`.
- `src/view/mod.rs` is a child module of `main.rs`; `src/view/ui/mod.rs` keeps UI constants/style helpers close to rendering code.
- Core app wiring follows Iced Elm-style flow from `main.rs`:
  - `App::new`
  - `App::update`
  - `App::view`

## 2) Naming conventions used in code

### Types (PascalCase)
- Structs and enums use `PascalCase`.
- Examples from `src/main.rs` / `src/model/mod.rs` / `src/block/mod.rs` / `src/session/mod.rs`:
  - `ModelOption`, `ApplyCandidate`, `PersistedSessionData`
  - `LlmProvider`, `InferenceEngine`, `SettingsTab`, `Message`

### Functions and methods (snake_case)
- Free functions and methods use `snake_case`.
- Examples:
   - `resolve_user_path`, `fmt_context_length`, `extract_mention_query` (`src/util/mod.rs`)
   - `validate_tabbyapi_launcher_path`, `run_tool_round`, `kick_chat_stream` (`src/update.rs`)
   - `view_topbar`, `view_sidebar`, `view_statusbar` (`src/view/mod.rs`)

### Variables and fields (snake_case)
- Local variables and struct fields use `snake_case`.
- Examples: `selected_model_provider`, `tabby_retry_generation`, `context_length`, `prompt_per_million`.

### Constants (SCREAMING_SNAKE_CASE)
- Constants use `SCREAMING_SNAKE_CASE`.
- Examples:
   - `MAX_ATTACH_BYTES`, `PTY_MAX_LINES`, `TABBY_API_DEFAULT_PORT` (`src/util/mod.rs` / `src/model/mod.rs`)
   - `TABBY_CONNECT_RETRIES_AFTER_START`, `TABBY_CONNECT_RETRY_DELAY_SECS` (`src/update.rs`)

### Module names (lowercase)
- Module file names and declarations are lowercase (for example `session`, `openrouter`, `update`, `view`).

## 3) Struct and impl patterns (`main.rs`)

- Data carriers are plain structs with explicit typed fields (for example `ModelOption`, `InactiveSession`, `UiState`).
- Behavior is attached with `impl` blocks (for example `impl App`, `impl std::fmt::Display for ModelOption`, `impl BlockBody`).
- Conversion / display logic is often implemented via trait impls instead of ad-hoc formatting in callers.

## 4) Update refactor pattern: state-transition helpers returning `Task<Message>`

`src/update.rs` uses a clear extraction pattern:

1. `pub(crate) fn update(&mut self, message: Message) -> Task<Message>` is a dispatcher.
2. Most `Message` arms delegate to focused helper methods.
3. Helper methods are grouped by concern using section comments (settings, key persistence, tabby connection, PTY, attachments, etc.).

Common helper shape:

- Signature: `fn <action>(&mut self, ...) -> Task<Message>`
- Return style:
  - `Task::none()` for pure state mutation
  - `Task::done(Message::...)` for immediate follow-up transition
  - `Task::perform(async ..., Message::...)` for async side effects

Examples:
- `save_api_key` -> `Task::perform(...)`
- `on_key_saved` -> `Task::done(Message::FetchModels)` on success
- `toggle_pty` -> `Task::done(Message::PtyStart)` only when required

## 5) UI element construction patterns (`src/view/mod.rs`)

- View methods are split into small composable builders with `view_*` naming:
  - `view_topbar`, `view_sidebar`, `view_stream`, `view_rightpanel`, `view_settings`, `view_statusbar`, etc.
- UI composition uses `iced` widget macros and builders (`row!`, `column!`, `stack!`, `container(...)`, `button(...)`, `scrollable(...)`).
- Pattern used repeatedly:
  - build smaller `Element<Message>` parts
  - compose into larger sections
  - apply spacing/padding/style near composition site

## 6) Serialization and persistence patterns (`src/session/mod.rs`)

- Persistence models derive serde traits:
  - `#[derive(..., Serialize, Deserialize)]`
- Backward compatibility uses a dedicated old schema type (`OldPersistedSession`) and migration path.
- Optional/backfill fields use `#[serde(default)]` (for example `model`, `scroll_y`).
- Files are saved as pretty JSON via `serde_json::to_string_pretty`.

## 7) Error handling patterns

### Result propagation as `Result<T, String>`
- Across app/service boundaries, errors are frequently normalized to `String`.
- Typical style:
  - `ok_or_else(|| "...".to_string())?`
  - `.map_err(|e| format!("...: {e}"))?`
  - `return Err(format!("..."));`

### Keystore handling pattern
- Two styles are used intentionally:
  1. **Critical flows**: propagate explicit result through async callbacks
     - examples: `save_api_key`, `on_key_saved`, `save_tabby_settings`, `on_tabby_saved`
  2. **Best-effort persistence side writes**: ignore failure explicitly with `let _ = ...`
     - examples: `let _ = keystore::write_selected_model(...)`, `let _ = keystore::write_tabby_base_url(...)`

### Status message pattern
- User-facing operation status is centralized through `self.status` updates in update handlers.
- Provider connection health additionally uses `self.tabby_status` (`Some(Ok(...))` / `Some(Err(...))`).
- Error text is often humanized before display (`openrouter::humanize_error`, `hf::humanize_error`, and composed hint messages).
- Status strings include Korean operational text in UI-facing messages; this is presentation-level messaging, not business-logic branching.

## 8) Dependency management pattern (`Cargo.toml`)

- Runtime dependencies are in `[dependencies]`; test-only dependencies in `[dev-dependencies]`.
- Heavy crates are configured with explicit features:
  - `iced = { version = "0.14", features = ["tokio", "advanced", "highlighter", "markdown"] }`
  - `tokio = { version = "1", features = ["rt-multi-thread", "macros", "sync", "process", "io-util"] }`
  - `reqwest` disables default features and enables `rustls-tls` + `json` + `stream`.
- Release profile is optimized explicitly (`lto = true`, `codegen-units = 1`, `strip = true`).

## 9) Test organization pattern

- **Unit tests** are colocated with source modules using `#[cfg(test)] mod tests`.
  - Examples: `src/session/mod.rs`, `src/update.rs`, and other modules compiled into `src/main.rs`.
- **Integration test** exists under `tests/`:
  - `tests/test.rs`
- Run `cargo test --all-targets -- --test-threads=1` for the current executable test count. The test process is single-threaded because Windows ConPTY and external-process fixtures are not safe to run concurrently. The dated 2026-08-19 verification reported 666 passing unit tests, 11 ignored tests (including the Linux POSIX PTY Ctrl+C manual-QA case), and 1 external integration smoke test.

## 10) Git commit message style

- Recent commits use plain-English imperative/descriptive lines without semantic prefixes (`feat:`, `fix:`, etc.).
- Examples from `git log --oneline`:
  - `Extract update handler groups into helpers`
  - `Extract App UI state groups`
  - `Improve TabbyAPI EXL2 folder resolution for nested variants`

## 11) Verification harness workflow

- Shared harness entry points:
  - `scripts/harness.ps1` (Windows)
  - `scripts/harness.sh` (Linux/macOS)
- Default ordered flow includes:
  1. `cargo fmt -- --check`
  2. `cargo check`
  3. `cargo test --all-targets -- --test-threads=1`
  4. `cargo clippy --all-targets` (warning-tolerant unless strict mode is enabled)

For day-to-day verification baseline in this repo, the core sequence is **fmt -> check -> test**, with clippy as an additional harness step.
