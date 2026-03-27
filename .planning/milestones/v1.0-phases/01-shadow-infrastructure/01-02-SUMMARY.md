---
phase: 01-shadow-infrastructure
plan: 02
subsystem: testing
tags: [shadow, clap, reqwest, tokio, test-client, runner, retry-polling]

# Dependency graph
requires:
  - phase: 01-01
    provides: "tests/shadow/ workspace crate with ShadowConfig YAML and TuwunelConfig TOML generation"
provides:
  - "matrix-test-client binary with smoke subcommand polling /_matrix/client/versions"
  - "Shadow runner module (run_shadow) invoking shadow binary with structured result capture"
  - "ShadowResult with per-host stdout/stderr/pcap accessors and failure diagnostics"
affects: [01-03-PLAN]

# Tech tracking
tech-stack:
  added: [clap (CLI subcommands), reqwest with rustls (HTTP client), tokio (async runtime for test client)]
  patterns: [retry polling with tokio::time::sleep for Shadow simulated time, ShadowResult structured output with host file accessors]

key-files:
  created:
    - tests/shadow/src/runner.rs
  modified:
    - tests/shadow/Cargo.toml
    - tests/shadow/src/bin/matrix_test_client.rs
    - tests/shadow/src/lib.rs

key-decisions:
  - "Used reqwest directly instead of matrix-sdk for Phase 1 smoke test (only needs GET to /_matrix/client/versions)"
  - "Used current_thread tokio runtime in test client (lighter weight under Shadow)"
  - "Used danger_accept_invalid_certs since Shadow network lacks real TLS certs"
  - "Used rustls feature (not rustls-tls) matching reqwest 0.13 feature names"

patterns-established:
  - "Test client uses ExitCode::SUCCESS/FAILURE for Shadow expected_final_state checking"
  - "ShadowResult.find_host_stdouts/stderrs use directory listing for PID-agnostic file discovery"
  - "Automatic failure diagnostics on non-zero exit (seed + data_dir printed to stderr)"

requirements-completed: [SHAD-02, SHAD-05, SHAD-06, SHAD-09]

# Metrics
duration: 3min
completed: 2026-03-26
---

# Phase 01 Plan 02: Test Client Binary and Shadow Runner Summary

**matrix-test-client binary with retry-polling smoke subcommand and Shadow runner module with structured result capture and failure diagnostics**

## Performance

- **Duration:** 3 min
- **Started:** 2026-03-26T01:31:13Z
- **Completed:** 2026-03-26T01:33:47Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments
- matrix-test-client binary with clap-based smoke subcommand that polls /_matrix/client/versions with configurable retry loop
- Shadow runner module that invokes shadow binary, captures exit code/stdout/stderr, and provides per-host file accessors
- Automatic failure diagnostics printing seed and log directory paths on non-zero exit

## Task Commits

Each task was committed atomically:

1. **Task 1: Add runtime dependencies and build matrix-test-client binary** - `30e5a558` (feat)
2. **Task 2: Create Shadow runner module with output capture and failure diagnostics** - `0e16f87c` (feat)

## Files Created/Modified
- `tests/shadow/Cargo.toml` - Added clap, reqwest (rustls), tokio dependencies
- `tests/shadow/src/bin/matrix_test_client.rs` - Full smoke subcommand implementation with retry polling
- `tests/shadow/src/lib.rs` - Added runner module export
- `tests/shadow/src/runner.rs` - Shadow process invocation, ShadowResult struct, host file accessors, failure diagnostics

## Decisions Made
- Used reqwest directly instead of matrix-sdk for Phase 1 smoke test -- only needs a single GET to /_matrix/client/versions; matrix-sdk will be added in Phase 2
- Used current_thread tokio runtime in test client for lighter weight under Shadow simulation
- Used danger_accept_invalid_certs(true) since Shadow network does not have real TLS certificates
- Fixed reqwest feature name from `rustls-tls` (plan) to `rustls` (actual 0.13 API)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed reqwest feature name from rustls-tls to rustls**
- **Found during:** Task 1 (cargo check)
- **Issue:** Plan specified `features = ["rustls-tls"]` but reqwest 0.13 uses `rustls` as the feature name
- **Fix:** Changed to `features = ["rustls"]` in Cargo.toml
- **Files modified:** tests/shadow/Cargo.toml
- **Verification:** cargo check --package shadow-test-harness succeeds
- **Committed in:** 30e5a558 (Task 1 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking)
**Impact on plan:** Necessary fix for compilation. No scope creep.

## Issues Encountered
None beyond the auto-fixed reqwest feature name.

## Known Stubs
None -- both the test client binary and runner module are fully implemented.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Test client binary and Shadow runner ready for Plan 03 (integration test wiring)
- Plan 03 will create the actual Shadow smoke test combining config generation, test client, and runner

## Self-Check: PASSED

All created/modified files verified present. Both task commits verified in git log.

---
*Phase: 01-shadow-infrastructure*
*Completed: 2026-03-26*
