# Delta Plan

## A) Executive Summary

### Current state (repo-grounded)
- Desktop app built with Tauri 2 (Rust backend) and React 19 + Vite frontend.【src-tauri/Cargo.toml, package.json】
- Core simulation is concentrated under `src-tauri/src/simulation/*` with genetics, fish behavior, boids, ecosystem, persistence, scenarios, events.
- Frontend canvas orchestration lives mostly in `src/App.tsx` and renderer code under `src/renderer/*`.
- Audio is a procedural Web Audio engine in `src/audio/audioEngine.ts` with unit tests in `src/audio/audioEngine.test.ts`.
- Baseline frontend tests and build are green locally.
- Rust test baseline is blocked by missing Linux system dependency (`glib-2.0` pkg-config file) required for Tauri stack compilation.

### Key risks
- Environment-specific Rust verification failures can hide regressions if not documented clearly.
- Settings values are forwarded to audio engine without runtime numeric guardrails in the setter path.
- Long autonomous sessions risk context loss without durable checkpointing artifacts.

### Improvement themes (prioritized)
1. Session resilience + auditable planning/logging artifacts (`codex/*`).
2. Hardening of audio settings handling (clamp + invalid input guard).
3. Verification clarity and environment troubleshooting documentation.

## B) Constraints & Invariants (Repo-derived)

### Explicit invariants
- Keep current stack and architecture (Tauri + Rust simulation + React/Canvas).
- Preserve existing behavior of audio methods as no-throw no-ops when not initialized.
- Maintain existing frontend test suite passing.

### Implicit invariants (inferred)
- `master_volume` represents normalized gain-like value (expected range 0..1).
- Muted state should always force output gain to 0 regardless of stored master volume.
- Settings updates should be side-effect safe and not crash UI loops.

### Non-goals
- No broad refactor of simulation architecture.
- No persistence schema changes.
- No CI pipeline redesign.

## C) Proposed Changes by Theme (Prioritized)

### Theme 1: Session resilience artifacts
- **Current approach:** No dedicated codex runbook files in repo root.
- **Proposed change:** Add and maintain `codex/SESSION_LOG.md`, `codex/PLAN.md`, `codex/DECISIONS.md`, `codex/CHECKPOINTS.md`, `codex/VERIFICATION.md`, `codex/CHANGELOG_DRAFT.md`.
- **Why:** Enables interruption-safe autonomous execution with clear resume state.
- **Tradeoffs:** Adds process documentation overhead.
- **Scope boundary:** Documentation only.
- **Migration:** N/A.

### Theme 2: Audio settings hardening
- **Current approach:** `masterVolume` setter stores and applies incoming number directly.
- **Proposed change:** Clamp non-finite/out-of-range values to `[0, 1]` and add tests.
- **Why:** Prevents unexpected gain behavior from malformed setting values.
- **Tradeoffs:** Slight behavior change for invalid inputs (now normalized rather than passed through).
- **Scope boundary:** `src/audio/audioEngine.ts` and related tests only.
- **Migration:** Backward compatible.

### Theme 3: Verification troubleshooting clarity
- **Current approach:** README has test commands but no Linux package troubleshooting for Tauri deps.
- **Proposed change:** Add concise troubleshooting note for missing `glib-2.0` pkg-config prerequisite.
- **Why:** Improves developer onboarding and baseline verification repeatability.
- **Tradeoffs:** Adds platform-specific note to docs.
- **Scope boundary:** README only.
- **Migration:** N/A.

## D) File/Module Delta (Exact)

### ADD
- `codex/SESSION_LOG.md` — chronological implementation log.
- `codex/PLAN.md` — this plan.
- `codex/DECISIONS.md` — judgment calls and alternatives.
- `codex/CHECKPOINTS.md` — resumable checkpoints.
- `codex/VERIFICATION.md` — command evidence.
- `codex/CHANGELOG_DRAFT.md` — delivery draft notes.

### MODIFY
- `src/audio/audioEngine.ts` — add volume sanitization/clamping helper and setter hardening.
- `src/audio/audioEngine.test.ts` — add tests for clamped/non-finite values.
- `README.md` — add Rust/Tauri Linux dependency troubleshooting note.

### REMOVE/DEPRECATE
- None.

### Boundary rules
- No changes to Rust simulation logic in this iteration.
- No cross-cutting UI state refactor.

## E) Data Models & API Contracts (Delta)
- **Current:** Audio API is class setters/getters, no explicit external schema.
- **Proposed:** `masterVolume` setter contract tightened to normalized finite range [0,1].
- **Compatibility:** Backward compatible for valid inputs; invalid values now coerced.
- **Migrations:** None.
- **Versioning:** Internal behavior hardening only.

## F) Implementation Sequence (Dependency-Explicit)
1. Create codex artifact files with discovery evidence and checkpoints.
   - Verify: markdown lint not configured; run `npm test` to ensure no code breakage from docs edits.
   - Rollback: remove new files.
2. Implement audio clamp helper + setter updates.
   - Verify: `npm test`.
   - Rollback: revert `audioEngine.ts`.
3. Add unit tests for clamping and non-finite volume inputs.
   - Verify: `npm test`.
   - Rollback: revert test additions.
4. Add README troubleshooting section for missing glib pkg-config.
   - Verify: `npm run build`.
   - Rollback: revert README edits.
5. Run final full suite available in environment.
   - Verify: `npm test`, `npm run build`, `cargo test` (expected env warning documented).

## G) Error Handling & Edge Cases
- **Current pattern:** Guard clauses (`if (!ctx) return`) and safe no-throw behavior in audio methods.
- **Improvement:** Prevent invalid numeric propagation in `masterVolume`.
- **Edge cases:**
  - `NaN`, `Infinity`, `-Infinity` volume input.
  - Volume <0 and >1 bounds.
  - muted toggle while volume changed.
- **Tests:** Extend audio unit tests for clamp behavior.

## H) Integration & Testing Strategy
- Integration points: `App.tsx` settings update path into `AudioEngine.masterVolume`.
- Unit tests: update `src/audio/audioEngine.test.ts`.
- Regression checks: run existing Vitest suite + frontend build.
- DoD:
  - Tests pass.
  - Build passes.
  - Cargo test status documented with env blocker details.

## I) Assumptions & Judgment Calls
### Assumptions
- Valid UI-generated `master_volume` should already be in 0..1 but may drift via persisted config or external inputs.
- Documenting Linux dependency blocker in README is acceptable within current docs style.

### Judgment calls
- Chose targeted hardening over larger settings validation refactor.
- Kept Rust code untouched due environment blocker and need for small, reversible delta.
