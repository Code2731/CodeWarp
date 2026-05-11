#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
hooks_path="$repo_root/.githooks"

if [[ ! -d "$hooks_path" ]]; then
  echo "Hooks path not found: $hooks_path" >&2
  exit 1
fi

git config core.hooksPath ".githooks"
echo "Installed git hooks path: .githooks"
echo "pre-push will now run the harness by default."
