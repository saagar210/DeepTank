# Decisions Log

## 2026-02-10
1. **Decision:** Limit implementation scope to targeted hardening and operational docs.
   - **Why:** Preserve repo stability while improving reliability and resume-ability.
   - **Alternatives considered:** broad refactor of settings pipeline (rejected as high risk).

2. **Decision:** Treat `cargo test` failure as environment limitation, not code failure.
   - **Why:** Error indicates missing system package (`glib-2.0.pc`), not Rust compile/type issue in project code.
   - **Alternatives considered:** introducing cargo feature gates for tests (rejected; would alter project contracts and hide dependency requirement).

3. **Decision:** Clamp `masterVolume` values to `[0,1]` and coerce non-finite values to default.
   - **Why:** Defensive programming for persisted or externally-injected config values.
   - **Alternatives considered:** throw on invalid value (rejected to keep no-throw setter behavior).
