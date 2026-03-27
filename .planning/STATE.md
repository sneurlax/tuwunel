---
gsd_state_version: 1.0
milestone: v1.0
milestone_name: milestone
status: Ready to execute
stopped_at: Completed 03-02-PLAN.md
last_updated: "2026-03-27T05:05:27.014Z"
progress:
  total_phases: 4
  completed_phases: 3
  total_plans: 9
  completed_plans: 9
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-03-25)

**Core value:** Deterministic, reproducible E2E tests that verify tuwunel's Matrix protocol behavior under realistic network conditions
**Current focus:** Phase 03 — network-conditions-and-load

## Current Position

Phase: 03 (network-conditions-and-load) — EXECUTING
Plan: 2 of 2

## Performance Metrics

**Velocity:**

- Total plans completed: 0
- Average duration: —
- Total execution time: —

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| - | - | - | - |

**Recent Trend:**

- Last 5 plans: —
- Trend: —

*Updated after each plan completion*
| Phase 01 P01 | 4min | 2 tasks | 9 files |
| Phase 01 P02 | 3min | 2 tasks | 4 files |
| Phase 01 P03 | 1min | 1 tasks | 1 files |
| Phase 02 P01 | 8min | 2 tasks | 10 files |
| Phase 02 P02 | 3min | 2 tasks | 5 files |
| Phase 02 P04 | 3min | 2 tasks | 3 files |
| Phase 03 P02 | 3min | 2 tasks | 5 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- [Init]: Phase B (Shadow tests) before Phase A (embed crate) — baseline before code changes
- [Init]: io_uring must be disabled for Shadow builds — hard blocker, build-time assertion required
- [Init]: Shadow harness lives in this repo under tests/shadow/; embed crate under src/embed/
- [Init]: EMBD-10 (in-memory transport) may slip to v2 — flag at plan time for Phase 4
- [Phase 01]: Used marker feature shadow=[] instead of cfg flag for compile_error guard (Cargo stable lacks per-profile rustflags)
- [Phase 01]: shadow_features includes all defaults except io_uring and systemd (systemd unnecessary under Shadow)
- [Phase 01]: Used reqwest directly for Phase 1 smoke test instead of matrix-sdk (only needs GET to /_matrix/client/versions)
- [Phase 01]: Test builds shadow-profile binaries itself via cargo subprocess rather than requiring pre-built binaries
- [Phase 02]: Used ruma + reqwest instead of matrix-sdk due to async-channel 2.3.1 fork conflict with matrix-sdk requirement for >= 2.5.0
- [Phase 02]: MatrixClient wrapper struct provides equivalent API to matrix-sdk::Client for CS API operations
- [Phase 02]: Used raw reqwest HTTP calls instead of matrix-sdk for CS API operations due to async-channel version conflict
- [Phase 02]: Manual URL percent-encoding for room aliases to avoid adding urlencoding dependency
- [Phase 02]: Used raw CS API to-device messaging for SAS verification instead of matrix-sdk (async-channel conflict)
- [Phase 03]: Creator starts at 5s, joiners at 10s for room creation ordering

### Pending Todos

None yet.

### Blockers/Concerns

- [Phase 1]: io_uring disable must be verified empirically in smoke test — exact Shadow failure mode unknown
- [Phase 1]: tokio queue interval tuning for Shadow (lower intervals needed) — starting point values need smoke test validation
- [Phase 4]: OnceLock refactoring in src/main/runtime.rs — spike this first in Phase 4; determines embed API shape

## Session Continuity

Last session: 2026-03-27T05:05:27.012Z
Stopped at: Completed 03-02-PLAN.md
Resume file: None
