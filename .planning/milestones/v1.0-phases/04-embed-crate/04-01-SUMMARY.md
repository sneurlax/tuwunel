---
phase: 04-embed-crate
plan: 01
subsystem: embed
tags: [rust, embed, figment, tokio, rocksdb, oncelock]

# Dependency graph
requires:
  - phase: 01-shadow-harness
    provides: Shadow test infrastructure and workspace conventions
provides:
  - Multi-call-safe OnceLock initialization in runtime.rs
  - tuwunel-embed crate scaffold with EmbeddedHomeserver struct
  - Builder pattern for programmatic config construction
  - Port 0 dynamic port support via pre-bind
  - Auto tempdir provisioning for RocksDB
affects: [04-02, embed-integration-tests]

# Tech tracking
tech-stack:
  added: [tempfile, figment-serialized-provider]
  patterns: [builder-pattern-config, port-0-prebind, oncelock-get-or-init]

key-files:
  created:
    - src/embed/Cargo.toml
    - src/embed/src/lib.rs
    - src/embed/src/config.rs
  modified:
    - src/main/runtime.rs
    - src/main/logging.rs

key-decisions:
  - "Used get_or_init instead of set().expect() for OnceLock statics -- first caller wins semantics"
  - "Made logging::init and TracingFlameGuard public to allow embed crate access"
  - "Used default-features = false on tuwunel dependency to avoid io_uring build requirement"
  - "Used workspace deps for tuwunel-core/router/service, path dep for tuwunel main crate"

patterns-established:
  - "Builder pattern: fluent API with sensible defaults, build_figment separates config from startup"
  - "Port 0 pre-bind: TcpListener::bind then extract port and drop listener before server start"
  - "Embed logging: log_global_default=false to avoid tracing global subscriber conflicts"

requirements-completed: [EMBD-01, EMBD-03, EMBD-05, EMBD-08]

# Metrics
duration: 7min
completed: 2026-03-27
---

# Phase 04 Plan 01: Embed Crate Foundation Summary

**OnceLock multi-instance safety fix and tuwunel-embed crate scaffold with figment-based config builder, port 0 pre-bind, and auto tempdir**

## Performance

- **Duration:** 7 min
- **Started:** 2026-03-27T17:09:02Z
- **Completed:** 2026-03-27T17:16:16Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- Fixed OnceLock panics in runtime.rs enabling multiple server instances per process
- Created tuwunel-embed crate with EmbeddedHomeserver struct and Builder config API
- Implemented port 0 dynamic assignment via TCP pre-bind pattern
- Auto tempdir provisioning for RocksDB when no explicit path given
- Verified crate compiles successfully with cargo check

## Task Commits

Each task was committed atomically:

1. **Task 1: Fix OnceLock statics in runtime.rs** - `e04cef1d` (feat)
2. **Task 2: Create tuwunel-embed crate scaffold** - `66721382` (feat)

## Files Created/Modified
- `src/main/runtime.rs` - Changed .set().expect() to .get_or_init() for 3 OnceLock statics
- `src/main/logging.rs` - Made init() and TracingFlameGuard public for embed crate access
- `src/embed/Cargo.toml` - Crate manifest with workspace dependencies
- `src/embed/src/lib.rs` - EmbeddedHomeserver struct with start/stop/base_url methods
- `src/embed/src/config.rs` - Builder pattern with figment config, port 0 pre-bind, auto tempdir

## Decisions Made
- Used get_or_init for OnceLock statics (first-caller-wins, backwards compatible with single-call main binary)
- Made logging::init and TracingFlameGuard public (was pub(crate)) to allow embed crate to initialize logging
- Used default-features = false on tuwunel main crate dependency to avoid io_uring build requirement (no liburing on build host)
- Used workspace deps for internal crates where available, path dep for main tuwunel crate (no workspace entry exists)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Made logging::init and TracingFlameGuard public**
- **Found during:** Task 2 (embed crate creation)
- **Issue:** logging::init was pub(crate) and TracingFlameGuard was pub(crate) type, inaccessible from embed crate
- **Fix:** Changed both to pub visibility
- **Files modified:** src/main/logging.rs
- **Verification:** cargo check -p tuwunel-embed succeeds
- **Committed in:** 66721382 (Task 2 commit)

**2. [Rule 3 - Blocking] Used default-features = false for tuwunel dependency**
- **Found during:** Task 2 (embed crate creation)
- **Issue:** io_uring in default features requires liburing system library not available on build host
- **Fix:** Added default-features = false to tuwunel dependency in Cargo.toml
- **Files modified:** src/embed/Cargo.toml
- **Verification:** cargo check -p tuwunel-embed succeeds
- **Committed in:** 66721382 (Task 2 commit)

**3. [Rule 3 - Blocking] Used workspace deps instead of path deps for internal crates**
- **Found during:** Task 2 (embed crate creation)
- **Issue:** Path deps like `path = "../core"` failed because Cargo package names use underscores (tuwunel_core) not hyphens
- **Fix:** Used workspace = true references which handle the package name mapping
- **Files modified:** src/embed/Cargo.toml
- **Verification:** cargo check -p tuwunel-embed succeeds
- **Committed in:** 66721382 (Task 2 commit)

---

**Total deviations:** 3 auto-fixed (3 blocking)
**Impact on plan:** All auto-fixes necessary for the crate to compile. No scope creep.

## Issues Encountered
None beyond the auto-fixed deviations above.

## Known Stubs
- `src/embed/src/lib.rs` line ~63: `register_user` method returns `todo!()` -- intentionally stubbed, will be implemented in plan 02

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- Embed crate foundation complete, ready for plan 02 (lifecycle and integration tests)
- Builder produces valid figment config that constructs a working tuwunel_core::Config
- EmbeddedHomeserver::stop() implemented with graceful shutdown sequence
- register_user() stub needs implementation in plan 02
