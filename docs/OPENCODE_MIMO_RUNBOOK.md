# OpenCode + MiMo Operational Runbook

Phase 3-3 setup, verification, and troubleshooting reference for the OpenCode / oh-my-openagent / MiMo stack.

---

## Scope

This runbook covers setup facts, verification steps, runtime troubleshooting, and safety rules for the MiMo provider integrated through OpenCode. It does not repeat the routing matrix. For routing decisions and model assignments, see `docs/OPENCODE_ROUTING.md`.

## Config Files

| File | Path | Purpose |
|------|------|---------|
| OpenCode config root | `C:\Users\USER\.config\opencode` | All OpenCode configs live here |
| Global config | `opencode.jsonc` | OpenCode global settings |
| Routing config | `oh-my-openagent.json` | OmO agent routing |
| TUI config | `tui.json` | TUI display settings |

These files are global user config. They must not be committed to this repo.

## MiMo Provider Facts

- Provider ID: `mimo`
- Base URL: `https://token-plan-sgp.xiaomimimo.com/v1`
- Environment variable: `MIMO_API_KEY`
- Model IDs: `mimo/mimo-v2.5` (base), `mimo/mimo-v2.5-pro` (pro)

MiMo is a custom provider registered through OpenCode compatibility. It is the primary model for the `writing` route only. It must not be promoted to primary for core reasoning or coding routes.

## Authentication Rules

- **MIMO_API_KEY** must be set as a user environment variable before running any MiMo verification.
- If OpenAI OAuth or OpenRouter API keys are also in use, ensure those are configured through their respective flows.
- Never print, log, or commit API keys, tokens, or auth.json contents.

## Verification

### Run the full helper

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\verify-opencode-mimo.ps1"
```

This script:

1. Clears leaked `OPENCODE_*` shell variables (see Troubleshooting).
2. Checks that `MIMO_API_KEY` is set.
3. Runs `opencode models mimo` to list MiMo models.
4. Smoke-tests MiMo Pro (`mimo/mimo-v2.5-pro`), MiMo base (`mimo/mimo-v2.5`), GPT-5.5 (`openai/gpt-5.5`), and Codex Spark (`openai/gpt-5.3-codex-spark`) via `opencode run`.
5. Runs `bunx oh-my-openagent doctor` to check the agent config.

### Individual commands

```powershell
bunx oh-my-openagent doctor
```

```powershell
opencode models mimo
```

## Runtime Troubleshooting

### `Session not found` on `opencode run`

**Symptom:** `opencode run` fails with `Session not found`.

**Cause:** Desktop or client `OPENCODE_*` variables leak into the shell from the OpenCode desktop app. The affected variables are:

- `OPENCODE_CLIENT`
- `OPENCODE_DISABLE_EMBEDDED_WEB_UI`
- `OPENCODE_SERVER_USERNAME`
- `OPENCODE_SERVER_PASSWORD`

**Workaround:** Remove those variables before running `opencode run`:

```powershell
Remove-Item -LiteralPath "Env:\OPENCODE_CLIENT" -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "Env:\OPENCODE_DISABLE_EMBEDDED_WEB_UI" -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "Env:\OPENCODE_SERVER_USERNAME" -ErrorAction SilentlyContinue
Remove-Item -LiteralPath "Env:\OPENCODE_SERVER_PASSWORD" -ErrorAction SilentlyContinue
```

The helper script `scripts\verify-opencode-mimo.ps1` already clears these variables automatically.

## Known Warnings

`bunx oh-my-openagent doctor` may report:

```
writing=mimo/mimo-v2.5-pro (unknown)
```

This is an accepted custom-provider compatibility warning. MiMo is known through OpenCode compatibility, not through the built-in provider registry. This does not indicate a misconfiguration and can be ignored.

## Safety Rules

1. Never commit secrets, API keys, or global config files to this repo.
2. Do not print or log `auth.json` contents.
3. Do not promote MiMo to primary for core reasoning or coding routes. Its role outside `writing` is limited to non-core and low-stakes fallback.
4. For routing decisions and model assignments, link to `docs/OPENCODE_ROUTING.md`.

## Acceptance Criteria

- [ ] `docs/OPENCODE_MIMO_RUNBOOK.md` exists and covers all sections listed above.
- [ ] No secrets, API keys, or auth file contents appear in any file.
- [ ] No global OpenCode config files were modified or committed.
- [ ] The runbook does not duplicate the full routing matrix from `docs/OPENCODE_ROUTING.md`.
- [ ] The helper script verifies MiMo Pro, MiMo base, GPT-5.5, Spark, and doctor.

## Commit Guidance

One documentation-only commit:

```
Add OpenCode MiMo operational runbook
```

The commit must contain only `docs/OPENCODE_MIMO_RUNBOOK.md`. No script changes, no config changes, no other files.

## Related Docs

- `docs/OPENCODE_ROUTING.md` -- routing matrix and policy principles
- `scripts/verify-opencode-mimo.ps1` -- verification helper script
