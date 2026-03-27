# Phase 4: Embed Crate - Context

**Gathered:** 2026-03-27
**Status:** Ready for planning

<domain>
## Phase Boundary

Build `tuwunel-embed` as a new workspace member crate that wraps tuwunel's existing server lifecycle into an `EmbeddedHomeserver` API for in-process testing. Consumers call `start()` to get a running server with a `base_url()`, can run multiple instances concurrently, and `stop()` for graceful shutdown. Includes programmatic config construction, OnceLock safety for multi-instance, port 0 support, and a `register_user()` convenience method. TCP transport only (in-memory Router extraction deferred to v2).

</domain>

<decisions>
## Implementation Decisions

### OnceLock / Global State Strategy
- **D-01:** Shared runtime, per-instance server. One tokio runtime and one logging init shared across all `EmbeddedHomeserver` instances. OnceLock statics in `runtime.rs` (`WORKER_AFFINITY`, `GC_ON_PARK`, `GC_MUZZY`) changed from `.set().expect()` to `get_or_init()` — first caller wins, subsequent calls are no-ops. Each instance gets its own `Server`, `Services`, and RocksDB tempdir.
- **D-02:** Logging init (`tracing::subscriber::set_global_default`) called once, guarded against double-registration panics. Subsequent instances skip logging init. Tracing is inherently global — per-instance logging is not needed.

### Config Construction API
- **D-03:** Primary API is minimal params with sensible defaults: `EmbeddedHomeserver::start("server-name")` constructs config internally with auto-provisioned tempdir, port 0, and registration disabled. An optional builder pattern (`EmbeddedHomeserver::builder().server_name("x").port(0).registration_token("y").build().start()`) is available for customization.
- **D-04:** Config constructed via figment internally — consumers never touch figment directly. Builder fields map to tuwunel config keys. Defaults optimized for testing: minimal logging, no TLS, no federation, ephemeral database.

### In-Memory Transport (EMBD-10)
- **D-05:** Deferred to v2. Phase 4 ships with TCP-only transport. Consumers connect via reqwest/hyper to `base_url()`. Port 0 (OS-assigned) eliminates port conflicts. The axum Router extraction in `src/router/router.rs::build()` is too tightly coupled for v1 scope.

### Crate Boundary and Upstream Diff
- **D-06:** Thin wrapper over existing public API. Embed crate depends on `tuwunel-router` directly (for `start`/`run`/`stop`) and `tuwunel-core` (for `Server`, `Config`). No forking of `src/main/` functions. Changes to upstream code limited to: `get_or_init` for OnceLock statics, idempotent logging init guard.
- **D-07:** Embed crate provides its own config construction and runtime management but delegates server lifecycle to `tuwunel-router::start()` / `run()` / `stop()`. Skips CLI arg parsing, signal handling, hot-reload, and systemd notification.

### Port 0 Support (CONF-01, deferred from Phase 1)
- **D-08:** Port 0 support at Claude's discretion for implementation approach. Must satisfy EMBD-03: `base_url()` returns the actual bound port. Researcher should evaluate `src/router/serve.rs` for how to extract the bound address after `TcpListener::bind("0.0.0.0:0")`.

### Claude's Discretion
- Port 0 implementation approach (D-08) — how to propagate bound port from listener to embed API
- Builder pattern ergonomics — which config fields to expose, naming conventions
- Whether `register_user()` uses the existing MatrixClient from test harness or standalone reqwest calls
- Runtime thread count defaults for embed use case (likely lower than production defaults)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Tuwunel Server Lifecycle (core dependency)
- `src/main/lib.rs` — Public API: `exec()`, `run()`, `async_exec()`, `async_start()`, `async_run()`, `async_stop()`
- `src/main/runtime.rs` — OnceLock statics (lines 34-36), `new()` function that must become multi-call-safe
- `src/main/server.rs` — `Server::new()`: config loading, logging init, resource limits
- `src/main/logging.rs` — `init()` function, `set_global_default()` call (line 134/143)
- `src/main/args.rs` — `Args` struct used by `runtime::new()` and `Server::new()`

### Router (server start/run/stop)
- `src/router/mod.rs` — `start()`, `run()`, `stop()` extern fns wrapping `run.rs`
- `src/router/run.rs` — `start()` builds Services, `run()` starts listener + admin, `stop()` shuts down
- `src/router/serve.rs` — TCP/Unix listener binding, `get_bind_addrs()` — key for port 0
- `src/router/router.rs` — `build()` returns axum Router (EMBD-10 v2 extraction point)

### Config System
- `src/core/config/mod.rs` — `Config::load()`, `Config::new()`, figment-based construction
- `tuwunel-example.toml` — Default config reference; embed defaults derived from this

### Phase 1-3 Artifacts
- `tests/shadow/` — Existing test harness crate; embed crate is a parallel workspace member
- `.planning/phases/01-shadow-infrastructure/01-CONTEXT.md` — D-07: port 0 deferral rationale
- `.planning/phases/01-shadow-infrastructure/01-RESEARCH.md` — OnceLock analysis, runtime constraints

### Workspace Structure
- `Cargo.toml` (workspace root) — workspace members list, shared deps, profiles

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tuwunel-router::start/run/stop` — Complete server lifecycle, ready to call from embed crate
- `tuwunel_core::Server` — Server state struct, takes Config + runtime handle + logger
- `tuwunel_core::config::Config` — Full config with figment, supports programmatic construction
- `tests/shadow/src/scenarios/common.rs::MatrixClient` — HTTP client with `register_with_token()`, could be reused for `register_user()`

### Established Patterns
- Workspace member crates follow `src/{crate_name}/` directory convention
- All crates use hard tabs, max 98 char lines, edition 2024
- Cargo.toml shared deps via `[workspace.dependencies]`
- Server shutdown via broadcast channel on `tuwunel_core::Server`

### Integration Points
- New crate added to `[workspace.members]` in root `Cargo.toml`
- Depends on `tuwunel-core` and `tuwunel-router` (and transitively `tuwunel-service`, `tuwunel-database`)
- `src/main/runtime.rs` OnceLock changes affect the main binary (must remain compatible)
- `src/main/logging.rs` idempotency guard affects the main binary (must remain compatible)

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches within the decisions above.

</specifics>

<deferred>
## Deferred Ideas

- **EMBD-10: In-memory axum Router transport** — Deferred to v2 per D-05. Router extraction from `src/router/router.rs::build()` requires decoupling middleware stack from TCP listener. TCP + port 0 is sufficient for v1.

</deferred>

---

*Phase: 04-embed-crate*
*Context gathered: 2026-03-27*
