# DeepTank Architecture

## System Overview

DeepTank is a desktop aquarium simulator with two primary execution layers designed for high-performance simulation with responsive UI rendering.

```
┌─────────────────────────────────────────────────────────┐
│           React UI (60fps Canvas Rendering)             │
├─────────────────────────────────────────────────────────┤
│          Tauri IPC Bridge (Command/Event)               │
├─────────────────────────────────────────────────────────┤
│    Rust Simulation Engine (30Hz Dedicated Thread)       │
├─────────────────────────────────────────────────────────┤
│           SQLite Database (Local File)                  │
└─────────────────────────────────────────────────────────┘
```

### Layer 1: Simulation Engine (Rust, 30Hz)

Runs on a dedicated thread, updating game state at 30 ticks/second (~33ms per tick, typically 2-5ms actual).

**Core Modules:**

| Module | Purpose | Lines |
|--------|---------|-------|
| **genome.rs** | 23-trait genetic system, inheritance, mutations | 532 |
| **fish.rs** | 8-state behavioral AI, life cycle, energy | 578 |
| **boids.rs** | Flocking physics, separation/alignment/cohesion, spatial grid | 513 |
| **ecosystem.rs** | Food system, predation, reproduction, species clustering, water quality | 1,634 |
| **persistence.rs** | SQLite schema, save/load, snapshots | 631 |
| **events.rs** | Environmental triggers (5 event types) | 296 |
| **config.rs** | 40+ tunable parameters | 226 |
| **achievements.rs** | 15 achievement conditions and tracking | 180 |
| **scenarios.rs** | 5 challenge modes | 250 |
| **ollama.rs** | LLM integration for species naming/narration | 190 |

**Data Flow (Per 30Hz Tick):**

1. **Lock** `Mutex<SimulationState>` (read current state)
2. **Physics:** Update all fish positions using boids forces
3. **Behavior:** Update 8-state AI for each fish
4. **Ecology:** Process food decay, predation, reproduction, water quality
5. **Events:** Check environmental event triggers
6. **Achievement:** Check achievement conditions
7. **Persistence:** Auto-save to SQLite (every 5000 ticks)
8. **Emit:** Send `frame-update` event to UI with delta
9. **Unlock** `Mutex<SimulationState>`

**Performance Characteristics:**

- **Tick Time:** ~2-5ms (comfortable headroom on 33ms budget)
- **Fish Count:** Optimized for 100-500 fish
- **Spatial Grid:** 25x25 grid for O(n) boids instead of O(n²)
- **Database:** WAL mode, non-blocking writes

### Layer 2: React Frontend (60fps)

Renders directly to canvas every frame via `requestAnimationFrame`. Does NOT re-render React tree on every frame (direct canvas manipulation for performance).

**Modules:**

| Module | Purpose | Type |
|--------|---------|------|
| **App.tsx** | Main orchestration, IPC commands, event listeners | Component |
| **canvasRenderer.ts** | Direct 2D drawing, sprite caching, interpolation | Util |
| **fishSprite.ts** | Procedural fish sprite generation (no images) | Util |
| **audioEngine.ts** | Web Audio API procedural synthesis | Util |
| **components/** | 15 React UI panels | Components |

**Canvas Rendering Pipeline:**

1. Receive `frame-update` from simulation (throttled 4Hz for React state)
2. Update React state (selected fish, panels, etc.)
3. React re-renders UI components only (not canvas)
4. `requestAnimationFrame` triggers canvas draw:
   - Clear canvas
   - Draw background, terrain
   - Interpolate fish positions between simulation frames
   - Draw each fish sprite (cached)
   - Draw effects (ripples, particles, alerts)
   - Draw UI overlays

**Why No React Re-renders on Canvas?**
- React is ~30fps max (batched updates)
- Canvas needs 60fps for smooth motion
- Direct canvas writes bypass React's reconciliation (saves ~20ms per frame)
- Fish positions interpolated between 30Hz simulation ticks

**UI Components (15 total):**

| Component | Purpose |
|-----------|---------|
| TopBar | Header with stats, toggles for panels |
| Toolbar | Simulation controls (pause, speed, feed, mute) |
| Inspector | Fish detail viewer, genome inspector |
| StatsPanel | Population graphs, diversity metrics |
| SettingsPanel | 30+ sliders for tuning simulation |
| SpeciesGallery | Cards showing each species info |
| PhylogeneticTree | Lineage visualization |
| ReplayControls | Time-series graphs and snapshots |
| BreedingPanel | Manual breeding interface |
| AchievementPanel | 15 achievement tracker |
| DecorationPalette | Decoration picker |
| TankSwitcher | Multi-tank management |
| ScenarioPanel | 5 challenge modes |
| NarrationTicker | AI-generated narration scroll |
| Toasts | Non-blocking notifications |

### Layer 3: IPC Bridge (Tauri)

56+ commands handle communication between Rust simulation and React frontend.

**Command Categories:**

**Simulation Control:**
- `pause_simulation()` / `resume_simulation()`
- `set_speed_multiplier(f32)`
- `step_forward()`
- `reset_tank()`

**UI State Sync:**
- `get_frame_state()` → Current frame (30Hz, throttled to 4Hz)
- `select_fish(id)` → Fish details
- `get_stats()` → Population snapshot

**Settings & Config:**
- `update_config(key, value)` → Tunable parameters
- `get_config_defaults()` → Reset to defaults

**File I/O:**
- `save_aquarium(path)` / `load_aquarium(path)`
- `export_population_csv()`

**Multi-Tank:**
- `create_tank(name)` / `delete_tank(id)`
- `switch_tank(id)`

### Layer 4: Database (SQLite, Local File)

**Location:** `~/.config/deeptank/tanks/tank_name.db` (per-tank databases)

**Schema (11 tables):**

| Table | Purpose |
|-------|---------|
| schema_version | Version tracking for migrations |
| aquarium | Tank metadata (tick_count, water_quality, timestamps) |
| settings | Config key-value pairs |
| genomes | 23-field genome records (primary data) |
| fish | Active fish state + references |
| species | Species definitions + trait centroids |
| population_snapshots | Historical population (1000-tick intervals) |
| species_snapshots | Per-species population history |
| events | Logged births, deaths, predation, extinction |
| journal_entries | AI-generated narration entries |
| achievements | Achievement unlock records |
| decorations | Tank decoration placements |

**Features:**
- WAL mode for non-blocking reads
- Foreign keys enabled
- Auto-save every 5000 ticks
- Snapshot every 1000 ticks
- Full state restoration on load

## Thread Safety & Concurrency

**Simulation Thread (Rust):**
```
loop {
  state = db.load()?;           // Read from DB
  state.step();                 // Compute physics, AI, ecology
  db.save(&state)?;             // Write snapshot
  emit_frame_update(state);     // Send to UI
  sleep(33ms);
}
```

**UI Thread (React/JavaScript):**
```
window.addEventListener('frame-update', (event) => {
  setCurrentFrame(event.detail);  // Trigger React re-render
  requestAnimationFrame(() => {
    drawCanvas(event.detail);     // Direct canvas write
  });
});
```

**Locking Strategy:**
- Simulation holds `Mutex<SimulationState>` exclusively during tick
- UI reads snapshot (no locking, safe because Rust owned types prevent data races)
- IPC commands use proper error handling for state conflicts

## Performance Optimization Techniques

### 1. Spatial Grid (Boids Physics)
Instead of O(n²) distance checks for each fish pair, divide canvas into 25x25 grid:
```
Each fish only checks neighbors in adjacent grid cells → O(n) average
10x-25x speedup with 100+ fish
```

### 2. Sprite Caching (Canvas Rendering)
Fish sprites are cached based on genome + pattern:
```
First occurrence: Generate sprite (procedural, ~5ms)
Cache in memory: { genomeId → HTMLImageElement }
Reuse: All fish with same genome draw cached sprite (no regeneration)
Result: 60fps even with many identical phenotypes
```

### 3. Event Throttling (UI Updates)
Simulation emits 30Hz, UI processes 4Hz:
```
Simulation: Every frame, emit frame-update
UI: Process every ~8th frame (250ms)
Result: UI stays responsive, not overloaded with events
Canvas: Interpolates between 30Hz frames to show 60fps motion
```

### 4. Direct Canvas Drawing
No React re-renders for canvas:
```
React: Paint UI components (buttons, panels) ~20-30fps
Canvas: Direct 2D draws (fish, background) 60fps, unaffected by React
Result: High FPS canvas + responsive UI
```

## Type Safety

### Rust Type System
- No `unsafe` code in simulation logic
- Strong guarantees on genome mutations (can't have invalid traits)
- Compile-time fish state validation (can't be in two states at once)

### TypeScript Strict Mode
- `"strict": true` in tsconfig.json
- No `any` types allowed
- All IPC contracts defined in `src/types.ts`

**Type Contract Example:**
```typescript
// src/types.ts (source of truth)
interface FrameState {
  tick: number;
  fish: Fish[];
  species: Species[];
  waterQuality: number;
  // ...
}

// src-tauri/src/lib.rs
#[tauri::command]
pub fn get_frame_state() -> Result<FrameState, String> {
  // Rust struct serialized to JSON matches TypeScript interface
}

// src/App.tsx
const [frame, setFrame] = useState<FrameState | null>(null);
// TypeScript enforces correct shape
```

## Error Handling Strategy

### Graceful Degradation
- Ollama unavailable? Fall back to deterministic naming
- Audio context not initialized? Continue without sound
- Database write fails? Retry with exponential backoff
- Invalid config value? Clamp to valid range

### Validation
- All Tauri command params validated before use
- Type contracts prevent invalid IPC messages
- Database constraints prevent corrupted data

### Logging
- Simulation errors logged to stderr with context
- UI errors logged to browser console
- CI/CD test failures show in GitHub Actions UI

## Scaling Considerations

**Current Design Handles:**
- 100-500 fish efficiently
- 10-20 species concurrently
- 50+ hours of runtime (snapshots prevent unbounded memory)
- Thousands of population snapshots

**Future Scaling (Post-1.0):**
- **WebGL rendering** for 1000+ fish (replace Canvas 2D)
- **Chunked snapshots** for years of data (compress old snapshots)
- **Multi-threaded physics** (Rayon for CPU-bound boids)
- **Streaming saves** (incremental snapshots instead of full DB writes)

## Module Dependencies

```
lib.rs (main entry)
  ├─ simulation::mod
  │   ├─ genome.rs (standalone)
  │   ├─ fish.rs (uses genome)
  │   ├─ boids.rs (uses fish)
  │   ├─ ecosystem.rs (uses fish, genome)
  │   ├─ events.rs (modifies ecosystem)
  │   ├─ achievements.rs (observes events)
  │   ├─ persistence.rs (serializes all above)
  │   ├─ config.rs (tunable for all above)
  │   ├─ scenarios.rs (wraps ecosystem with goals)
  │   └─ ollama.rs (optional, for naming)
  └─ tauri::* (IPC bindings)

App.tsx (React entry)
  ├─ types.ts (contracts)
  ├─ renderer/canvasRenderer.ts
  │   └─ fishSprite.ts
  ├─ audio/audioEngine.ts
  └─ components/* (UI panels)
```

No circular dependencies. Clear layering.

## Design Decisions

### Why Rust for Simulation?
- **Performance:** 100x faster than JavaScript for physics loop
- **Memory safety:** No garbage collection pauses during tick
- **Concurrency:** Safe multithreading with type system
- **Native:** Direct file system access, no sandbox

### Why React for UI?
- **Reactivity:** State changes automatically update UI
- **Component model:** 15 independent panels, easy to extend
- **Ecosystem:** Vitest testing, TypeScript support
- **Interop:** Tauri provides seamless JS-Rust bridge

### Why Canvas 2D, not WebGL?
- **Simplicity:** Easier to debug rendering
- **Portability:** Works on older systems
- **Sufficient:** 60fps with 500 fish and sprite cache
- **Future:** Can upgrade to WebGL if needed

### Why SQLite, not Server?
- **Local:** No network dependency, faster saves
- **Portable:** Single file, easy to backup/share
- **Simple:** No schema migration complexity
- **Privacy:** All data stays on user's machine

## Future Architecture Changes

**Phase 4+ Enhancements:**

1. **Renderer Update**
   - Replace Canvas 2D with WebGL (1000+ fish)
   - Add particle effects for water currents
   - Advanced lighting/shadows

2. **Physics Improvements**
   - Multi-threaded boids with Rayon
   - GPU-accelerated neighbor detection

3. **Database Optimizations**
   - Snapshot compression
   - Incremental saves
   - Historical data export

4. **Educational Features**
   - Visual explanations of genetics
   - Parameter sensitivity analysis
   - Gene flow visualization

---

**Architecture is designed for clarity, performance, and maintainability. Easy to extend without refactoring core systems.**
