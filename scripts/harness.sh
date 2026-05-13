#!/usr/bin/env bash
set -euo pipefail

skip_fmt=0
skip_check=0
skip_tests=0
skip_clippy=0
strict_clippy=0
target_dir=""
endpoint=""
token=""
timeout_sec=5

usage() {
  cat <<'EOF'
Usage: scripts/harness.sh [options]

Options:
  --skip-fmt           Skip cargo fmt -- --check
  --skip-check         Skip cargo check
  --skip-tests         Skip cargo test --all-targets
  --skip-clippy        Skip cargo clippy --all-targets
  --strict-clippy      Fail when clippy reports issues
  --target-dir DIR     Set CARGO_TARGET_DIR for this harness run
  --endpoint URL       Run endpoint check against URL (/v1/models)
  --token TOKEN        Bearer token for endpoint check
  --timeout-sec N      Endpoint timeout in seconds (default: 5)
  -h, --help           Show this help
EOF
}

run_step() {
  local name="$1"
  shift
  echo "==> $name"
  "$@"
  echo "OK: $name"
  echo
}

models_url() {
  local base="$1"
  base="${base%"${base##*[![:space:]]}"}"
  base="${base#"${base%%[![:space:]]*}"}"
  base="${base%/}"

  if [[ "$base" == */v1 ]]; then
    echo "$base/models"
  else
    echo "$base/v1/models"
  fi
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-fmt) skip_fmt=1 ;;
    --skip-check) skip_check=1 ;;
    --skip-tests) skip_tests=1 ;;
    --skip-clippy) skip_clippy=1 ;;
    --strict-clippy) strict_clippy=1 ;;
    --target-dir)
      target_dir="${2:-}"
      shift
      ;;
    --endpoint)
      endpoint="${2:-}"
      shift
      ;;
    --token)
      token="${2:-}"
      shift
      ;;
    --timeout-sec)
      timeout_sec="${2:-5}"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      usage
      exit 2
      ;;
  esac
  shift
done

if [[ -n "${target_dir// }" ]]; then
  export CARGO_TARGET_DIR="$target_dir"
  echo "Using CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
fi

if [[ $skip_fmt -eq 0 ]]; then
  run_step "cargo fmt -- --check" cargo fmt -- --check
fi

if [[ $skip_check -eq 0 ]]; then
  run_step "cargo check" cargo check
fi

if [[ $skip_tests -eq 0 ]]; then
  run_step "cargo test --all-targets" cargo test --all-targets
fi

if [[ $skip_clippy -eq 0 ]]; then
  echo "==> cargo clippy --all-targets"
  set +e
  cargo clippy --all-targets
  clippy_exit=$?
  set -e

  if [[ $clippy_exit -ne 0 ]]; then
    if [[ $strict_clippy -eq 1 ]]; then
      echo "cargo clippy failed with exit code $clippy_exit" >&2
      exit "$clippy_exit"
    fi
    echo "WARN: cargo clippy reported issues (exit $clippy_exit). Continuing because --strict-clippy was not set." >&2
  else
    echo "OK: cargo clippy --all-targets"
    echo
  fi
fi

if [[ -n "${endpoint// }" ]]; then
  url="$(models_url "$endpoint")"
  echo "==> endpoint health check ($url)"
  tmp="$(mktemp)"
  trap 'rm -f "$tmp"' EXIT
  if [[ -n "${token// }" ]]; then
    code="$(curl -sS -m "$timeout_sec" -H "Authorization: Bearer $token" -o "$tmp" -w "%{http_code}" "$url")"
  else
    code="$(curl -sS -m "$timeout_sec" -o "$tmp" -w "%{http_code}" "$url")"
  fi

  if [[ "$code" -lt 200 || "$code" -ge 300 ]]; then
    echo "Endpoint health check failed: HTTP $code ($url)" >&2
    echo "Response preview:" >&2
    head -c 240 "$tmp" >&2 || true
    echo >&2
    exit 1
  fi

  echo "HTTP $code from $url"
  echo "OK: endpoint health check ($url)"
  echo
fi

echo "Harness completed successfully."
