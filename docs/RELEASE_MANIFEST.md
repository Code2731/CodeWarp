# Portable release archive manifest

Each portable archive is rooted at `codewarp-<Cargo package version>/` and contains:

| Input | Purpose |
| --- | --- |
| `codewarp` or `codewarp.exe` | Release binary for the target platform |
| `README.md` | Usage and platform notes |
| `LICENSE-MIT` | Project MIT license |
| `LICENSE-APACHE` | Project Apache-2.0 license |
| `assets/fonts/LICENSE.txt` | Pretendard and bundled-font SIL OFL text |
| `assets/fonts/LICENSE-JetBrainsMono.txt` | JetBrains Mono SIL OFL text |

Run `python scripts/validate-release-manifest.py --binary <release binary>` before archiving.
The validator reads the package version and license expression from `Cargo.toml`; it also rejects
hard-coded CodeWarp versions in the app/MCP identity sources.
