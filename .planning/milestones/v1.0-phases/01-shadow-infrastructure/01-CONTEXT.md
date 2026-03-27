# Phase 1: Shadow Infrastructure - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Build a Shadow-compatible tuwunel binary (io_uring disabled) and a smoke scenario that verifies the server starts, responds to /_matrix/client/versions, and exits cleanly under Shadow's simulated network. Includes a test client binary, programmatic config generation, readiness detection, and failure diagnostics (seed, logs, PCAP).

</domain>

<decisions>
## Implementation Decisions

### Test Client Design
- **D-01:** Test client is a Rust binary using matrix-sdk as the HTTP/Matrix client library. This gives full E2EE support out of the box for Phase 2 scenarios.
- **D-02:** Single binary with clap subcommands (e.g., `matrix-test-client smoke`, `matrix-test-client auth`). Shadow YAML references the same binary with different args.
- **D-03:** Test client crate location is at Claude's discretion. Likely `tests/shadow/` (per STATE.md prior decision) but researcher/planner may choose differently based on build system constraints.

### Readiness Detection
- **D-04:** Primary: Poll `/_matrix/client/versions` endpoint in a retry loop with simulated-time backoff. Secondary: Parse Shadow's captured stdout for error diagnostics if polling fails. Belt-and-suspenders approach.

### Build Profile Strategy
- **D-05:** Dedicated Cargo profile `[profile.shadow]` in workspace `Cargo.toml` (inherits from release). Paired with explicit feature set excluding `io_uring`.
- **D-06:** Build-time enforcement via `compile_error!` — if both `io_uring` and a `shadow` cfg marker are active simultaneously, compilation fails. Catches misconfigured builds immediately.

### Port 0 / Server Changes
- **D-07:** Port 0 support approach and phasing are at Claude's discretion. Shadow's virtual networking may make port 0 unnecessary in Phase 1 (each host gets its own virtual IP, so a hardcoded port like 8448 doesn't conflict). If port 0 is deferred, it moves to Phase 4 where server changes are expected. Researcher should evaluate whether Shadow's network model eliminates the need.

### Claude's Discretion
- Test client crate location (D-03) — Claude picks based on workspace conventions and build system constraints
- Port 0 implementation approach and phase placement (D-07) — evaluate Shadow's virtual network model first
- Port exposure mechanism if port 0 is kept in Phase 1 — log + file write vs shared state

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Shadow
- `~/src/monero/shadow` — Shadow source code; reference for YAML config format and CLI invocation
- Shadow v3.3.0 docs — YAML topology format, host/process/network configuration, deterministic seed and stop_time options

### Tuwunel Build System
- `Cargo.toml` (workspace root) — workspace members, shared dependency versions, existing profiles
- `src/main/Cargo.toml` lines 60, 105-110 — io_uring feature flag definition and cascade
- `rust-toolchain.toml` — pinned nightly 1.94.0

### Tuwunel Server Internals
- `src/main/lib.rs` — public API: `exec`, `run`, `async_exec`, `async_start`
- `src/main/runtime.rs` — OnceLock statics (WORKER_AFFINITY, GC_ON_PARK, GC_MUZZY) that panic on re-init
- `src/router/serve.rs` — TCP/Unix listener binding, `get_bind_addrs()` usage, systemd listener passthrough
- `src/core/config/mod.rs` line 3171 — `get_bind_addrs()` implementation; `ListeningPort` struct at line 3079

### Existing Test Patterns
- `.planning/codebase/TESTING.md` — test framework, file organization, run commands
- `src/main/tests/` — existing smoke tests (smoke.rs, smoke_async.rs, smoke_shutdown.rs)
- `tests/complement/` — existing complement test infrastructure

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/main/lib.rs` public API (`exec`, `Server`, `Runtime`, `Args`) — the test harness can reference this for understanding server lifecycle
- `src/main/tests/smoke.rs` — existing smoke test patterns to align with
- figment-based config system — supports programmatic construction via `figment::Figment::from(Serialized::defaults(config))` without TOML files

### Established Patterns
- io_uring feature flag cascades through all 6 crates (main → router → admin → api → service → database) — disabling at workspace root propagates correctly
- Workspace uses `members = ["src/*"]` — new workspace crates go under `src/`
- Test patterns: `#[tokio::test]`, insta snapshots, separate test files declared with `mod tests;`

### Integration Points
- `config.get_bind_addrs()` returns `Vec<SocketAddr>` — the bind address pipeline where port 0 support would hook in
- `serve::serve()` in `src/router/serve.rs` — where TcpListener is bound, actual port would be captured here
- Shadow process model: Shadow runs real binaries as separate processes on virtual hosts with simulated networking

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 01-shadow-infrastructure*
*Context gathered: 2026-03-25*
