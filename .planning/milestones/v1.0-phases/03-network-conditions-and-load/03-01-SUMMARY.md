---
phase: 03-network-conditions-and-load
plan: 01
subsystem: testing
tags: [shadow, gml, network-impairment, topology, e2ee]

# Dependency graph
requires:
  - phase: 02-cs-api-and-e2ee-tests
    provides: "E2EE messaging scenario, three_host_config, MatrixClient, Shadow test patterns"
provides:
  - "TopologyFixture builder with three named fixtures (slow_mobile, high_latency, lossy_link)"
  - "Inline GML graph support in NetworkGraph"
  - "three_host_config_with_topology() for impaired network tests"
  - "NET-05 integration test: E2EE under 200ms RTT + 2% loss"
affects: [03-02, load-testing, future-network-scenarios]

# Tech tracking
tech-stack:
  added: []
  patterns: [TopologyFixture builder pattern, inline GML graph generation]

key-files:
  created:
    - tests/shadow/tests/net_impairment.rs
  modified:
    - tests/shadow/src/config/shadow.rs

key-decisions:
  - "TopologyFixture uses owned builder pattern (consuming self) for chaining"
  - "three_host_config_with_topology delegates to three_host_config then overrides network field"

patterns-established:
  - "TopologyFixture builder: named constructor + with_* overrides + to_gml()/to_network()"
  - "Integration tests for impaired networks follow same pattern as e2ee.rs"

requirements-completed: [NET-01, NET-02, NET-03, NET-04, NET-05]

# Metrics
duration: 2min
completed: 2026-03-26
---

# Phase 03 Plan 01: Network Impairment Summary

**Inline GML topology support with three named fixtures and E2EE-under-impairment integration test using 200ms RTT + 2% packet loss**

## Performance

- **Duration:** 2 min
- **Started:** 2026-03-26T04:35:43Z
- **Completed:** 2026-03-26T04:37:15Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments
- Extended NetworkGraph with optional inline GML field for custom topologies
- Created TopologyFixture builder with slow_mobile, high_latency, lossy_link fixtures
- Added three_host_config_with_topology for impaired network Shadow configs
- Created net_impairment.rs integration test verifying E2EE under 100ms one-way latency + 2% loss

## Task Commits

Each task was committed atomically:

1. **Task 1: Add inline GML graph support and TopologyFixture builder** - `ae4af1f2` (feat)
2. **Task 2: Create net_impairment integration test** - `cac6e200` (feat)

## Files Created/Modified
- `tests/shadow/src/config/shadow.rs` - Extended NetworkGraph with inline GML, added TopologyFixture builder with 3 fixtures and overrides, added three_host_config_with_topology
- `tests/shadow/tests/net_impairment.rs` - Integration test for E2EE messaging under 200ms RTT + 2% packet loss

## Decisions Made
- TopologyFixture uses consuming self builder pattern (with_latency, with_loss, etc.) for ergonomic chaining
- three_host_config_with_topology delegates to existing three_host_config then overrides the network field, minimizing code duplication
- GML string uses format!() with explicit \x20 escapes for indentation (compatible with hard tabs code style)

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered
None

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
- TopologyFixture builder ready for use in Plan 02 load testing
- Inline GML support enables any future custom topology scenarios
- NET-01 through NET-05 requirements satisfied

---
*Phase: 03-network-conditions-and-load*
*Completed: 2026-03-26*
