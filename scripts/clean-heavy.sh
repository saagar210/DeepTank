#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_CARGO_TARGET_DIR="${XDG_CACHE_HOME:-$HOME/.cache}/deeptank/cargo-target"

cd "$ROOT_DIR"
rm -rf dist
rm -rf src-tauri/target
rm -rf node_modules/.vite
rm -rf .lean-tmp
rm -rf "${TMPDIR:-/tmp}/deeptank-lean"
rm -rf "$DEFAULT_CARGO_TARGET_DIR"

echo "Removed heavy build artifacts: dist, src-tauri/target, node_modules/.vite, .lean-tmp, ${TMPDIR:-/tmp}/deeptank-lean, $DEFAULT_CARGO_TARGET_DIR"
