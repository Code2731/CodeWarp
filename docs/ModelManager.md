# Model Manager Architecture (CodeWarp)

This document describes how CodeWarp manages model sources, Hugging Face downloads, EXL2 presets, and TabbyAPI runtime flow based on the current Rust implementation in:

- `src/main.rs`
- `src/update.rs`
- `src/hf.rs`
- `src/tabby.rs`

---

## 1) High-level model source architecture

CodeWarp merges models from two provider families into one selector (`ModelOption`):

- **OpenRouter** (`LlmProvider::OpenRouter`)
- **OpenAI-compatible local/remote endpoint** (`LlmProvider::OpenAICompat`) — used for TabbyAPI/TabbyML/xLLM/vLLM/Ollama/custom endpoints

Each selector option stores:

- `id` (model id)
- `provider`
- `provider_label` (e.g. `TabbyAPI`, `TabbyML`, `xLLM`, fallback `[Local]`)
- display metadata (`ko_friendly`, `favorite`, `context_length`, pricing)

The selector text is formatted by provider:

- OpenRouter: `[OR] ...`
- OpenAICompat: `[<label>] ...` (or `[Local]` if label is empty)

### How models appear in the selector

#### OpenRouter models

1. `Message::FetchModels` calls `openrouter::list_models`.
2. `Message::ModelsLoaded(Ok(models))` removes old OpenRouter options and repopulates `model_options` with fresh OpenRouter entries.
3. Pricing/context are mapped into `ModelOption`.
4. `refresh_model_combo()` rebuilds combo-box state using active filters/sort.

#### OpenAI-compatible endpoint models (Tabby-style/local endpoint)

1. `Message::FetchTabbyModels` / `FetchTabbyModelsRetry` calls `tabby::list_models(base_url, token)`.
2. `Message::TabbyModelsLoaded(Ok(ids))` removes previous OpenAICompat options and inserts current IDs as OpenAICompat entries.
3. Those entries are marked as `free` (`prompt_per_million = 0`, `completion_per_million = 0`).
4. Provider tag comes from `openai_compat_label` (`[TabbyAPI]`, `[TabbyML]`, etc.).
5. `refresh_model_combo()` rebuilds the visible selector list.

If both providers expose the same model id, provider disambiguation is handled by `selected_model_provider` sync logic.

---

## 2) Hugging Face download pipeline

Primary entry point:

- `Message::StartHfDownload` in `src/update.rs`
- backend stream: `hf::download_repo(...)` in `src/hf.rs`

### End-to-end flow

1. **Input capture**
   - Repo id from `hf_repo_input`.
   - Optional HF token (`hf_token_input`, or keystore fallback).
   - Download dir from `model_dir_input` (auto-defaulted if empty).
   - Optional revision (`hf_revision`) and folder name (`hf_folder_name`).

2. **Start**
   - Ensures destination directory exists.
   - Persists model directory via `keystore::write_model_dir`.
   - Starts abortable task with `hf::download_repo`.

3. **Repository metadata + revision handling (`hf.rs`)**
   - Loads model info for requested revision.
   - On non-main 404, tries branch fallback logic:
     - exact branch match
     - normalized revision match
     - closest `bpw` branch (numeric distance)
     - fallback to `main` or first branch
   - On failure, includes annotated branch suggestions in error text.

4. **File tree resolution**
   - Fetches recursive repo tree (`/tree/<rev>?recursive=true`) and filters non-directory entries.
   - Uses this as sibling file list.

5. **Download streaming + progress events**
   - Emits `DownloadEvent` sequence:
     - `Started { total_files }`
     - `FileStart { idx, name, size }`
     - `FileProgress { idx, bytes_done, bytes_total }` (every ~1MB and file end)
     - `FileDone`
     - `AllDone` or `Error`
   - Preserves nested folder structure under `<dest>/<folder_name>/...`.

6. **UI state updates**
   - `Message::HfDownloadEvent` updates `hf_dl` progress state.
   - On `Error`, status is humanized (`hf::humanize_error`) plus extracted fallback hints.

7. **Post-download model selection integration**
   - On `AllDone`, app resolves a valid TabbyAPI model directory (must include `config.json` + weight files).
   - Then auto-configures runtime fields for TabbyAPI (engine, port, URL, selected model path, launcher if discoverable).

---

## 3) EXL2 preset system

EXL2 presets are declared as `const EXL2_PRESETS` in `src/main.rs`.

Each preset contains:

- `repo_id`
- `revision` (bpw branch like `4.0bpw`, `3.5bpw`, `6.0bpw`)
- `folder_name`
- `label`, `note`, `vram`

### Preset trigger path

1. UI sends `Message::DownloadExl2Preset(idx)`.
2. `prepare_exl2_preset_download(idx)` sets:
   - `hf_repo_input = preset.repo_id`
   - `hf_revision = Some(preset.revision)`
   - `hf_folder_name = Some(preset.folder_name)`
3. It immediately dispatches `Message::StartHfDownload`.

### Folder resolution behavior

Model folder resolution for EXL2/TabbyAPI uses helpers in `src/main.rs`:

- validates direct model dir (`config.json` + weight file)
- if parent contains multiple candidate subfolders, resolves by `bpw` hint extracted from folder name when possible
- detects downloaded preset folders by exact match first, then repo-stem + bpw heuristic

This is why preset `folder_name` is important: it helps deterministic selection after download.

---

## 4) TabbyAPI runtime lifecycle

Runtime orchestration is handled in `src/update.rs` with `InferenceEngine::TabbyApi` and shared spawn infrastructure.

### Lifecycle stages

1. **Launcher validation**
   - `validate_tabbyapi_launcher_path` enforces valid launcher script path.
   - Allowed launcher names: `Start.bat`/`Start.cmd` (Windows), `start.sh`, or `main.py`.
   - Explicitly rejects `tabby/tabby.exe/tabby.cmd/tabby.bat` (TabbyML CLI).

2. **Model path validation for TabbyAPI**
   - Selected model path must resolve to a valid TabbyAPI model dir (`config.json` + weights).

3. **Config generation**
   - `write_tabbyapi_config_for_launcher(...)` writes runtime `config.yml` near launcher.
   - Config pins host `127.0.0.1`, chosen port, `disable_auth: true`, and selected model dir/name.

4. **Spawn**
   - `StartInference` composes command (`Start.bat --config config.yml` / `./start.sh --config config.yml`).
   - `resolve_runtime_spawn_command` adapts to script type (`cmd.exe /C ...`, python runner, etc.).
   - `spawn_inference_stream` starts child process and streams stdout/stderr lines.

5. **PID + logs + process state**
   - First log line includes `[pid:<n>]`, captured to `inference_pid`.
   - Log buffer kept as FIFO.
   - Exit is handled by `Message::InferenceExited` (status update + model option cleanup).

6. **Ping/model discovery**
   - After spawn start, CodeWarp schedules automatic connection attempts.
   - It calls `tabby::list_models` against configured URL.
   - If temporarily unreachable, retry loop runs (generation-safe retries with delay/count guard).

7. **Endpoint ready**
   - On `TabbyModelsLoaded(Ok(ids))`, OpenAICompat model options are populated and selector is refreshed.
   - Status becomes connected, and a local model may become selected if current selection is not local.

### `tabby::list_models` endpoint compatibility

`src/tabby.rs` runtime model listing behavior:

- Primary: `GET {base}/v1/models`
- Compatibility fallback for 404/405: `GET {base}/v1/model/list`
- Accepts multiple JSON response shapes and normalizes to model id vector
- Applies both bearer and `x-api-key` headers when token is provided

---

## 5) Inference engine enum and model namespace behavior

`InferenceEngine` in `src/main.rs`:

- `XLlm`
- `VLlm`
- `LlamaServer`
- `TabbyMl`
- `TabbyApi`
- `Ollama`
- `Custom`

### Namespace effect

`shares_model_namespace(...)` controls whether model selection is preserved when engine changes:

- Shared namespace: `XLlm` / `VLlm` / `LlamaServer`
- Isolated namespaces: `TabbyMl`, `TabbyApi`, `Ollama`, `Custom` (each isolated)

When switching engine via `Message::SelectInferenceEngine`, if namespaces do not match, `inference_selected_model` is cleared.

This prevents accidentally reusing incompatible model identifiers across runtime families.

---

## 6) Model selection persistence and session persistence

### Selected model persistence

- User model selection (`select_model`) writes model id to keystore (`write_selected_model`).
- On startup (`App::new`), `read_selected_model` restores previous selection candidate.
- Auto-selection paths (e.g. first compatible model after refresh) also persist via keystore writes.

### Runtime/model-related persistence

Model management-related settings are persisted through keystore writes during updates, including:

- model directory (`write_model_dir`)
- inference binary/script path (`write_inference_binary`)
- local endpoint base URL (`write_tabby_base_url`)
- OpenAICompat label (`write_openai_compat_label`)

### Session save behavior

`save_session()` persists current + inactive sessions:

- conversation history
- user/assistant blocks (tool-result blocks are intentionally ephemeral and not saved)
- session metadata (title, scroll position, next block id)

So model selection is keystore-backed, while chat state and visible assistant/user history are session-backed.

---

## 7) Related constants and preset inventories

### General model presets (`MODEL_PRESETS`)

Defined in `src/main.rs` for quick Hugging Face repo input fill (non-EXL2 flow).

### EXL2 presets (`EXL2_PRESETS`)

Defined in `src/main.rs` for one-click EXL2 download with explicit `revision` (bpw) and deterministic `folder_name`.

Current preset families include Llama 3.2/3.1 and Gemma EXL2 variants with VRAM notes.

---

## 8) Practical summary

- CodeWarp’s model selector is a merged view of OpenRouter + OpenAI-compatible endpoint models.
- HF downloads are stream-driven with robust revision fallback logic and EXL2-aware folder targeting.
- TabbyAPI runtime is validated, configured, spawned, and health-checked before model IDs are surfaced in the selector.
- EXL2 presets are not just labels: they encode revision (`bpw`) and folder naming used downstream for model path resolution.
- Persistence is split between keystore (settings/selection) and session storage (conversation/UI state).
