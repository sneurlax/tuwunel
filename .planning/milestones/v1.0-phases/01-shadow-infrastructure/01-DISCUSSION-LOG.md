# Phase 1: Shadow Infrastructure - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-25
**Phase:** 01-shadow-infrastructure
**Areas discussed:** Test client design, Readiness detection, Build profile strategy, Port 0 / server changes

---

## Test Client Design

| Option | Description | Selected |
|--------|-------------|----------|
| Rust binary with reqwest | Minimal Rust binary using reqwest for HTTP + serde_json. No ruma dependency. Lightweight, fast to compile. | |
| Rust binary with ruma | Rust binary using ruma client types for type-safe Matrix API calls. More boilerplate but compile-time protocol checks. | |
| Rust binary with matrix-sdk | Use matrix-sdk as test client. Full E2EE support built-in. Heaviest dependency but closest to real clients. | ✓ |

**User's choice:** Rust binary with matrix-sdk
**Notes:** Full E2EE support out of the box matters for Phase 2 E2EE scenarios.

### Follow-up: Binary structure

| Option | Description | Selected |
|--------|-------------|----------|
| Single binary + subcommands | One crate, one binary, clap subcommands. Single compile target. | ✓ |
| Separate binaries per scenario | One binary per test type. More isolation but more build targets. | |

**User's choice:** Single binary + subcommands

### Follow-up: Crate location

| Option | Description | Selected |
|--------|-------------|----------|
| tests/shadow/ | Matches STATE.md decision. Not a workspace member. | |
| src/shadow-client/ | Workspace member under src/. Gets workspace dep management. | |
| You decide | Claude picks based on codebase conventions. | ✓ |

**User's choice:** You decide

---

## Readiness Detection

| Option | Description | Selected |
|--------|-------------|----------|
| Poll /_matrix/client/versions | Retry loop with simulated-time backoff. Simple, tests actual HTTP path. | |
| Parse stdout for ready marker | Server prints known string. Fragile across upstream updates. | |
| Both (poll with stdout fallback) | Primary: poll versions. Fallback: check stdout for error diagnostics. | ✓ |

**User's choice:** Both (poll with stdout fallback)
**Notes:** Belt and suspenders — poll as primary, stdout as diagnostic fallback.

---

## Build Profile Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Cargo profile + feature set | Dedicated [profile.shadow] inheriting release. Paired with feature exclusion. | ✓ |
| Feature flag only | Use existing release profile. Just pass feature flags. | |
| Makefile/script wrapper | Shell script encoding correct cargo flags. | |

**User's choice:** Cargo profile + feature set

### Follow-up: Build-time io_uring assertion

| Option | Description | Selected |
|--------|-------------|----------|
| compile_error! if io_uring + shadow | cfg check emits compile_error! if both active. Catches misconfigured builds. | ✓ |
| CI check only | Verify in CI that shadow build doesn't pull io_uring deps. | |
| You decide | Claude picks most appropriate enforcement. | |

**User's choice:** compile_error! if io_uring + shadow

---

## Port 0 / Server Changes

| Option | Description | Selected |
|--------|-------------|----------|
| Log bound port + write to file | After bind(0), log port and write to file. Minimal server change. | |
| Config callback / shared state | Store bound port in Server state. More invasive but cleaner API. | |
| You decide | Claude picks balancing minimal diff with Phase 4 reusability. | ✓ |

**User's choice:** You decide

### Follow-up: Phase placement

| Option | Description | Selected |
|--------|-------------|----------|
| Keep in Phase 1 | Needed for Shadow smoke test. Better to have from start. | |
| Defer to Phase 4 | Phase 1 can hardcode port in Shadow configs. Port 0 part of embed work. | |
| You decide | Claude evaluates whether Shadow's virtual networking makes port 0 unnecessary in Phase 1. | ✓ |

**User's choice:** You decide
**Notes:** Shadow virtual IPs may eliminate need for port 0 in Phase 1. Researcher should evaluate.

---

## Claude's Discretion

- Test client crate location — pick based on workspace conventions
- Port 0 approach and phase placement — evaluate Shadow's network model first
- Port exposure mechanism — log + file write vs shared state

## Deferred Ideas

None — discussion stayed within phase scope.
