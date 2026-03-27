# Phase 4: Embed Crate - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-27
**Phase:** 04-embed-crate
**Areas discussed:** OnceLock/global state, Config construction API, In-memory transport scope, Crate boundary

---

## OnceLock / Global State Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| Shared runtime, per-instance server | One runtime + logging, `get_or_init` for OnceLocks, per-instance Server/Services/RocksDB | ✓ |
| Refactor OnceLocks out of statics | Move values into RuntimeConfig struct, pass through call chain | |
| One runtime per instance | Thread-locals or per-instance state containers, full isolation | |

**User's choice:** Shared runtime, per-instance server
**Notes:** Minimal diff, fits testing use case. OnceLock values are process-wide tuning that should be set once.

---

## Config Construction API

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal params with defaults | `start("server-name")` with sensible defaults, optional builder for customization | ✓ |
| Builder pattern only | `builder().server_name("x").port(0).build().start()` | |
| Raw figment passthrough | `start(figment)` — maximum flexibility, leaky abstraction | |

**User's choice:** Minimal params with defaults, with an optional builder pattern
**Notes:** Simple one-liner for common case, builder available when consumers need to tune.

---

## In-Memory Transport Scope (EMBD-10)

| Option | Description | Selected |
|--------|-------------|----------|
| Defer to v2 | TCP-only in v1, port 0 eliminates conflicts | ✓ |
| Include in v1 | Extract Router, expose via `router()` method | |

**User's choice:** Defer to v2
**Notes:** Router extraction too tightly coupled for v1. TCP + port 0 is sufficient. ROADMAP.md already flagged this as potentially v2.

---

## Crate Boundary and Upstream Diff

| Option | Description | Selected |
|--------|-------------|----------|
| Thin wrapper over existing API | Depend on tuwunel-router, minimal upstream changes | ✓ |
| Fork key functions | Copy/adapt Server::new(), runtime::new() into embed crate | |
| Refactor shared core | Extract tuwunel-server-core crate for both binary and embed | |

**User's choice:** Thin wrapper over existing public API
**Notes:** Aligns with "minimal and rebaseable" constraint. Upstream changes limited to get_or_init and idempotent logging.

## Claude's Discretion

- Port 0 implementation approach
- Builder pattern ergonomics
- register_user() implementation strategy
- Runtime thread count defaults for embed

## Deferred Ideas

- EMBD-10: In-memory axum Router transport — deferred to v2
