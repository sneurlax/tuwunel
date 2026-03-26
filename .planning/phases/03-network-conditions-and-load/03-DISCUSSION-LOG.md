# Phase 3: Network Conditions and Load - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-25
**Phase:** 03-network-conditions-and-load
**Areas discussed:** Topology fixtures, Load test design, Impairment values, Pass/fail thresholds

---

## Topology Fixtures

| Option | Description | Selected |
|--------|-------------|----------|
| Rust builder API | Extend ShadowConfig with TopologyFixture::slow_mobile() etc. Type-safe, composable. | ✓ |
| YAML template files | Ship .yaml files in tests/shadow/fixtures/. Easy to inspect but harder to parameterize. | |
| GML template files | Ship .gml files referenced by path. Most direct but adds file management. | |

**User's choice:** Rust builder API
**Notes:** Consistent with existing ShadowConfig pattern.

| Option | Description | Selected |
|--------|-------------|----------|
| Preset with overrides | Defaults + builder overrides (.with_latency(), .with_loss()) | ✓ |
| Purely preset | Fixed values per name, new experiments need new functions | |
| Fully parameterized | No presets, always specify all values | |

**User's choice:** Preset with overrides

---

## Load Test Design

| Option | Description | Selected |
|--------|-------------|----------|
| One host per client | 100 Shadow hosts + 1 server. Most realistic. | ✓ |
| Batched hosts | 10 hosts with 10 clients each. Reduces host count. | |
| Single host, many processes | One host, 100 processes. Least realistic. | |

**User's choice:** One host per client
**Notes:** "At least 1. One host per client, but as many as are useful."

| Option | Description | Selected |
|--------|-------------|----------|
| Register + login + send one message | Minimal flow per LOAD-02. Simplest. | ✓ |
| Full CS API flow per client | More thorough but 100 room creations may overwhelm. | |
| Mixed workload | Various operations. Realistic but complex. | |

**User's choice:** Register + login + send one message first, then more as useful.

| Option | Description | Selected |
|--------|-------------|----------|
| First client creates, rest join by alias | Same pattern as Phase 2 alice/bob. | ✓ |
| Server admin pre-creates room | Use admin API. Simpler but requires research. | |
| Each client creates own room | No coordination. Doesn't test concurrent participation. | |

**User's choice:** First client creates, rest join by alias.

---

## Impairment Values

| Option | Description | Selected |
|--------|-------------|----------|
| Realistic mobile profiles | slow-mobile: 150ms/1%/5Mbit, high-latency: 500ms/0%/100Mbit, lossy-link: 50ms/5%/10Mbit | ✓ |
| Conservative/mild | Lower values, less stressful, less realistic | |
| Claude decides | Let researcher pick based on docs and real-world data | |

**User's choice:** Realistic mobile profiles

---

## Pass/Fail Thresholds

| Option | Description | Selected |
|--------|-------------|----------|
| Binary pass/fail | All tests: exit 0 within stop_time or fail. No partial success. | ✓ |
| Allow partial failure | 95/100 clients is a pass. More realistic but complex. | |
| Timed thresholds | Assert response times. Informative but hard to implement. | |

**User's choice:** Binary pass/fail

---

## Claude's Discretion

- Shadow stop_time values for each scenario
- Additional fixtures beyond the three named ones
- GML graph builder internal implementation
- Whether load test reuses existing scenario logic

## Deferred Ideas

None — discussion stayed within phase scope
