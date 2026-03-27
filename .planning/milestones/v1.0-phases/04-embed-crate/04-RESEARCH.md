# Phase 4: Embed Crate - Research

**Researched:** 2026-03-27
**Domain:** Rust crate design, in-process server embedding, tuwunel lifecycle API
**Confidence:** HIGH

## Summary

The `tuwunel-embed` crate wraps tuwunel's existing server lifecycle (`tuwunel-router::start/run/stop`) into an `EmbeddedHomeserver` API. The key technical challenges are: (1) making OnceLock statics in `src/main/runtime.rs` multi-call-safe via `get_or_init()`, (2) making logging initialization idempotent to avoid `set_global_default` panics, (3) extracting the actual bound port when using port 0 via `axum_server::Handle::listening()`, and (4) managing per-instance `Server` + `Services` + RocksDB tempdir lifetimes.

The existing architecture is well-suited for embedding. `tuwunel-router` already exposes `start()`, `run()`, and `stop()` as public functions taking `Arc<Server>` / `Arc<Services>`. The embed crate bypasses `src/main/` entirely for runtime management but needs targeted patches to `src/main/runtime.rs` (OnceLock) and `src/main/logging.rs` (idempotent init) to make them safe for multi-instance use.

**Primary recommendation:** Create `src/embed/` as a thin wrapper crate that constructs `Config` via figment programmatically, creates its own tokio runtime (or reuses the caller's), and delegates all server lifecycle to `tuwunel-router`. Modify 3 existing files (`runtime.rs`, `logging.rs`, `server.rs`) with minimal, backwards-compatible changes.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Shared runtime, per-instance server. One tokio runtime and one logging init shared across all `EmbeddedHomeserver` instances. OnceLock statics in `runtime.rs` (`WORKER_AFFINITY`, `GC_ON_PARK`, `GC_MUZZY`) changed from `.set().expect()` to `get_or_init()` -- first caller wins, subsequent calls are no-ops. Each instance gets its own `Server`, `Services`, and RocksDB tempdir.
- **D-02:** Logging init (`tracing::subscriber::set_global_default`) called once, guarded against double-registration panics. Subsequent instances skip logging init. Tracing is inherently global -- per-instance logging is not needed.
- **D-03:** Primary API is minimal params with sensible defaults: `EmbeddedHomeserver::start("server-name")` constructs config internally with auto-provisioned tempdir, port 0, and registration disabled. An optional builder pattern (`EmbeddedHomeserver::builder().server_name("x").port(0).registration_token("y").build().start()`) is available for customization.
- **D-04:** Config constructed via figment internally -- consumers never touch figment directly. Builder fields map to tuwunel config keys. Defaults optimized for testing: minimal logging, no TLS, no federation, ephemeral database.
- **D-05:** In-memory transport deferred to v2. Phase 4 ships with TCP-only transport. Port 0 (OS-assigned) eliminates port conflicts.
- **D-06:** Thin wrapper over existing public API. Embed crate depends on `tuwunel-router` directly (for `start`/`run`/`stop`) and `tuwunel-core` (for `Server`, `Config`). No forking of `src/main/` functions.
- **D-07:** Embed crate provides its own config construction and runtime management but delegates server lifecycle to `tuwunel-router::start()` / `run()` / `stop()`. Skips CLI arg parsing, signal handling, hot-reload, and systemd notification.
- **D-08:** Port 0 support at Claude's discretion for implementation approach.

### Claude's Discretion
- Port 0 implementation approach (D-08) -- how to propagate bound port from listener to embed API
- Builder pattern ergonomics -- which config fields to expose, naming conventions
- Whether `register_user()` uses the existing MatrixClient from test harness or standalone reqwest calls
- Runtime thread count defaults for embed use case (likely lower than production defaults)

### Deferred Ideas (OUT OF SCOPE)
- **EMBD-10: In-memory axum Router transport** -- Deferred to v2. TCP + port 0 is sufficient for v1.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| EMBD-01 | `tuwunel-embed` crate exists as new workspace member with `EmbeddedHomeserver` struct | Workspace uses `members = ["src/*"]` glob -- placing crate at `src/embed/` auto-includes it |
| EMBD-02 | `EmbeddedHomeserver::start(config)` starts a tuwunel server in-process and returns when ready | `tuwunel-router::start()` builds Services, `run()` starts listener; embed spawns run() as background task |
| EMBD-03 | `EmbeddedHomeserver::base_url()` returns URL with actual bound port | `axum_server::Handle::listening()` returns `Option<SocketAddr>` after bind -- provides actual port for port 0 |
| EMBD-04 | `EmbeddedHomeserver::stop()` performs graceful shutdown via broadcast channel | `tuwunel_core::Server::shutdown()` sends SIGTERM via broadcast; `tuwunel-router::stop()` handles cleanup |
| EMBD-05 | Auto-provisioned tempdir for RocksDB | Use `tempfile::TempDir` -- owned by `EmbeddedHomeserver`, dropped on stop |
| EMBD-06 | Multiple instances run concurrently in same process | Per D-01: shared runtime, per-instance Server/Services/RocksDB; OnceLock `get_or_init()` prevents panics |
| EMBD-07 | Tracing/logging init guarded against double-registration panics | Wrap `set_global_default` in `try` or use `Once` guard; subsequent calls skip |
| EMBD-08 | OnceLock statics in runtime.rs handled safely for embed use | Change `.set().expect()` to `.get_or_init(\|\| value)` -- first caller wins |
| EMBD-09 | `register_user()` convenience method registers user via Matrix API | Use reqwest + ruma JSON directly (same pattern as `tests/shadow/src/scenarios/common.rs::MatrixClient`) |
| EMBD-10 | In-memory HTTP transport via extracted axum Router | **DEFERRED to v2** per D-05 |
</phase_requirements>

## Architecture Patterns

### Recommended Crate Structure
```
src/embed/
  Cargo.toml
  mod.rs          # lib root: pub struct EmbeddedHomeserver, pub fn, re-exports
  config.rs       # Builder pattern, figment construction, defaults
  tests.rs        # Integration tests (multi-instance, port 0, register_user)
```

### Pattern 1: Server Lifecycle Delegation
**What:** The embed crate does NOT replicate `src/main/` logic. It constructs a `tuwunel_core::Config` via figment, creates a `tuwunel_core::Server`, then calls `tuwunel_router::start()` and spawns `tuwunel_router::run()` as a background tokio task. Stop calls `tuwunel_core::Server::shutdown()` then `tuwunel_router::stop()`.
**When to use:** Always -- this is the only pattern for v1.
**Key insight:** The `start()` in `tuwunel-router/run.rs` takes `Arc<Server>` and returns `Arc<Services>`. The `run()` takes `Arc<Services>` and blocks until shutdown signal. The embed crate spawns `run()` on the runtime and keeps the `Arc<Services>` for later `stop()`.

### Pattern 2: Port 0 via Handle::listening()
**What:** `axum_server::Handle<SocketAddr>` has an async `listening()` method that returns `Option<SocketAddr>` once the server binds. For port 0 (OS-assigned), this returns the actual bound address. The embed crate needs to access this handle to extract the port.
**Challenge:** The `Handle` is created inside `tuwunel-router/run.rs::run()` as a local `ServerHandle` and is not currently exposed. There are two approaches:
1. **Recommended: Poll `/_matrix/client/versions` with port from TcpListener pre-bind.** The embed crate binds a `std::net::TcpListener` to port 0, extracts the local port, then passes that port to the config. The OS reserves the port. When axum-server starts, it binds to the same port. This avoids modifying `src/router/`.
2. **Alternative: Modify serve.rs to expose bound address.** Add a mechanism to propagate the bound address back. More invasive.

**Recommended approach for port 0:** Pre-bind a `TcpListener` on port 0, extract `local_addr().port()`, close it, then configure tuwunel with that port. There is a theoretical TOCTOU race, but in practice for testing, this is sufficient. A more robust approach would be to hold the listener open and pass it through, but the current `axum_server::bind()` in `serve/plain.rs` binds its own listener.

**Better alternative: Use `config.listening` + direct TcpListener.** Actually, looking at `serve/plain.rs` more carefully, `axum_server::from_tcp(listener)` accepts pre-bound listeners (used for systemd socket activation). The embed crate could bind a `TcpListener` on port 0, record the address, and pass it through. However, this requires modifying the router to accept passed-in listeners for the non-systemd path.

**Simplest working approach:** Pre-bind on port 0, extract port, drop listener, set port in config. Readiness detected by polling `/_matrix/client/versions` (already proven pattern from Shadow tests). The TOCTOU race window is negligible in test contexts.

### Pattern 3: Config Construction via Figment
**What:** Construct `Figment` programmatically using `figment::providers::Serialized` or direct key-value pairs via `Figment::from(("key", value))` chaining, then extract into `tuwunel_core::Config`.
**Example:**
```rust
use figment::Figment;
use figment::providers::Serialized;

let figment = Figment::new()
    .merge(("server_name", server_name))
    .merge(("database_path", db_path.to_str().unwrap()))
    .merge(("port", port))
    .merge(("address", "127.0.0.1"))
    .merge(("listening", true))
    .merge(("allow_registration", false))
    .merge(("startup_netburst", false))
    .merge(("log", "warn"))
    .merge(("log_global_default", false)); // skip global subscriber

let config = tuwunel_core::config::Config::new(&figment)?;
```
**Key detail:** `Config::load()` reads env vars (`TUWUNEL_CONFIG`, etc.) and TOML files -- the embed crate should NOT call `Config::load()`. Instead, construct `Figment::new()` directly and only call `Config::new(&figment)` to deserialize.

### Pattern 4: Logging Idempotency Guard
**What:** `src/main/logging.rs::init()` calls `set_global_default(subscriber)` which panics if called twice. Two approaches:
1. **Guard in embed crate:** Set `log_global_default: false` in config, handle subscriber in embed code using `try_init()` or `set_default()` (per-thread, not global).
2. **Guard in logging.rs:** Wrap `set_global_default` call with a `std::sync::Once` or `OnceLock<bool>` check. More robust but modifies upstream code.

**Recommended:** Use config option `log_global_default: false` for embed instances. This skips the problematic `set_global_default` call entirely (see logging.rs line 142-144). The embed crate can optionally set up its own subscriber for the first instance. This requires ZERO changes to `logging.rs`.

### Anti-Patterns to Avoid
- **Forking src/main/ functions:** Do not copy/paste `Server::new()` or `runtime::new()`. Use the existing functions with modified inputs.
- **Creating a separate tokio runtime per instance:** The runtime is heavyweight. Share one runtime across instances per D-01.
- **Using Args::default() in embed:** This calls `clap::Parser::parse()` which reads process CLI args. Construct `Args` fields directly instead.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Temp directory management | Manual mkdtemp + cleanup | `tempfile::TempDir` | Auto-cleanup on drop, unique names, handles edge cases |
| Config construction | Manual TOML string building | `figment::Figment` with `merge()` pairs | Type-safe, matches tuwunel's own config system |
| User registration | Custom HTTP/Matrix protocol impl | Reqwest + JSON (same pattern as shadow MatrixClient) | Already proven in Phase 2, handles UIAA flow |
| Server readiness detection | Custom health check | Poll `/_matrix/client/versions` | Standard Matrix approach, already implemented in shadow common.rs |
| Port allocation | Manual port scanning | Port 0 (OS-assigned) | Kernel guarantees no collision |

## Common Pitfalls

### Pitfall 1: OnceLock Panic on Second runtime::new() Call
**What goes wrong:** `runtime::new()` calls `WORKER_AFFINITY.set(val).expect("set WORKER_AFFINITY...")` which panics if called twice.
**Why it happens:** OnceLock::set returns Err if already initialized, and `.expect()` panics on Err.
**How to avoid:** Change to `WORKER_AFFINITY.get_or_init(|| val)` for all three statics. First caller wins. This is backwards-compatible -- the main binary only calls `runtime::new()` once anyway.
**Warning signs:** Panic message "set WORKER_AFFINITY from program argument".

### Pitfall 2: set_global_default Panic
**What goes wrong:** `tracing::subscriber::set_global_default()` panics if called more than once per process.
**Why it happens:** Global tracing subscriber is a process-wide singleton. Second call fails.
**How to avoid:** Use `log_global_default: false` in embed config. This causes `logging::init()` to skip the `set_global_default` call entirely (logging.rs line 142-144). The embed crate either sets up its own subscriber once or lets the calling test framework handle logging.
**Warning signs:** Panic with "the global default tracing subscriber failed to be initialized".

### Pitfall 3: Args::default() Parses CLI Args
**What goes wrong:** `Args::default()` calls `clap::Parser::parse()` which reads `std::env::args()`. In test contexts, this picks up cargo-test arguments (like `--test-threads=1`) and may fail or produce wrong values.
**Why it happens:** The `Default` impl for `Args` delegates to `Args::parse()`.
**How to avoid:** The embed crate should NOT use `Args` at all. Construct the tokio runtime directly using `tokio::runtime::Builder`, and construct `Config` via `Figment` without going through `Server::new()` from `src/main/server.rs`. Build the `tuwunel_core::Server` struct directly.
**Warning signs:** Unexpected clap errors or wrong default values in test runs.

### Pitfall 4: Config Env Var Pollution
**What goes wrong:** `Config::load()` reads `TUWUNEL_CONFIG`, `CONDUIT_CONFIG`, and `CONDUWUIT_CONFIG` env vars, and merges `TUWUNEL__*` env overrides. If set in the environment, they contaminate embed config.
**Why it happens:** `Config::load()` is designed for the standalone binary use case.
**How to avoid:** Do NOT call `Config::load()`. Construct `Figment::new()` directly and call `Config::new(&figment)` only. This bypasses all env var and file reading.
**Warning signs:** Config values coming from unexpected sources.

### Pitfall 5: RocksDB Tempdir Dropped Too Early
**What goes wrong:** If `TempDir` is dropped (goes out of scope) while the server is still running, the RocksDB directory is deleted, causing database errors.
**Why it happens:** `TempDir` deletes its directory on drop.
**How to avoid:** Store `TempDir` as a field of `EmbeddedHomeserver` so it lives as long as the server. Only drop after `stop()` completes.
**Warning signs:** RocksDB "file not found" or corruption errors during server operation.

### Pitfall 6: sys::maximize_fd_limit Panic
**What goes wrong:** `Server::new()` in `src/main/server.rs` calls `sys::maximize_fd_limit().expect(...)` and `sys::maximize_thread_limit().expect(...)`. These may fail in sandboxed environments.
**Why it happens:** These modify process-wide resource limits.
**How to avoid:** The embed crate should call these once (they're idempotent -- raising limits is fine). Or better: since the embed crate bypasses `src/main/server.rs::Server::new()`, it constructs `tuwunel_core::Server::new(config, runtime, logger)` directly, which does NOT call these functions. The embed crate can optionally call them once if needed.
**Warning signs:** "Unable to increase maximum file descriptor limit" panic.

### Pitfall 7: Port 0 TOCTOU Race
**What goes wrong:** Pre-bind on port 0, extract port, close listener, then configure tuwunel with that port. Between close and tuwunel's bind, another process may claim the port.
**Why it happens:** The port becomes available again after the listener is closed.
**How to avoid:** In testing contexts this is extremely unlikely. For robustness: bind the listener, keep it open, set `SO_REUSEADDR`, and configure tuwunel with the port. Or accept the tiny risk for v1.
**Warning signs:** "Address already in use" error on server start.

## Code Examples

### Figment Programmatic Config Construction
```rust
// Source: figment docs + tuwunel Config::new() analysis
use figment::Figment;
use tuwunel_core::config::Config;

fn build_embed_config(
    server_name: &str,
    database_path: &str,
    port: u16,
) -> tuwunel_core::Result<Config> {
    let figment = Figment::new()
        .merge(("server_name", server_name))
        .merge(("database_path", database_path))
        .merge(("port", port))
        .merge(("address", "127.0.0.1"))
        .merge(("listening", true))
        .merge(("allow_registration", false))
        .merge(("startup_netburst", false))
        .merge(("log", "warn"))
        .merge(("log_global_default", false));

    Config::new(&figment)
}
```

### OnceLock get_or_init() Fix (runtime.rs)
```rust
// Before (panics on second call):
WORKER_AFFINITY
    .set(args.worker_affinity)
    .expect("set WORKER_AFFINITY from program argument");

// After (first caller wins, no panic):
WORKER_AFFINITY.get_or_init(|| args.worker_affinity);
```

### Server Lifecycle in Embed Crate
```rust
// Source: analysis of src/main/lib.rs + src/router/mod.rs
use std::sync::Arc;
use tuwunel_core::Server as CoreServer;

// 1. Build config via figment (no CLI, no env vars)
let config = build_embed_config(server_name, db_path, port)?;

// 2. Create logger (with log_global_default: false)
let (flame_guard, logger) = tuwunel::logging::init(&config)?;

// 3. Create core server
let core_server = Arc::new(CoreServer::new(config, Some(&runtime_handle), logger));

// 4. Start services (builds DB, runs migrations)
let services = tuwunel_router::start(&core_server).await?;

// 5. Spawn run() as background task (this starts the listener)
let run_handle = tokio::spawn(tuwunel_router::run(&services));

// 6. Wait for readiness (poll versions endpoint)
poll_versions(&base_url).await?;

// 7. To stop: signal shutdown, then call stop()
core_server.shutdown()?;
run_handle.await??;  // run() returns after shutdown signal
tuwunel_router::stop(services).await?;
```

### Port 0 Extraction
```rust
// Pre-bind to get OS-assigned port
let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
let port = listener.local_addr()?.port();
drop(listener); // Release so tuwunel can bind
// Use `port` in config construction
```

## Discretion Recommendations

### Port 0 Implementation (D-08)
**Recommendation:** Pre-bind + extract + drop approach. Simple, no upstream changes needed. The TOCTOU race is negligible for testing. Readiness confirmed by polling `/_matrix/client/versions`.

### Builder Pattern Fields
**Recommendation:** Expose these fields in the builder:
- `server_name` (required, String)
- `port` (default: 0 = OS-assigned)
- `address` (default: "127.0.0.1")
- `registration_token` (Option<String>, enables registration when set)
- `log_level` (default: "warn")
- `database_path` (Option<PathBuf>, default: auto tempdir)
- `worker_threads` (default: 2, lower than production)

### register_user() Implementation
**Recommendation:** Standalone reqwest calls using the same UIAA two-step flow proven in `tests/shadow/src/scenarios/common.rs::MatrixClient::register_with_token()`. Do NOT depend on the shadow test crate. Copy the essential HTTP logic (register step 1 + step 2) into the embed crate. This keeps the embed crate self-contained.

### Runtime Thread Count
**Recommendation:** Default `worker_threads: 2` for embed use case. This is the minimum (`WORKER_THREAD_MIN` in runtime.rs) and sufficient for testing. Keeps resource usage low when running multiple instances.

## Upstream Code Changes Required

| File | Change | Impact | Backwards Compatible |
|------|--------|--------|---------------------|
| `src/main/runtime.rs` L45-55 | `.set().expect()` to `.get_or_init()` for 3 OnceLock statics | First caller wins; no behavioral change for single-call case | YES |
| (None needed for logging) | N/A | Config `log_global_default: false` already handles this | N/A |

**Key insight:** Only ONE file needs modification in upstream code. The logging issue is solved by config, not code changes.

## Project Constraints (from CLAUDE.md)

- Hard tabs, max 98 char lines, edition 2024
- Imports grouped with `StdExternalCrate`
- Import granularity at crate level
- `unwrap_used = "warn"` -- no unwrap outside tests
- `as_conversions = "warn"`, `arithmetic_side_effects = "warn"`
- Workspace members follow `src/{crate_name}/` directory convention
- All crates use `[workspace.dependencies]` for shared deps
- New crate at `src/embed/` is auto-included via `members = ["src/*"]`
- Fork changes must be minimal and rebaseable on upstream

## Open Questions

1. **tuwunel_router::start/run/stop are `extern "Rust"` functions**
   - What we know: They're marked `#[unsafe(no_mangle)] pub extern "Rust"` for dynamic module loading (hot-reload). They take `Arc<Server>` / `Arc<Services>` and return boxed futures.
   - What's unclear: Whether calling them from another crate in the same binary (static linking) works without issues. The `extern "Rust"` annotation should be transparent for static linking.
   - Recommendation: Verify by calling `tuwunel_router::start()` directly. The boxed future return type is a minor ergonomic inconvenience but functional.

2. **Config::check() validation**
   - What we know: It warns about loopback addresses in containers, checks TLS config consistency, warns when listening=false.
   - What's unclear: Whether it rejects any config values that are valid for embed but look wrong for production (e.g., port 0 might be rejected).
   - Recommendation: Call `config.check()` and handle any errors. Port 0 passes through `get_bind_addrs()` without validation issues since it's just a u16.

3. **Multiple RocksDB instances memory usage**
   - What we know: Each instance gets its own tempdir and RocksDB instance. RocksDB is configured per-instance.
   - What's unclear: Default RocksDB block cache and memory settings may be tuned for production. Two instances may use excessive memory.
   - Recommendation: Accept defaults for v1. If memory is an issue in tests, add `rocksdb_cache_capacity_mb` to the builder.

## Sources

### Primary (HIGH confidence)
- `src/main/runtime.rs` -- OnceLock statics (lines 34-36, 45-55), verified panicking `.set().expect()` pattern
- `src/main/logging.rs` -- `set_global_default` call (line 178-180), `log_global_default` config gate (line 142-144)
- `src/main/server.rs` -- `Server::new()` constructor, dependency on `Args` and `logging::init()`
- `src/main/lib.rs` -- Public API: `async_start()`, `async_run()`, `async_stop()`
- `src/router/mod.rs` -- `start()`, `run()`, `stop()` extern fns
- `src/router/run.rs` -- `start()` builds Services, `run()` starts listener + admin
- `src/router/serve.rs` -- TCP binding via `axum_server::bind()`
- `src/router/serve/plain.rs` -- `axum_server::bind(*addr)` per address
- `axum-server-0.8.0/src/handle.rs` -- `Handle::listening()` returns bound address (line 79-89)
- `src/core/config/mod.rs` -- `Config::load()` (env vars), `Config::new()` (figment extract), port/address fields
- `src/core/server.rs` -- `Server::new(config, runtime, logger)`, `shutdown()` via broadcast channel
- `Cargo.toml` -- workspace `members = ["src/*"]` glob includes src/embed/

### Secondary (MEDIUM confidence)
- `tests/shadow/src/scenarios/common.rs` -- MatrixClient UIAA registration flow (proven pattern for register_user)
- `tests/shadow/src/config/tuwunel.rs` -- Programmatic config construction pattern (serde approach vs figment)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - all dependencies are already in workspace, no new external deps needed except `tempfile`
- Architecture: HIGH - direct code analysis of all lifecycle functions, handle API, and config system
- Pitfalls: HIGH - each pitfall identified from specific code lines in the source

**Research date:** 2026-03-27
**Valid until:** 2026-04-27 (stable -- tuwunel internals unlikely to change in 30 days)
