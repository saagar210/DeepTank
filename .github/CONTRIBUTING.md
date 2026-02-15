# Contributing to DeepTank

Thanks for your interest in contributing! This guide will help you get started.

## Getting Started

### 1. Fork & Clone
```bash
git clone https://github.com/YOUR_USERNAME/DeepTank.git
cd DeepTank
```

### 2. Install Dependencies
```bash
npm install
```

### 3. Run in Development
```bash
npm run tauri dev    # Hot reload, dev tools enabled
```

The app opens with full Tauri dev inspector (press F12 in-app).

## Making Changes

### Create a Branch
```bash
git checkout -b feature/my-feature
# or
git checkout -b fix/my-bug
```

### Before Pushing

1. **Run tests locally:**
   ```bash
   npm test          # Frontend tests
   npx tsc --noEmit  # Type checking
   npm run build     # Ensure production build works
   ```

2. **Commit with conventional style:**
   - `feat: add new feature` — New features
   - `fix: resolve bug` — Bug fixes
   - `docs: update README` — Documentation
   - `refactor: clean up code` — Code cleanup
   - `test: add test suite` — Test additions
   - `chore: update deps` — Dependency updates

   **Example:**
   ```bash
   git add .
   git commit -m "feat: add pause button to toolbar"
   ```

3. **Push your branch:**
   ```bash
   git push origin feature/my-feature
   ```

## Submitting a Pull Request

1. Go to [DeepTank/pulls](https://github.com/saagar210/DeepTank/pulls)
2. Click "New Pull Request"
3. Select your branch and fill in the description
4. Link any related issues (e.g., "Closes #42")
5. Ensure CI checks pass (test workflow + build workflow)

### PR Guidelines
- Keep PRs focused (one feature or bug fix per PR)
- Include a clear description of what changed and why
- Reference any issues or discussions
- All tests must pass before merge

## Code Style

- **TypeScript:** Strict mode enabled (`"strict": true`), no `any` types
- **Rust:** Standard Rust conventions, `cargo fmt` before commit
- **Comments:** Only add comments for non-obvious logic
- **Naming:** Descriptive names, avoid abbreviations

## Testing Your Changes

### Frontend
```bash
npm test                 # Watch mode
npm test -- --run       # Run once
```

### Desktop App
```bash
npm run tauri dev       # Development mode with hot reload
npm run tauri build     # Production build
```

## Architecture

See [ARCHITECTURE.md](../docs/ARCHITECTURE.md) for system design. Key boundaries:

- **`src-tauri/src/simulation/`** — Physics, AI, genetics (all in Rust)
- **`src/renderer/`** — Canvas drawing (no React re-renders)
- **`src/components/`** — UI panels (React hooks)
- **`src/audio/`** — Web Audio procedural synth

## Questions?

Open an [issue](https://github.com/saagar210/DeepTank/issues) with the label `question`.

Happy coding! 🐠
