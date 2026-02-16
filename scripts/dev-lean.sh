#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LEAN_BASE_DIR="${TMPDIR:-/tmp}/deeptank-lean"
mkdir -p "$LEAN_BASE_DIR"
RUN_DIR="$(mktemp -d "$LEAN_BASE_DIR/run.XXXXXX")"

cleanup() {
  if [[ -d "$RUN_DIR" ]]; then
    rm -rf "$RUN_DIR"
  fi
}

trap cleanup EXIT INT TERM

export CARGO_TARGET_DIR="$RUN_DIR/cargo-target"
export VITE_CACHE_DIR="$RUN_DIR/vite-cache"

echo "[lean-dev] Using ephemeral CARGO_TARGET_DIR=$CARGO_TARGET_DIR"
echo "[lean-dev] Using ephemeral VITE_CACHE_DIR=$VITE_CACHE_DIR"

cd "$ROOT_DIR"
npm run tauri -- dev
