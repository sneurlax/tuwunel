# Tuwunel E2E Testing & Embedding

## What This Is

A friendly fork of tuwunel (Rust Matrix homeserver) that adds deterministic E2E testing via Shadow network simulation and a library embedding API. Shadow runs real tuwunel binaries under simulated network conditions with deterministic time control, replacing flaky Docker-based testing. The embed crate exposes tuwunel as an in-process library for downstream consumers like matrix-rust-client.

## Core Value

Deterministic, reproducible E2E tests that verify tuwunel's Matrix protocol behavior under realistic network conditions — before and after any code changes.

## Requirements

### Validated

- ✓ Matrix homeserver functionality — existing tuwunel codebase
- ✓ Client-Server API (auth, sync, rooms, messaging, E2EE) — existing
- ✓ Server-Server federation API — existing
- ✓ RocksDB embedded storage — existing
- ✓ figment-based configuration (TOML + env + programmatic) — existing
- ✓ Library API in src/main/lib.rs (exec, Server, Runtime, Args) — existing
- ✓ Graceful shutdown via broadcast channel — existing

### Active

- [ ] Shadow test infrastructure running stock tuwunel binary
- [ ] Basic E2E tests under Shadow (auth, sync, rooms, messaging)
- [ ] E2EE tests under Shadow (key exchange, verification, encrypted messaging)
- [ ] tuwunel-embed crate with EmbeddedHomeserver API
- [ ] Programmatic config construction (no file dependencies)
- [ ] Port 0 / dynamic port support
- [ ] Advanced Shadow scenarios (latency, packet loss, bandwidth limits)
- [ ] Load testing under Shadow (multi-client concurrency)
- [ ] In-memory transport via extracted axum Router

### Out of Scope

- Modifying Matrix protocol behavior — the server must behave identically to upstream
- Docker-based test infrastructure — that lives in matrix-rust-client
- Flutter/FFI integration — that's matrix-rust-client's concern
- Production deployment tooling — this fork is for testing
- Federation between multiple tuwunel instances — stretch goal for later
- Custom storage backends (sled, redb) — RocksDB is sufficient

## Context

**Tuwunel** is an 8-crate Cargo workspace (tuwunel-core, tuwunel-macros, tuwunel-database, tuwunel-service, tuwunel-api, tuwunel-admin, tuwunel-router, src/main). It already has a library/binary split — `src/main/lib.rs` exports a public API and `main.rs` is a thin wrapper. Uses axum + tokio + hyper for HTTP, RocksDB for storage, figment for config, ruma for Matrix protocol types.

**Shadow** (v3.3.0, installed at `~/.local/bin/shadow`) is a discrete-event network simulator that runs real Linux binaries via syscall interception. It simulates network topology with configurable latency, bandwidth, and packet loss. Time is deterministic — no wall-clock dependencies. Shadow source is at `~/src/monero/shadow`.

**matrix-rust-client** (at `../`) is the downstream consumer. It wraps matrix-sdk 0.9 and is currently tested via Docker testcontainers against Synapse and tuwunel. Those tests suffer from timing flakes (6 sync rounds with 200ms delays for E2EE), slow startup (3-10s for Synapse), and no network simulation capability.

**Testing strategy**: Phase B (Shadow tests on stock tuwunel) comes before Phase A (embed crate) to establish a baseline — we want to prove tuwunel works correctly under Shadow before making any code changes.

## Constraints

- **Upstream compatibility**: Fork changes must be minimal and rebaseable on upstream tuwunel releases
- **Shadow syscall coverage**: tuwunel uses standard networking (axum/hyper/tokio) which Shadow supports, but io_uring (a tuwunel feature flag) may need to be disabled for Shadow builds
- **RocksDB threading**: Each embedded server instance needs its own tempdir; multiple RocksDB instances in one process are supported but need separate paths
- **Shadow build**: Requires CMake + Cargo hybrid build (`./setup build && ./setup install`), installed to `~/.local/bin/shadow`
- **No path whitespace**: Shadow's LD_PRELOAD shim requires paths without whitespace

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Test stock tuwunel under Shadow before adding embed crate | Establishes correctness baseline; isolates effects of our changes | — Pending |
| Shadow harness lives in this repo (not matrix-rust-client) | We're testing tuwunel itself; matrix-rust-client gets its own harness later | — Pending |
| Add new tuwunel-embed crate rather than modifying src/main | Minimal diff from upstream; new workspace member is cleanly separable | — Pending |
| Disable io_uring feature for Shadow builds | Shadow may not support io_uring syscalls; standard epoll works fine | — Pending |
| In-memory transport as later phase | Requires deeper router extraction; TCP-based testing validates more of the real stack | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd:transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd:complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-03-25 after initialization*
