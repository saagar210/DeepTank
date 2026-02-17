#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/deeptank/cargo-target"

if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
  export CARGO_TARGET_DIR="$DEFAULT_CARGO_TARGET_DIR"
fi

mkdir -p "$CARGO_TARGET_DIR"

cd "$ROOT_DIR"
node ./node_modules/@tauri-apps/cli/tauri.js "$@"
