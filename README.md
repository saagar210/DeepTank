# DeepTank

A living desktop aquarium where fish evolve, hunt, school, and speciate — powered by real genetics, boids physics, and way too much Rust.

![Tauri 2](https://img.shields.io/badge/Tauri-2-blue) ![React 19](https://img.shields.io/badge/React-19-61dafb) ![Rust](https://img.shields.io/badge/Rust-edition%202021-orange) ![Canvas 2D](https://img.shields.io/badge/Rendering-Canvas%202D-green)

## What is this?

Drop some fish in a tank. Watch them eat, breed, fight, and evolve. Come back later and find entirely new species you never designed. Every fish has a unique genome with 23 heritable traits — color, speed, aggression, fin shape, pattern, metabolism — all subject to mutation, natural selection, and the occasional algae bloom.

## Features

**Genetics & Evolution**
- 23-trait genome system with dominant/blended inheritance and circular hue math
- Two-tier mutations (2% large, 10% small) with inbreeding penalties
- Automatic species detection via single-linkage clustering on genome distance
- Shannon-Wiener diversity index tracked in real-time

**Behavioral AI**
- 8 behavioral states: Swimming, Foraging, Fleeing, Satiated, Courting, Resting, Hunting, Dying
- Boids flocking (separation/alignment/cohesion) with spatial grid acceleration
- Pack hunting with coordinated strikes and safety-in-numbers defense
- Territory claiming, disease spreading, and coordinated school fleeing

**Ecosystem**
- 3 food types (flake, pellet, live) with different sink rates and nutrition
- Water quality system affected by population, plants, and decayed food
- Egg laying near decorations with incubation periods and egg predation
- Auto-feeder, day/night cycle, and 4 decoration types that affect water recovery

**Environmental Events**
- Algae Bloom, Cold Snap, Heatwave, Current Surge, Plankton Bloom
- Each temporarily modifies metabolism, water quality, or current forces

**UI & Interaction**
- Canvas renderer at 60fps interpolating 30Hz simulation ticks
- 4 visual themes: Aquarium, Tropical, Deep Ocean, Freshwater
- Fish inspector with full genome details and lineage tree
- Manual breeding panel with offspring trait preview
- Species gallery, population graphs, achievement system (15 achievements)
- 5 challenge scenarios (Survival, Apex Predator, Biodiversity, Peaceful Kingdom, Ice Age)
- Multi-tank support, keyboard shortcuts, zoom/pan, glass tapping
- Procedural audio (Web Audio API) for ambient underwater sounds and event effects

**AI Integration**
- Optional Ollama connection for AI-generated species names, journal entries, and narration ticker

**Persistence**
- SQLite (WAL mode) with auto-save — full state restoration across sessions
- Population snapshots for historical graphs and replay

## Tech Stack

| Layer | Tech |
|---|---|
| Desktop shell | Tauri 2 |
| Simulation | Rust (30Hz tick loop on dedicated thread) |
| Frontend | React 19 + TypeScript (strict mode) |
| Rendering | Canvas 2D with DPR scaling + sprite caching |
| Database | SQLite via rusqlite (bundled) |
| Audio | Web Audio API (procedural synthesis) |
| AI | Ollama (optional, local LLM) |

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (v18+)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

### Run

```bash
npm install
npm run tauri dev
```

### Build

```bash
npm run tauri build
```

### Test

```bash
# Rust unit tests
cd src-tauri && cargo test

# Frontend tests
npx vitest run
```

## Keyboard Shortcuts

| Key | Action |
|---|---|
| `Space` | Pause / Resume |
| `F` | Drop food |
| `S` | Toggle stats panel |
| `M` | Mute / Unmute |
| `B` | Toggle breeding mode |
| `P` | Screenshot |
| `0` | Reset zoom |
| `Scroll` | Zoom in/out |
| `Alt+Drag` | Pan viewport |
| `Double-click` | Tap glass |

## Architecture

```
Simulation Thread (30Hz)          Canvas Renderer (60fps)
 Lock Mutex<SimState>              Interpolate between frames
 step(): physics, AI, ecology      Draw sprites + effects
 Emit frame-update event           No React re-render
 Unlock                            Direct canvas writes
         |                                  |
         +------ React State (4Hz) ---------+
                 Throttled UI updates
                 Component re-renders
```

## Project Structure

```
src-tauri/src/
  simulation/
    genome.rs        # 23-trait genetic system
    fish.rs          # Behavioral state machine
    boids.rs         # Flocking physics + spatial grid
    ecosystem.rs     # Food, eggs, species, predation, disease
    config.rs        # 40+ tunable parameters
    events.rs        # Environmental event system
    achievements.rs  # 15 unlockable achievements
    scenarios.rs     # 5 challenge scenarios
    persistence.rs   # SQLite schema + save/load
    ollama.rs        # LLM integration
src/
  App.tsx            # Main app + canvas orchestration
  renderer/          # Canvas 2D renderer + sprite generation
  audio/             # Procedural Web Audio engine
  components/        # 16 React UI panels
  types.ts           # Shared TypeScript interfaces
```

## License

MIT
