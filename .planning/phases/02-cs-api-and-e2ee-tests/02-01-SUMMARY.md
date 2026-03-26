---
phase: 02-cs-api-and-e2ee-tests
plan: 01
subsystem: testing
tags: [ruma, reqwest, shadow, matrix-cs-api, e2ee, clap]

# Dependency graph
requires:
  - phase: 01-shadow-infrastructure
    provides: Shadow test harness, runner, config builders, smoke test binary
provides:
  - Scenario module scaffold with common helpers (MatrixClient, registration, login, server readiness)
  - Multi-host Shadow config builder (three_host_config for server + alice + bob)
  - Subcommand stubs for cs-api, e2ee-messaging, sas-verify in matrix_test_client
affects: [02-cs-api-and-e2ee-tests]

# Tech tracking
tech-stack:
  added: [ruma (workspace, client-api features), tracing, tracing-subscriber]
  patterns: [MatrixClient wrapper over ruma+reqwest for CS API calls, UIAA two-step registration flow, run_in_runtime shared helper]

key-files:
  created:
    - tests/shadow/src/scenarios/mod.rs
    - tests/shadow/src/scenarios/common.rs
    - tests/shadow/src/scenarios/cs_api.rs
    - tests/shadow/src/scenarios/e2ee_msg.rs
    - tests/shadow/src/scenarios/sas_verify.rs
  modified:
    - tests/shadow/Cargo.toml
    - tests/shadow/src/lib.rs
    - tests/shadow/src/config/shadow.rs
    - tests/shadow/src/config/tuwunel.rs
    - tests/shadow/src/bin/matrix_test_client.rs

key-decisions:
  - "Used ruma + reqwest directly instead of matrix-sdk due to async-channel version conflict with workspace patches"
  - "MatrixClient wrapper struct provides same API surface as matrix-sdk::Client for CS API operations"

patterns-established:
  - "MatrixClient: reqwest-based Matrix CS API client with UIAA registration support"
  - "three_host_config: multi-host Shadow topology builder pattern for two-client scenarios"
  - "run_in_runtime: shared tokio runtime helper for all subcommands"

requirements-completed: [TEST-05, TEST-06]

# Metrics
duration: 8min
completed: 2026-03-26
---

# Phase 02 Plan 01: SDK Helpers and Scenario Scaffold Summary

**Ruma+reqwest MatrixClient with UIAA registration, multi-host Shadow topology builder, and cs-api/e2ee/sas subcommand stubs**

## Performance

- **Duration:** 8 min
- **Started:** 2026-03-26T02:29:37Z
- **Completed:** 2026-03-26T02:37:36Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments
- Scenario module scaffold with common helpers for SDK client creation, UIAA token registration, login, and server readiness polling
- Multi-host Shadow config builder (three_host_config) generating server + alice-host + bob-host topologies
- Three new subcommands (cs-api, e2ee-messaging, sas-verify) with --server-url and --role args
- Full workspace compilation verified with `cargo check` and `cargo build`

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ruma dependency and scenario module scaffold** - `3d3df82e` (feat)
2. **Task 2: Extend Shadow config and add subcommand stubs** - `4eb953f2` (feat)

## Files Created/Modified
- `tests/shadow/Cargo.toml` - Added ruma, tracing, tracing-subscriber dependencies
- `tests/shadow/src/lib.rs` - Added `pub mod scenarios;`
- `tests/shadow/src/scenarios/mod.rs` - Module declarations for common, cs_api, e2ee_msg, sas_verify
- `tests/shadow/src/scenarios/common.rs` - MatrixClient struct, wait_for_server, register_with_token, login_user, create_sdk_client
- `tests/shadow/src/scenarios/cs_api.rs` - Empty stub (populated by Plan 02)
- `tests/shadow/src/scenarios/e2ee_msg.rs` - Empty stub (populated by Plan 03)
- `tests/shadow/src/scenarios/sas_verify.rs` - Empty stub (populated by Plan 04)
- `tests/shadow/src/config/shadow.rs` - Added three_host_config() multi-host topology builder
- `tests/shadow/src/config/tuwunel.rs` - Added allow_encryption field to TuwunelGlobal
- `tests/shadow/src/bin/matrix_test_client.rs` - Added CsApi, E2eeMessaging, SasVerify subcommands

## Decisions Made
- **Used ruma + reqwest instead of matrix-sdk**: matrix-sdk 0.16 requires async-channel >= 2.5.0, but the workspace patches async-channel to a fork at 2.3.1 (adds LIFO queue scheduling). This is an irreconcilable version conflict. The ruma + reqwest approach provides equivalent functionality for CS API operations (registration, login, room management, messaging) since ruma is already in the workspace via the tuwunel fork. E2EE scenarios (Plans 03-04) will need to evaluate whether to update the fork or use alternative E2EE libraries.
- **MatrixClient wrapper struct**: Provides a clean API surface (register_with_token, login_user, access_token) that matches the plan's intended matrix-sdk::Client usage pattern, making future migration straightforward if the async-channel conflict is resolved.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Replaced matrix-sdk with ruma + reqwest due to async-channel version conflict**
- **Found during:** Task 1 (Add matrix-sdk dependency)
- **Issue:** matrix-sdk 0.16 requires async-channel >= 2.5.0; workspace [patch.crates-io] pins async-channel to fork at 2.3.1. Cargo cannot resolve this conflict.
- **Fix:** Used ruma (workspace dep, already compatible) + reqwest (already a dep) to implement the same MatrixClient functionality. Created a MatrixClient struct wrapping reqwest with UIAA registration, login, and server readiness helpers.
- **Files modified:** tests/shadow/Cargo.toml, tests/shadow/src/scenarios/common.rs
- **Verification:** `cargo check -p shadow-test-harness` and `cargo build -p shadow-test-harness --bin matrix-test-client` both succeed
- **Committed in:** 3d3df82e (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Replaced matrix-sdk with equivalent ruma+reqwest implementation. All plan acceptance criteria met except the specific matrix-sdk dependency in Cargo.toml. Functional equivalence preserved.

## Known Stubs
- `tests/shadow/src/scenarios/cs_api.rs` - Empty, populated by Plan 02
- `tests/shadow/src/scenarios/e2ee_msg.rs` - Empty, populated by Plan 03
- `tests/shadow/src/scenarios/sas_verify.rs` - Empty, populated by Plan 04
- `run_cs_api()` in matrix_test_client.rs - Placeholder TODO, implemented by Plan 02
- `run_e2ee_messaging()` in matrix_test_client.rs - Placeholder TODO, implemented by Plan 03
- `run_sas_verify()` in matrix_test_client.rs - Placeholder TODO, implemented by Plan 04

All stubs are intentional scaffolding for subsequent plans in this phase and do not prevent the plan's goal (foundation scaffold) from being achieved.

## Issues Encountered
- async-channel version conflict between workspace patch (2.3.1 fork) and matrix-sdk requirement (>= 2.5.0). Resolved by using ruma + reqwest directly, as documented in Deviations.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Scenario module scaffold ready for Plan 02 (CS API implementation)
- MatrixClient helpers ready for use in all scenario implementations
- Multi-host Shadow config builder ready for integration tests
- Subcommand routing ready for scenario-specific async functions

## Self-Check: PASSED

All 10 created/modified files verified present. Both task commits (3d3df82e, 4eb953f2) verified in git log.

---
*Phase: 02-cs-api-and-e2ee-tests*
*Completed: 2026-03-26*
