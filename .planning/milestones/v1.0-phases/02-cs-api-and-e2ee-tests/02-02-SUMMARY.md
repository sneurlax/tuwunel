---
phase: 02-cs-api-and-e2ee-tests
plan: 02
subsystem: testing
tags: [matrix, cs-api, shadow, reqwest, ruma, integration-test]

requires:
  - phase: 02-cs-api-and-e2ee-tests/01
    provides: MatrixClient with register/login, three_host_config, ShadowRunner, scenarios module scaffold

provides:
  - CS API scenario (register, login, create room, send message, join, sync, verify)
  - MatrixClient helpers for room creation, messaging, joining, and sync
  - Shadow integration test for two-client CS API flow
  - Shared build_shadow_binaries helper for test reuse

affects: [02-cs-api-and-e2ee-tests/03, 02-cs-api-and-e2ee-tests/04]

tech-stack:
  added: []
  patterns: [two-client Shadow scenario with staggered start times, raw CS API via reqwest]

key-files:
  created:
    - tests/shadow/src/scenarios/cs_api.rs
    - tests/shadow/tests/cs_api.rs
    - tests/shadow/tests/common/mod.rs
  modified:
    - tests/shadow/src/scenarios/common.rs
    - tests/shadow/src/bin/matrix_test_client.rs

key-decisions:
  - "Used raw reqwest HTTP calls instead of matrix-sdk due to async-channel version conflict"
  - "Added room/message/sync helpers directly to MatrixClient rather than separate functions"
  - "Used atomic counter for transaction IDs instead of random UUIDs"
  - "Manual URL encoding for room aliases (# and :) to avoid adding urlencoding dependency"

patterns-established:
  - "Two-client scenario pattern: alice creates resources, bob verifies via staggered Shadow start times"
  - "Shared test helper module at tests/common/mod.rs for build_shadow_binaries"
  - "Sync response parsing via raw serde_json::Value traversal for timeline events"

requirements-completed: [TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06]

duration: 3min
completed: 2026-03-26
---

# Phase 02 Plan 02: CS API Scenario Summary

**Two-client Matrix CS API test scenario with alice (register, login, create room, send message) and bob (register, login, join by alias with retry, sync, verify message receipt) via Shadow simulation**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-26T02:42:20Z
- **Completed:** 2026-03-26T02:45:42Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Implemented complete CS API scenario covering registration, login, room creation, messaging, joining, and sync
- Added create_room, send_text_message, join_room, join_room_with_retry, and sync methods to MatrixClient
- Created Shadow integration test with three-host topology (server, alice, bob) and staggered start times
- Extracted shared build_shadow_binaries helper for test reuse across integration tests

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement CS API scenario (alice and bob roles)** - `00449fa6` (feat)
2. **Task 2: Wire CS API integration test with Shadow runner** - `e2e82db1` (feat)

## Files Created/Modified
- `tests/shadow/src/scenarios/cs_api.rs` - CS API scenario with alice and bob flows
- `tests/shadow/src/scenarios/common.rs` - Added room/message/sync helpers to MatrixClient
- `tests/shadow/src/bin/matrix_test_client.rs` - Wired cs-api subcommand to scenario
- `tests/shadow/tests/cs_api.rs` - Shadow integration test for CS API two-client flow
- `tests/shadow/tests/common/mod.rs` - Shared build_shadow_binaries helper

## Decisions Made
- Used raw reqwest HTTP calls to Matrix CS API instead of matrix-sdk (async-channel 2.5.0 vs 2.3.1 conflict prevents matrix-sdk compilation in workspace)
- Added helper methods directly to MatrixClient struct rather than standalone functions for ergonomic chaining
- Used atomic u64 counter for transaction IDs (deterministic, no external RNG dependency)
- Manual percent-encoding for room alias URL characters (# -> %23, : -> %3A) to avoid adding urlencoding crate

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adapted matrix-sdk API calls to reqwest/ruma**
- **Found during:** Task 1 (CS API scenario implementation)
- **Issue:** Plan references matrix_sdk::Client, ruma types like RoomOrAliasId, SyncSettings, RoomMessageEventContent -- none available due to matrix-sdk compile failure
- **Fix:** Implemented all CS API operations as raw HTTP calls via reqwest on MatrixClient. Added create_room, send_text_message, join_room, join_room_with_retry, sync methods.
- **Files modified:** tests/shadow/src/scenarios/common.rs, tests/shadow/src/scenarios/cs_api.rs
- **Verification:** cargo check -p shadow-test-harness compiles clean
- **Committed in:** 00449fa6 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Adaptation was pre-planned in critical_deviation context. Same functional coverage achieved via raw HTTP. No scope creep.

## Issues Encountered
None beyond the pre-documented matrix-sdk incompatibility.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- CS API scenario complete, provides foundation for E2EE messaging (Plan 03) and SAS verification (Plan 04)
- MatrixClient now has full CRUD operations needed for all remaining scenarios
- Shared test helper module ready for reuse by e2ee and sas_verify integration tests

---
*Phase: 02-cs-api-and-e2ee-tests*
*Completed: 2026-03-26*
