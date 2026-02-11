# Changelog Draft

## Theme: Session Resilience
- Added a persistent codex runbook (`SESSION_LOG`, `PLAN`, `DECISIONS`, `CHECKPOINTS`, `VERIFICATION`) so autonomous work can be interrupted and resumed safely.

## Theme: Audio Settings Hardening
- Hardened `AudioEngine.masterVolume` handling to coerce invalid values and clamp to `[0,1]`.
- Added `masterVolume` getter for safer observability/tests.
- Added regression tests for negative, oversized, and non-finite volume values.

## Theme: Verification/Onboarding Clarity
- Added README troubleshooting guidance for Linux environments missing `glib-2.0.pc` while running Rust/Tauri tests.
