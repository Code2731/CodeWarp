# Harness System

This project now includes a shared harness for repeatable local verification and CI execution.

## Goals

- Keep one standard command flow for local checks and CI.
- Catch regressions early with a predictable step order.
- Optionally verify OpenAI-compatible endpoint health (`/v1/models`) from the same tool.

## Entry Points

- Windows: `scripts/harness.ps1`
- Linux/macOS: `scripts/harness.sh`

## Default Sequence

When no skip flags are provided, the harness runs:

1. `cargo fmt -- --check`
2. `cargo check`
3. `cargo test --all-targets`
4. `cargo clippy --all-targets`

`clippy` is warning-tolerant by default. Use strict mode when needed.

## Usage

### Windows (PowerShell)

```powershell
pwsh -File scripts/harness.ps1
```

Strict clippy:

```powershell
pwsh -File scripts/harness.ps1 -StrictClippy
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

Run endpoint health check:

```bash
bash scripts/harness.sh --skip-clippy --endpoint http://localhost:8080
```

## Skip Flags

- `fmt`: `-SkipFmt` / `--skip-fmt`
- `check`: `-SkipCheck` / `--skip-check`
- `tests`: `-SkipTests` / `--skip-tests`
- `clippy`: `-SkipClippy` / `--skip-clippy`

## CI Integration

The GitHub Actions workflow now uses this harness:

- Matrix harness job (`ubuntu`, `windows`) for `fmt + check + test`
- Separate strict clippy job on `ubuntu`

This keeps local and CI behavior aligned through a shared execution path.

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

This enables `.githooks/pre-push`, which runs harness (`fmt + check + test`, clippy skipped).

Temporary bypass:

```bash
CODEWARP_SKIP_HOOK_HARNESS=1 git push
```
