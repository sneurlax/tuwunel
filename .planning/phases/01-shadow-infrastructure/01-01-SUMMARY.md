---
phase: 01-shadow-infrastructure
plan: 01
subsystem: infra
tags: [shadow, cargo-profile, serde-yaml, toml, io-uring, build-system]

# Dependency graph
requires: []
provides:
  - "[profile.shadow] Cargo build profile for Shadow simulation builds"
  - "compile_error guard preventing io_uring + shadow feature combination"
  - "shadow_features convenience feature set excluding io_uring"
  - "tests/shadow/ workspace crate with ShadowConfig YAML generation"
  - "tests/shadow/ workspace crate with TuwunelConfig TOML generation"
affects: [01-02-PLAN, 01-03-PLAN]

# Tech tracking
tech-stack:
  added: [serde_yaml (Shadow config), tempfile (test isolation)]
  patterns: [Shadow YAML generation via serde Serialize, tuwunel TOML generation via toml crate, marker feature for build-time guards]

key-files:
  created:
    - tests/shadow/Cargo.toml
    - tests/shadow/src/lib.rs
    - tests/shadow/src/config/mod.rs
    - tests/shadow/src/config/shadow.rs
    - tests/shadow/src/config/tuwunel.rs
    - tests/shadow/src/bin/matrix_test_client.rs
  modified:
    - Cargo.toml
    - src/main/Cargo.toml
    - src/main/lib.rs

key-decisions:
  - "Used marker feature shadow=[] instead of cfg flag for compile_error guard (Cargo stable lacks per-profile rustflags)"
  - "Added serde_json with std feature since workspace dep has default-features=false"
  - "shadow_features includes all defaults except io_uring and systemd"

patterns-established:
  - "Shadow config structs use BTreeMap for deterministic key ordering"
  - "Default impls for Shadow structs encode research-validated safe defaults (seed=42, model_unblocked_syscall_latency=true)"
  - "tests/shadow in workspace members but NOT default-members"

requirements-completed: [SHAD-01, SHAD-03, SHAD-04, SHAD-07, SHAD-08, CONF-02, CONF-03]

# Metrics
duration: 4min
completed: 2026-03-26
---

# Phase 01 Plan 01: Shadow Build Profile and Config Generation Summary

**Shadow build profile with io_uring compile guard, and typed Shadow YAML + tuwunel TOML config generation in tests/shadow/ crate**

## Performance

- **Duration:** 4 min
- **Started:** 2026-03-26T01:24:59Z
- **Completed:** 2026-03-26T01:28:54Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments
- Shadow build profile (`[profile.shadow]`) inheriting from release with debug symbols
- compile_error! guard preventing accidental io_uring + shadow feature combination
- Shadow YAML config generation with typed structs (ShadowConfig, General, Network, Host, Process)
- Tuwunel TOML config generation with programmatic construction and deterministic defaults
- shadow_features convenience feature set for single-command Shadow builds

## Task Commits

Each task was committed atomically:

1. **Task 1: Add shadow Cargo profile and io_uring compile guard** - `7ac8b816` (feat)
2. **Task 2: Create shadow test harness crate with config generation** - `3f7d8d12` (feat)

## Files Created/Modified
- `Cargo.toml` - Added tests/shadow to workspace members, added [profile.shadow]
- `src/main/Cargo.toml` - Added shadow marker feature and shadow_features convenience set
- `src/main/lib.rs` - Added compile_error! for io_uring + shadow
- `tests/shadow/Cargo.toml` - New crate with serde, serde_yaml, toml, tempfile deps
- `tests/shadow/src/lib.rs` - Crate root exporting config module
- `tests/shadow/src/config/mod.rs` - Config module exporting shadow and tuwunel submodules
- `tests/shadow/src/config/shadow.rs` - Shadow YAML config structs with serde Serialize
- `tests/shadow/src/config/tuwunel.rs` - Tuwunel TOML config struct with programmatic construction
- `tests/shadow/src/bin/matrix_test_client.rs` - Placeholder binary (implemented in Plan 02)

## Decisions Made
- Used marker feature `shadow = []` instead of `--cfg shadow` rustflag because Cargo stable lacks per-profile rustflags
- Added `serde_json` with explicit `std` feature since workspace dep has `default-features = false`
- shadow_features includes all default features except io_uring and systemd (systemd unnecessary under Shadow)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added std feature to serde_json dependency**
- **Found during:** Task 2 (cargo check)
- **Issue:** Workspace serde_json has default-features=false without std; shadow-test-harness failed to compile
- **Fix:** Added features = ["std"] to serde_json in tests/shadow/Cargo.toml
- **Files modified:** tests/shadow/Cargo.toml
- **Verification:** cargo check --package shadow-test-harness succeeds
- **Committed in:** 3f7d8d12 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary fix for compilation. No scope creep.

## Issues Encountered
None beyond the auto-fixed serde_json std feature.

## Known Stubs
- `tests/shadow/src/bin/matrix_test_client.rs` - Placeholder binary with `unimplemented!()`. Will be implemented in Plan 02 (01-02-PLAN.md).

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Shadow build profile and config generation ready for Plan 02 (test client binary)
- Plan 02 will add matrix-sdk, clap, reqwest, tokio deps and implement the test client
- Plan 03 will wire everything together into a Shadow smoke test

## Self-Check: PASSED

All 6 created files verified present. Both task commits (7ac8b816, 3f7d8d12) verified in git log.

---
*Phase: 01-shadow-infrastructure*
*Completed: 2026-03-26*
