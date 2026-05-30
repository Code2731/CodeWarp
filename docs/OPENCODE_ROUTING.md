# OpenCode Routing Policy

Phase 3-2 decision: no global config tuning required.

## Config Location

Global OpenCode configs live under `C:\Users\USER\.config\opencode` and must not be committed to this repo.

## Policy Principles

- **GPT-5.5** handles general reasoning, planning, visual/high-judgment paths.
- **GPT-5.3 Codex** handles long agentic engineering, deep, and high implementation paths.
- **GPT-5.3 Codex Spark** handles fast iteration and low-effort paths.
- **MiMo** is primary only for writing tasks and otherwise serves as a non-core/low-stakes fallback.

## Routing Matrix

| Route                | Primary Model                     | Fallback                        |
|----------------------|-----------------------------------|---------------------------------|
| sisyphus             | openai/gpt-5.5 (medium)           |                                 |
| oracle               | openai/gpt-5.5 (high)             |                                 |
| prometheus           | openai/gpt-5.5 (high)             |                                 |
| metis                | openai/gpt-5.5 (high)             | openrouter/openai/gpt-5.5       |
| momus                | openai/gpt-5.5 (xhigh)            | openrouter/openai/gpt-5.5       |
| hephaestus           | openai/gpt-5.3-codex (high)       |                                 |
| atlas                | openai/gpt-5.3-codex (medium)     |                                 |
| sisyphus-junior      | openai/gpt-5.3-codex-spark (medium) |                               |
| librarian            | openai/gpt-5.4-mini-fast          | mimo/mimo-v2.5-pro              |
| explore              | openai/gpt-5.4-mini-fast          | mimo/mimo-v2.5-pro              |
| multimodal-looker    | openai/gpt-5.5 (medium)           | mimo/mimo-v2.5                  |
| visual-engineering   | openai/gpt-5.5 (high)             |                                 |
| ultrabrain           | openai/gpt-5.5 (xhigh)            |                                 |
| deep                 | openai/gpt-5.3-codex (high)       |                                 |
| artistry             | openai/gpt-5.5 (xhigh)            |                                 |
| quick                | openai/gpt-5.3-codex-spark        | mimo/mimo-v2.5                  |
| unspecified-low      | openai/gpt-5.3-codex-spark (medium) | mimo/mimo-v2.5                |
| unspecified-high     | openai/gpt-5.3-codex (high)       |                                 |
| writing              | mimo/mimo-v2.5-pro (medium)        | openai/gpt-5.5 (medium)         |

## MiMo Constraints

MiMo is the primary model for the `writing` route only. It must not be promoted to primary for core reasoning or coding routes. Its role outside writing is limited to non-core and low-stakes fallback scenarios.

## Accepted Warning

`oh-my-openagent doctor` may report `writing=mimo/mimo-v2.5-pro (unknown)` because MiMo is a custom provider known through OpenCode compatibility. This is expected and does not indicate a misconfiguration.

## Verification

Run the offline drift checker first to confirm the active OmO routing config matches this policy matrix:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\verify-opencode-routing.ps1"
```

For fixture-based validation without touching global config, override the config path:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\verify-opencode-routing.ps1" -ConfigPath "tests\fixtures\opencode-routing\valid-oh-my-openagent.json"
```

The drift checker is static and offline. It compares the exact route-name set, primary model, primary variant presence/value, fallback model count/value, and fallback variant presence/value. It does not call `opencode`, `bunx`, provider APIs, or network services, and it must not print `auth.json`, keys, tokens, or full global config contents.

Run the runtime smoke helper separately to confirm MiMo and OpenCode models resolve correctly:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\verify-opencode-mimo.ps1"
```

## Acceptance Checklist

- [ ] No secrets or auth paths were changed.
- [ ] No global config files were committed to this repo.
- [ ] Offline routing drift checker confirms the active OmO config matches this matrix.
- [ ] Runtime helper script verifies MiMo Pro, MiMo base, GPT-5.5, and Codex Spark resolve.
