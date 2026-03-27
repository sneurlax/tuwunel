# Project Retrospective

*A living document updated after each milestone. Lessons feed forward into future planning.*

## Milestone: v1.0 — E2E Testing & Embedding MVP

**Shipped:** 2026-03-27
**Phases:** 4 | **Plans:** 11 | **Tasks:** 22

### What Was Built
- Shadow test harness (`tests/shadow/`) with programmatic config generation, runner, and structured result capture
- Four test scenarios: smoke, CS API (register/login/rooms/messaging), E2EE (key exchange, SAS verification), load (100 concurrent clients)
- Network topology fixtures with configurable latency, packet loss, and bandwidth
- `tuwunel-embed` crate with `EmbeddedHomeserver` API for in-process testing (start, stop, register_user)

### What Worked
- Testing stock tuwunel before modifying code (Phases 1-3) established confidence that Phase 4 changes didn't break anything
- Using ruma+reqwest directly instead of matrix-sdk avoided a blocking async-channel dependency conflict
- Coarse granularity with yolo mode kept execution fast — 11 plans in ~34 minutes total
- Shadow's deterministic time eliminated all timing-related test flakiness

### What Was Inefficient
- REQUIREMENTS.md traceability table fell out of sync with actual completion — 9 requirements marked "Pending" in traceability despite being implemented
- ROADMAP.md progress table wasn't updated as phases completed (Phase 1, 2, 4 still showed "Planned")
- E2EE tests use fake keys/marker patterns rather than real crypto — functional but not a true E2EE validation

### Patterns Established
- `compile_error!` guard with marker feature for build-time feature exclusion (io_uring under Shadow)
- Shadow config builder pattern: typed Rust structs -> serde_yaml -> tempfile -> Shadow invocation
- Multi-host test topology: creator starts at t=5s, joiners at t=10s for deterministic ordering
- OnceLock `get_or_init` pattern for multi-instance safety in embedded server scenarios

### Key Lessons
1. Keep traceability tables up-to-date during plan execution, not just at milestone boundaries — stale status creates false alarm during archival
2. ruma provides sufficient coverage for CS API testing without matrix-sdk; the lighter dependency is worth maintaining
3. Shadow's GML graph format enables inline topology definitions — no need for external YAML topology files

### Cost Observations
- Model mix: primarily opus with haiku for parallel research
- Sessions: ~5 sessions across 3 days
- Notable: coarse granularity + yolo mode eliminated confirmation overhead, keeping plan execution under 5 minutes each

---

## Cross-Milestone Trends

### Process Evolution

| Milestone | Sessions | Phases | Key Change |
|-----------|----------|--------|------------|
| v1.0 | ~5 | 4 | Initial milestone — established Shadow testing patterns |

### Cumulative Quality

| Milestone | Plans | Code Added | Upstream Changes |
|-----------|-------|-----------|-----------------|
| v1.0 | 11 | +5,245 lines | 13 lines modified |

### Top Lessons (Verified Across Milestones)

1. Test the unmodified system first, then add features — establishes trust in the baseline
2. Avoid heavy SDK dependencies when lighter alternatives (ruma+reqwest) provide equivalent coverage
