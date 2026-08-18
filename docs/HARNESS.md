# Harness System

This project now includes a shared harness for repeatable local verification and CI execution.

## Goals

- Keep one standard command flow for local checks and CI.
- Catch regressions early with a predictable step order.
- Optionally verify OpenAI-compatible endpoint health (`/v1/models`) from the same tool.
- When `--token`/`-Token` is supplied, the endpoint check sends both `Authorization: Bearer` and `x-api-key`, matching the app's local-provider client.

## Entry Points

- Windows: `scripts/harness.ps1`
- Linux/macOS: `scripts/harness.sh`

## Default Sequence

When no skip flags are provided, the harness runs:

1. `cargo fmt -- --check`
2. `cargo check`
3. `cargo test --all-targets -- --test-threads=1`
4. `cargo clippy --all-targets`

`clippy` is warning-tolerant by default. Use strict mode when needed.

## Usage

### Windows (PowerShell)

```powershell
pwsh -File scripts/harness.ps1
```

If `pwsh` is unavailable, use Windows PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/harness.ps1
```

Strict clippy:

```powershell
pwsh -File scripts/harness.ps1 -StrictClippy
```

Use an alternate cargo target directory:

```powershell
pwsh -File scripts/harness.ps1 -CargoTargetDir C:\tmp\codewarp-target
```

Run endpoint health check:

```powershell
pwsh -File scripts/harness.ps1 -SkipClippy -Endpoint http://localhost:8080
```

### Linux/macOS (bash)

```bash
bash scripts/harness.sh
```

Strict clippy:

```bash
bash scripts/harness.sh --strict-clippy
```

Use an alternate cargo target directory:

```bash
bash scripts/harness.sh --target-dir /tmp/codewarp-target
```

Run endpoint health check:

```bash
bash scripts/harness.sh --skip-clippy --endpoint http://localhost:8080
```

### Provider streaming smoke

When a local OpenAI-compatible daemon is already running, verify the same model-list and
streaming-chat paths used by CodeWarp:

```powershell
python scripts/provider-smoke.py --endpoint http://127.0.0.1:11434 --model llama3:latest
```

Use the Tabby endpoint and token when applicable:

```powershell
python scripts/provider-smoke.py --endpoint http://localhost:8080 --model <model-id> --token <token>
```

The smoke tool does not start or stop a provider and never prints the token. It requires a
non-empty streamed text response and the `[DONE]` marker, so it is stronger than a `/v1/models`
health check alone.

## Skip Flags

- `fmt`: `-SkipFmt` / `--skip-fmt`
- `check`: `-SkipCheck` / `--skip-check`
- `tests`: `-SkipTests` / `--skip-tests`
- `clippy`: `-SkipClippy` / `--skip-clippy`

## CI Integration

The GitHub Actions workflow now uses this harness:

- Matrix harness job (`ubuntu`, `windows`) for `fmt + check + test`
- Separate strict clippy job on `ubuntu`
- Release smoke matrix (`ubuntu`, `windows`) for release build, Windows startup smoke, manifest validation, archive packaging, and artifact upload

The test process is intentionally single-threaded because the Windows ConPTY and external-process fixtures are not safe to run concurrently. This keeps local and CI behavior aligned through a shared execution path.

The Linux POSIX PTY Ctrl+C end-to-end case remains intentionally ignored because signal and process-group delivery is not reliable on the current CI runner. Windows ConPTY Ctrl+C is covered by the automated fixture: CodeWarp sends the standard byte signal and, after a short grace period, terminates only the foreground-child snapshot if ConPTY drops the signal. Packaged GUI button behavior remains a release acceptance check.

### Manual release acceptance

Run the packaged Windows binary and record the result for each item before calling a release candidate ready:

- Enter Korean, emoji, mixed-width text, and multiline input; verify cursor placement, editing, send, and regenerate never reverse or duplicate characters.
- Send a streaming request through OpenRouter and a local OpenAI-compatible endpoint; verify model loading, incremental output order, non-ASCII text, stop, retry, and actionable authentication errors.
- Start a compare request, switch sessions, create a new chat, delete the current session, and stop the request; verify no stale placeholder or diff remains.
- Start an MCP/PTY-backed action, cancel it, restart it, and close the window; verify owned processes are reaped and the next run can start cleanly.
- After an unclean close, relaunch and verify session recovery status, backup fallback behavior, and the persisted conversation.
- On Linux, test interactive PTY Ctrl+C separately and record it as manual QA until the POSIX runner is reliable; on Windows, confirm the packaged GUI `^C` button once as release acceptance for the full UI path.
- On an interactive Windows desktop, run `powershell -ExecutionPolicy Bypass -File scripts/gui-smoke.ps1` against the release executable to verify the real text editor preserves Korean, emoji, multiline paste, and input order.

## Git Hooks (Recommended)

To enforce harness checks before push, install the local hooks path:

Windows:

```powershell
pwsh -File scripts/install-hooks.ps1
```

Linux/macOS:

```bash
bash scripts/install-hooks.sh
```

This enables:

- `.githooks/pre-commit`: `cargo fmt -- --check` (only when Rust-related files are staged)
- `.githooks/pre-push`: harness (`fmt + check + test`, clippy skipped)

Temporary bypass:

```bash
CODEWARP_SKIP_HOOK_HARNESS=1 git push
```

```bash
CODEWARP_SKIP_HOOK_FMT=1 git commit -m "..."
```
