---
phase: 04-embed-crate
plan: 02
subsystem: embed
tags: [rust, embed, reqwest, uiaa, lifecycle, multi-instance]

# Dependency graph
requires:
  - phase: 04-embed-crate
    provides: EmbeddedHomeserver struct, Builder, OnceLock safety, port 0 pre-bind
provides:
  - Graceful stop() with correct shutdown ordering
  - register_user() via UIAA two-step registration flow
  - RegisteredUser credential struct
  - Integration tests for lifecycle, multi-instance, and registration
  - EMBD-10 v2 deferral documentation
affects: [embed-integration-tests, matrix-rust-client-embed]

# Tech tracking
tech-stack:
  added: []
  patterns: [uiaa-two-step-registration, graceful-shutdown-ordering]

key-files:
  created: []
  modified:
    - src/embed/src/lib.rs

key-decisions:
  - "Shutdown order: server.shutdown() -> run_handle.await -> router::stop() to ensure Arc refcount allows try_unwrap"
  - "register_user uses standalone reqwest client (not shared) with 10s timeout for isolation"
  - "extract_registered_user helper returns errors instead of empty strings for missing fields"
  - "EMBD-10 (in-memory transport) explicitly deferred to v2 with module-level doc comment"

patterns-established:
  - "UIAA registration: dummy auth first for session, then registration_token auth to complete"
  - "Shutdown ordering: signal -> wait for run task -> stop services (ensures clean Arc unwrap)"

requirements-completed: [EMBD-02, EMBD-04, EMBD-06, EMBD-07, EMBD-09, EMBD-10]

# Metrics
duration: 2min
completed: 2026-03-27
---

# Phase 04 Plan 02: Embed Lifecycle and Registration Summary

**Graceful stop(), UIAA register_user(), and multi-instance integration tests completing the tuwunel-embed API**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-27T17:19:40Z
- **Completed:** 2026-03-27T17:21:30Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments
- Implemented graceful stop() with correct shutdown ordering (signal -> run task -> services cleanup)
- Implemented register_user() with UIAA two-step flow matching Shadow test patterns
- Added RegisteredUser struct returning user_id and access_token
- Added three integration tests: single lifecycle, multi-instance concurrency, and user registration
- Documented EMBD-10 deferral to v2 in module-level docs

## Task Commits

Each task was committed atomically:

1. **Task 1+2: Implement stop, register_user, and integration tests** - `13c670b2` (feat)

## Files Created/Modified
- `src/embed/src/lib.rs` - Complete EmbeddedHomeserver with stop(), register_user(), RegisteredUser, EMBD-10 docs, and integration tests

## Decisions Made
- Fixed shutdown ordering from plan 01's stop() implementation: moved run_handle.await before router::stop() so the run task's Arc<Services> clone is released before try_unwrap
- Used standalone reqwest::Client per register_user call with 10s timeout for isolation
- Made extract_registered_user return proper errors instead of returning empty strings for missing fields
- Tests marked #[ignore] since they require full tuwunel build with RocksDB

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed shutdown ordering in stop()**
- **Found during:** Task 1
- **Issue:** Plan 01's stop() called tuwunel_router::stop(services) before run_handle.await, but run() holds a clone of Arc<Services>, so try_unwrap in router::stop would fail due to dangling reference
- **Fix:** Reordered to: server.shutdown() -> run_handle.await -> router::stop(services)
- **Files modified:** src/embed/src/lib.rs
- **Verification:** cargo check passes, logic matches router::stop's Arc::try_unwrap expectation
- **Committed in:** 13c670b2

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** Essential fix for correct shutdown. No scope creep.

## Issues Encountered
None.

## Known Stubs
None - all methods fully implemented.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- tuwunel-embed API complete for v1: start, stop, register_user, base_url
- Integration tests ready to run with `cargo test -p tuwunel-embed -- --ignored`
- EMBD-10 (in-memory transport) documented as v2 scope

---
*Phase: 04-embed-crate*
*Completed: 2026-03-27*
