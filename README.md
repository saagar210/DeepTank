# DeepTank Development Guide

This project has two development modes:

- Normal dev: fastest repeat startup (keeps build artifacts on disk).
- Lean dev: lower disk growth (uses temporary build caches and auto-cleans on exit).

## Canonical commands

Commands are defined in `package.json` scripts and Tauri build config:

- `npm run dev`: starts the frontend dev server (`vite`).
- `npm run tauri -- dev`: starts full desktop app dev mode (runs frontend + Rust app).
- `npm run build`: builds frontend for production.
- `npm run preview`: serves the production frontend build.
- `npm run dev:lean`: starts full desktop app in lean mode.
- `npm run clean:heavy`: removes heavy build artifacts only.
- `npm run clean:local`: removes all reproducible local artifacts.

Tauri config source: `src-tauri/tauri.conf.json`

- `beforeDevCommand`: `npm run dev`
- `beforeBuildCommand`: `npm run build`

## Normal dev

Use normal mode when startup speed matters more than disk use:

```bash
npm run tauri -- dev
```

Disk behavior:

- Keeps Rust build output under `${XDG_CACHE_HOME:-$HOME/.cache}/deeptank/cargo-target` (can be large).
- Keeps Vite cache under `node_modules/.vite`.

## Lean dev

Use lean mode when you want to keep disk usage low during daily work:

```bash
npm run dev:lean
```

What lean mode does:

- Sets temporary `CARGO_TARGET_DIR` for Rust build output.
- Sets temporary `VITE_CACHE_DIR` for Vite cache output.
- Stores lean caches in `${TMPDIR:-/tmp}/deeptank-lean` (outside the repo).
- Deletes that temporary run directory automatically when the app exits.

Tradeoff:

- Lower persistent disk usage.
- Slower next startup because caches are not reused.

## Cleanup commands

Targeted cleanup (heavy artifacts only):

```bash
npm run clean:heavy
```

Removes:

- `dist`
- `src-tauri/target`
- `node_modules/.vite`
- `.lean-tmp`
- `${TMPDIR:-/tmp}/deeptank-lean`
- `${XDG_CACHE_HOME:-$HOME/.cache}/deeptank/cargo-target`

Full reproducible local cleanup:

```bash
npm run clean:local
```

Removes:

- everything from `clean:heavy`
- `node_modules`

Use full cleanup when you need to reclaim maximum space and are fine reinstalling dependencies.
