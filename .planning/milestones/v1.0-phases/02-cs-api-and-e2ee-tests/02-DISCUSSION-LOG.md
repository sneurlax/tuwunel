# Phase 2: CS API and E2EE Tests - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-25
**Phase:** 02-cs-api-and-e2ee-tests
**Areas discussed:** Client library approach, Scenario structure, Two-client topology, E2EE determinism

---

## Client Library Approach

| Option | Description | Selected |
|--------|-------------|----------|
| Full matrix-sdk | Use matrix-sdk for all scenarios including E2EE. Built-in crypto, sync, room management. | ✓ |
| matrix-sdk for E2EE only | Keep raw reqwest for basic CS API, only bring in matrix-sdk for crypto scenarios. | |
| Raw ruma + reqwest | Use ruma types with reqwest directly, hand-rolling E2EE. | |

**User's choice:** Full matrix-sdk
**Notes:** Matches Phase 1 D-01 decision.

| Option | Description | Selected |
|--------|-------------|----------|
| Keep smoke as reqwest | Smoke is lightweight readiness check, no need for full SDK. | ✓ |
| Migrate smoke to matrix-sdk | Uniform codebase but adds SDK for a simple HTTP GET. | |

**User's choice:** Keep smoke as reqwest

| Option | Description | Selected |
|--------|-------------|----------|
| Git dep matching fork | Point matrix-sdk at compatible git revision with ruma fork. | |
| Crates.io release + patches | Use latest crates.io matrix-sdk with [patch] sections. | |
| You decide | Claude figures out best approach during research. | ✓ |

**User's choice:** You decide

---

## Scenario Structure

| Option | Description | Selected |
|--------|-------------|----------|
| One subcommand per flow | e.g., `cs-api` runs full register→login→room→message→sync flow. | ✓ |
| One subcommand per operation | e.g., `register`, `login`, `send-message`. Shadow YAML chains them. | |
| Scenario config file | Single `run-scenario` subcommand reading JSON/YAML descriptions. | |

**User's choice:** One subcommand per flow

| Option | Description | Selected |
|--------|-------------|----------|
| Exit code + stderr log | Exit 0/1, progress to stderr. Integration test reads per-host stderr files. | ✓ |
| Structured JSON output | Write JSON report for machine-readable pass/fail per step. | |
| You decide | Claude picks based on integration with existing runner. | |

**User's choice:** Exit code + stderr log

| Option | Description | Selected |
|--------|-------------|----------|
| Self-contained | Each subcommand registers its own users, creates own rooms. No shared state. | ✓ |
| Shared setup phase | `setup` subcommand runs first, other subcommands receive credentials via args. | |

**User's choice:** Self-contained

---

## Two-Client Topology

| Option | Description | Selected |
|--------|-------------|----------|
| Separate hosts | client-alice and client-bob on different Shadow hosts with own virtual IPs. | ✓ |
| Same host, different processes | Both clients on one Shadow host. Simpler but doesn't exercise network simulation. | |

**User's choice:** Separate hosts

| Option | Description | Selected |
|--------|-------------|----------|
| Deterministic naming | Predictable names: @alice:tuwunel-server, @bob:tuwunel-server, #test-room. | ✓ |
| CLI args from Shadow config | Pass user IDs and room IDs as CLI args in Shadow YAML. | |
| File-based handoff | Alice writes credentials to file, bob reads it. | |

**User's choice:** Deterministic naming

| Option | Description | Selected |
|--------|-------------|----------|
| Shadow start_time offset | Alice starts at 5s, bob at 15s. Deterministic ordering. | |
| Bob polls for room existence | Bob starts early, polls for room in retry loop. | |
| You decide | Claude picks based on operation durations under Shadow. | ✓ |

**User's choice:** You decide

---

## E2EE Determinism

| Option | Description | Selected |
|--------|-------------|----------|
| In-memory store | MemoryStore for crypto state. No filesystem persistence, starts fresh each run. | |
| SQLite store in tempdir | Default SQLite crypto store. More realistic but adds I/O under Shadow. | |
| You decide | Claude picks lightest option that works. | |

**User's choice:** Both — in-memory as default for speed, SQLite as option for robustness.

| Option | Description | Selected |
|--------|-------------|----------|
| Let SDK handle via sync | Normal sync loop, SDK auto-handles key upload and claims. | |
| Explicit step-by-step | Manually call each E2EE operation in sequence. | |
| You decide | Claude evaluates based on matrix-sdk API surface. | ✓ |

**User's choice:** You decide

| Option | Description | Selected |
|--------|-------------|----------|
| Automate SAS | Both clients auto-accept emoji match. Proves protocol works. | ✓ |
| Defer SAS to later | Focus on simpler E2EE scenarios first. | |
| You decide | Claude assesses complexity and decides. | |

**User's choice:** Automate SAS

---

## Claude's Discretion

- matrix-sdk version alignment with ruma fork (D-03)
- Timing between alice and bob (D-09)
- E2EE key exchange orchestration approach (D-11)
- Shadow stop_time values per scenario

## Deferred Ideas

None — discussion stayed within phase scope
