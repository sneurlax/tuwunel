---
phase: 01-shadow-infrastructure
plan: 03
subsystem: testing
tags: [shadow, integration-test, smoke-test, deterministic-testing]

# Dependency graph
requires:
  - phase: 01-01
    provides: "Shadow build profile, ShadowConfig YAML generation, TuwunelConfig TOML generation"
  - phase: 01-02
    provides: "matrix-test-client binary with smoke subcommand, run_shadow runner with ShadowResult"
provides:
  - "Integration smoke test wiring Shadow config generation, binary building, and runner invocation"
  - "End-to-end validation that Phase 1 Shadow pipeline works"
affects: []

# Tech tracking
tech-stack:
  added: []
  patterns: [integration test builds shadow-profile binaries as setup step, BTreeMap host ordering for deterministic Shadow IP assignment]

key-files:
  created:
    - tests/shadow/tests/smoke.rs
  modified: []

key-decisions:
  - "Test builds shadow-profile binaries itself via cargo subprocess rather than requiring pre-built binaries"
  - "Test uses hostname-based URL (http://tuwunel-server:8448) relying on Shadow's built-in hostname resolution"
  - "Test marked #[ignore] to avoid running in normal cargo test (requires Shadow installation and long build)"

patterns-established:
  - "Shadow smoke tests use tempdir for all generated configs and data directories"
  - "TUWUNEL_CONFIG env var passed through Shadow process environment for config path"

requirements-completed: [SHAD-02, SHAD-06, SHAD-07, SHAD-09]

# Metrics
duration: 1min
completed: 2026-03-26
---

# Phase 01 Plan 03: Shadow Smoke Integration Test Summary

**Integration test wiring ShadowConfig, TuwunelConfig, and run_shadow into a full smoke scenario with two Shadow hosts and deterministic seed validation**

## Performance

- **Duration:** 1 min
- **Started:** 2026-03-26T01:37:06Z
- **Completed:** 2026-03-26T01:38:39Z
- **Tasks:** 1
- **Files modified:** 1

## Accomplishments
- Capstone integration test exercising entire Phase 1 Shadow pipeline end-to-end
- Shadow smoke test with tuwunel-server (running state) and test-client (exited state) hosts
- Per-host stdout/stderr validation and client readiness assertion (SHAD-06)
- Failure diagnostics with seed and log paths visible in test output (SHAD-09)

## Task Commits

Each task was committed atomically:

1. **Task 1: Create Shadow smoke integration test** - `64923ea7` (feat)

## Files Created/Modified
- `tests/shadow/tests/smoke.rs` - Integration test building shadow binaries, generating configs, invoking Shadow, asserting success and per-host output

## Decisions Made
- Test builds shadow-profile binaries itself via cargo subprocess rather than requiring pre-built binaries -- ensures test is self-contained
- Test uses hostname-based URL (http://tuwunel-server:8448) relying on Shadow's built-in hostname resolution between simulated hosts
- Test marked #[ignore] to avoid running in normal cargo test since it requires Shadow installation and performs a full release build

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None.

## Known Stubs
None -- the smoke test is fully implemented and compiles.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Phase 1 Shadow infrastructure is complete: build profile, config generation, test client, runner, and smoke test
- The smoke test can be run with `cargo test -p shadow-test-harness --test smoke -- --ignored --nocapture` once Shadow is installed
- Ready for Phase 2 (basic E2E tests) which will build on this infrastructure

## Self-Check: PASSED

All created files verified present. Task commit (64923ea7) verified in git log.

---
*Phase: 01-shadow-infrastructure*
*Completed: 2026-03-26*
