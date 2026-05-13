#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

cd "${ROOT_DIR}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "[run] cargo is not installed or not in PATH." >&2
  exit 1
fi

echo "[run] Launching CodeWarp with cargo run..."
cargo run -- "$@"
