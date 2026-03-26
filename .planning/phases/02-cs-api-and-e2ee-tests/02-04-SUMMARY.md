---
phase: 02-cs-api-and-e2ee-tests
plan: 04
subsystem: testing
tags: [sas, verification, to-device, e2ee, shadow, matrix]

requires:
  - phase: 02-cs-api-and-e2ee-tests-03
    provides: E2EE messaging scenario pattern with key upload, encrypted room, sync polling
provides:
  - SAS verification scenario exercising to-device message routing under Shadow
  - Integration test validating full SAS protocol message flow
affects: [phase-03, embed-crate]

tech-stack:
  added: []
  patterns: [to-device messaging via raw CS API, multi-step protocol verification via sync polling]

key-files:
  created:
    - tests/shadow/src/scenarios/sas_verify.rs
    - tests/shadow/tests/sas_verify.rs
  modified:
    - tests/shadow/src/bin/matrix_test_client.rs

key-decisions:
  - "Used raw CS API to-device messaging instead of matrix-sdk SAS verification (async-channel conflict)"
  - "Test validates server-side to-device message routing, not cryptographic correctness"
  - "Full 8-step protocol flow: request, ready, start, key, key, mac, mac, done"

patterns-established:
  - "To-device message routing test: send via PUT sendToDevice, receive via sync to_device section"
  - "Multi-step protocol verification: state machine driven by sync polling loop"

requirements-completed: [E2EE-05]

duration: 3min
completed: 2026-03-26
---

# Phase 02 Plan 04: SAS Verification Summary

**SAS verification protocol message routing test via raw CS API to-device endpoints under Shadow simulation**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-26T02:53:25Z
- **Completed:** 2026-03-26T02:56:36Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments
- SAS verification scenario with Alice and Bob exchanging all 8 protocol messages via to-device routing
- Both clients auto-accept and auto-confirm through sync-driven state machine
- Integration test wired with Shadow runner using 180s stop_time for protocol round trips

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement SAS verification scenario** - `2a55c83b` (feat)
2. **Task 2: Wire SAS verification integration test** - `14e21434` (feat)

## Files Created/Modified
- `tests/shadow/src/scenarios/sas_verify.rs` - SAS verification scenario with alice/bob roles, to-device messaging, sync-driven protocol state machine
- `tests/shadow/tests/sas_verify.rs` - Integration test for SAS verification under Shadow simulation
- `tests/shadow/src/bin/matrix_test_client.rs` - Wired sas-verify subcommand to scenario module

## Decisions Made
- Used raw CS API to-device messaging endpoints instead of matrix-sdk SAS verification types due to async-channel workspace conflict. This tests tuwunel's to-device message routing (the server's responsibility) rather than cryptographic correctness (matrix-sdk's responsibility).
- Implemented full 8-step protocol flow (request/ready/start/key/key/mac/mac/done) with fake key material. The server does not validate crypto content, so fake values are sufficient to test routing.
- Used 180s stop_time for Shadow simulation to allow generous time for 8+ round-trip to-device messages.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Adapted matrix-sdk SAS API to raw CS API**
- **Found during:** Task 1 (SAS verification scenario implementation)
- **Issue:** Plan specified matrix-sdk SasVerification/SasState types and add_event_handler pattern, but matrix-sdk is unavailable due to async-channel version conflict
- **Fix:** Implemented SAS verification protocol using raw CS API sendToDevice endpoint with sync polling to receive to-device events. Both sides drive the protocol forward by sending appropriate responses when receiving each verification step.
- **Files modified:** tests/shadow/src/scenarios/sas_verify.rs
- **Verification:** cargo check -p shadow-test-harness passes
- **Committed in:** 2a55c83b (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking - expected, documented in plan critical_deviation)
**Impact on plan:** Adapted approach tests the same server capability (to-device message routing for SAS verification) from the server's perspective. No scope creep.

## Issues Encountered
None beyond the expected matrix-sdk unavailability.

## Known Stubs
None - all code paths are fully implemented.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- All E2EE scenarios (messaging + SAS verification) are complete
- Phase 02 CS API and E2EE test coverage is ready for verification
- Shadow simulation infrastructure supports all planned test scenarios

---
*Phase: 02-cs-api-and-e2ee-tests*
*Completed: 2026-03-26*
