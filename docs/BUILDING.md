# Building DeepTank from Source

This guide explains how to build and develop DeepTank on your machine.

## System Requirements

### macOS

- **Xcode Command Line Tools:**
  ```bash
  xcode-select --install
  ```

- **Rust (latest stable):**
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  source $HOME/.cargo/env
  ```

- **Node.js 20+:**
  Download from https://nodejs.org/ or:
  ```bash
  brew install node
  ```

- **pkg-config:**
  ```bash
  brew install pkg-config
  ```

### Linux (Debian/Ubuntu)

**Install system dependencies:**
```bash
sudo apt update
sudo apt install -y \
  build-essential \
  curl \
  wget \
  libssl-dev \
  libglib2.0-dev \
  pkg-config \
  libgtk-3-dev \
  librsvg2-dev \
  libayatana-appindicator3-dev
```

**Install Rust:**
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

**Install Node.js 20+:**
```bash
curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
sudo apt install -y nodejs
```

### Windows

1. **Visual Studio Build Tools 2022**
   - Download: https://visualstudio.microsoft.com/downloads/
   - Select "Desktop development with C++"
   - Includes MSVC compiler and Windows SDK

2. **Rust:**
   - Download: https://rustup.rs/
   - Follow the installer (default options)
   - Restart terminal after installation

3. **Node.js 20+:**
   - Download: https://nodejs.org/
   - Follow the installer (default options)

4. **Verify Installation:**
   ```powershell
   rustc --version
   cargo --version
   node --version
   npm --version
   ```

## Development Setup

### 1. Clone Repository

```bash
git clone https://github.com/saagar210/DeepTank.git
cd DeepTank
```

### 2. Install Dependencies

```bash
npm install
```

This installs:
- Frontend dependencies (React, Tauri CLI, etc.)
- Development tools (TypeScript, Vite, Vitest)

### 3. Run in Development Mode

```bash
npm run tauri dev
```

This command:
1. Starts Vite dev server (hot reload on file changes)
2. Compiles Rust code
3. Launches Tauri window with dev tools
4. Opens DevTools (F12 in-app for React dev tools, console, etc.)

**Development features:**
- Hot reload: Edit `src/` files, see changes immediately
- DevTools: Inspect React state, network, performance
- Rust reload: Changes to `src-tauri/src/` trigger recompile (takes ~5-10s)

## Running & Testing

### Frontend Tests

```bash
npm test              # Run all tests in watch mode
npm test -- --run     # Run once and exit
npm test -- --ui      # Open interactive UI
```

Tests include:
- Audio engine hardening
- Fish sprite generation
- Type safety validation

### Type Checking

```bash
npx tsc --noEmit      # Check for TypeScript errors
```

Run this before committing. CI will fail if there are type errors.

### Building for Production

```bash
npm run build         # Production bundle (frontend)
npm run tauri build   # Desktop app binary
```

**Output locations:**
- Frontend: `dist/` (minified, optimized)
- macOS: `src-tauri/target/release/bundle/dmg/`
- Linux: `src-tauri/target/release/bundle/appimage/`
- Windows: `src-tauri/target/release/bundle/msi/`

Build typically takes 2-3 minutes on first build, 30-60 seconds on rebuilds (incremental compilation).

### Rust Tests

```bash
cd src-tauri
cargo test          # Run all Rust tests
cargo test -- --nocapture  # Show print statements
```

**Note:** On Linux, ensure `libglib2.0-dev` is installed (see System Requirements).

## Project Structure

```
DeepTank/
├── src/                         # React frontend
│   ├── App.tsx                  # Main component
│   ├── main.tsx                 # React entry point
│   ├── types.ts                 # TypeScript interfaces
│   ├── components/              # 15 UI panels
│   ├── renderer/                # Canvas rendering engine
│   │   ├── canvasRenderer.ts
│   │   └── fishSprite.ts
│   └── audio/                   # Web Audio synth
│       └── audioEngine.ts
│
├── src-tauri/                   # Rust backend
│   ├── src/
│   │   ├── main.rs              # Tauri window setup
│   │   ├── lib.rs               # IPC commands + simulation state
│   │   └── simulation/          # Core simulation logic
│   │       ├── genome.rs        # 23-trait genetics
│   │       ├── fish.rs          # 8-state AI
│   │       ├── boids.rs         # Flocking physics
│   │       ├── ecosystem.rs     # Food, predation, species
│   │       ├── persistence.rs   # SQLite I/O
│   │       ├── config.rs        # 40+ parameters
│   │       ├── events.rs        # Environmental events
│   │       ├── achievements.rs  # Achievement tracking
│   │       ├── scenarios.rs     # Challenge modes
│   │       └── ollama.rs        # LLM integration
│   ├── Cargo.toml               # Rust dependencies
│   └── tauri.conf.json          # Desktop app config
│
├── docs/                        # Documentation
│   ├── ARCHITECTURE.md          # System design
│   ├── GENETICS.md              # Genetic algorithm
│   └── BUILDING.md              # This file
│
├── .github/
│   ├── workflows/               # CI/CD pipelines
│   │   ├── test.yml
│   │   ├── build.yml
│   │   └── release.yml
│   ├── CONTRIBUTING.md          # Contribution guide
│   ├── RELEASE.md               # Release process
│   └── scripts/
│       ├── bump-version.sh      # Version management
│       └── generate-changelog.sh
│
├── package.json                 # Frontend dependencies
├── tsconfig.json                # TypeScript config
├── vite.config.ts               # Vite bundler config
├── CHANGELOG.md                 # Release notes
└── README.md                    # Project overview
```

## Troubleshooting

### "pango-sys build failed" or "glib-2.0.pc missing" (Linux)

**Error:**
```
error: pkg-config for pango failed
error: could not find pango.pc
```

**Solution:**
Install GLib development files:
```bash
sudo apt install libglib2.0-dev pkg-config
```

Then retry build.

### "pkg-config not found" (macOS)

**Error:**
```
pkg-config: command not found
```

**Solution:**
```bash
brew install pkg-config
```

### Tauri window won't open (any OS)

**Symptoms:** `npm run tauri dev` compiles but window doesn't appear.

**Try:**
```bash
rm -rf src-tauri/target
npm run tauri dev
```

The target directory can get corrupted. Rebuilding from scratch often fixes it.

### Port 5173 already in use

**Error:**
```
Error listening on 127.0.0.1:5173
```

**Solution:**
Either:
```bash
# Kill the process using port 5173
lsof -ti:5173 | xargs kill -9   # macOS/Linux
netstat -ano | findstr :5173    # Windows (find PID, then kill it)

# Or specify different port
npm run dev -- --port 5174
```

### TypeScript errors in editor but tests pass

**Symptoms:** VSCode shows red squiggles, but `npm test` passes.

**Solution:**
- Restart TypeScript server: `Cmd+Shift+P` → "TypeScript: Restart TS Server"
- Ensure you're using workspace TypeScript:
  - In VSCode: `Cmd+Shift+P` → "TypeScript: Select TypeScript Version"
  - Choose "Use Workspace Version"

### Rust compilation takes forever (first build)

**Normal.** First build compiles all Tauri dependencies from scratch. Subsequent builds are faster.

**To speed up subsequent builds:**
- Keep `cargo check` running in background while developing
- Use `cargo build --release` for final build (optimized but slower to compile)

### Tests fail with "Unexpected character" (Windows)

**Cause:** Node.js version too old or npm cache corrupted.

**Solution:**
```bash
node --version  # Should be >= 20.0.0

npm cache clean --force
rm package-lock.json
npm install
npm test
```

### Database locked error during save

**Symptoms:** Console shows "database is locked"

**Cause:** SQLite WAL mode with concurrent read/write.

**Solution:** This is usually transient and resolves in seconds. If persistent:
```bash
# Delete WAL files
rm ~/.config/deeptank/tanks/*.db-wal
rm ~/.config/deeptank/tanks/*.db-shm
```

## Git Workflow

### Create a Feature Branch

```bash
git checkout -b feature/my-feature
```

Branch naming conventions:
- `feature/...` — new features
- `fix/...` — bug fixes
- `docs/...` — documentation
- `refactor/...` — code cleanup

### Commit with Conventional Style

```bash
git add .
git commit -m "feat: add pause button to toolbar"
```

Commit message format: `{type}: {description}`

Types:
- `feat:` — new feature
- `fix:` — bug fix
- `docs:` — documentation
- `refactor:` — code cleanup
- `test:` — test additions
- `chore:` — dependency updates, build config

### Run Tests Before Pushing

```bash
npm test          # Frontend tests
npx tsc --noEmit  # Type checking
npm run build     # Verify production build

cd src-tauri && cargo test  # Rust tests (if setup allows)
cd ..

git push origin feature/my-feature
```

### Open a Pull Request

1. Go to https://github.com/saagar210/DeepTank/pulls
2. Click "New Pull Request"
3. Select your branch
4. Fill in title and description
5. Link related issues (e.g., "Closes #42")
6. Wait for CI checks to pass

### Code Review

- Address feedback in new commits
- Don't force-push to branches with open PRs (makes review history confusing)
- Once approved, squash and merge

## Performance Tips

### Development

- Keep `npm run tauri dev` running while editing
- Use Chrome DevTools Performance tab to profile (F12 → Performance)
- Monitor canvas FPS: `requestAnimationFrame` hook logs to console

### Building

- Use release mode for accurate performance: `npm run tauri build`
- Check bundle size: `npm run build` then `ls -lh dist/`
- Large bundles indicate unused dependencies

## Extension Points

### Adding a UI Component

1. Create `src/components/MyPanel.tsx`
2. Import in `src/App.tsx`
3. Add toggle state and panel visibility
4. Invoke Tauri commands for data

### Adding a Simulation Parameter

1. Add field to `Config` struct in `src-tauri/src/simulation/config.rs`
2. Implement getter/setter in `lib.rs`
3. Add slider in `SettingsPanel.tsx`
4. Apply in simulation loop

### Adding a Trait to the Genome

1. Add field to `Genome` struct in `genome.rs`
2. Initialize in `new()` and `random()`
3. Implement inheritance in `crossover()`
4. Implement mutation in `mutate()`
5. Add to sprite generation if visual

## Documentation

See also:
- [ARCHITECTURE.md](./ARCHITECTURE.md) — System design
- [GENETICS.md](./GENETICS.md) — Genetic algorithm details
- [CONTRIBUTING.md](../.github/CONTRIBUTING.md) — Contribution guidelines
- [RELEASE.md](../.github/RELEASE.md) — Release process

## Getting Help

- **Issue:** Found a bug? Open [GitHub issue](https://github.com/saagar210/DeepTank/issues)
- **Question:** Have questions? Open [GitHub discussion](https://github.com/saagar210/DeepTank/discussions)
- **PR:** Ready to contribute? See [CONTRIBUTING.md](../.github/CONTRIBUTING.md)

---

**Happy building! 🐠**
