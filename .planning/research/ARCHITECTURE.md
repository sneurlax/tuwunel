# Architecture Research

**Domain:** Rust Matrix homeserver — Shadow network simulation testing + library embedding
**Researched:** 2026-03-25
**Confidence:** HIGH (based on direct codebase inspection and Shadow source/docs)

## Standard Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                        Test Harness Layer                             │
│                                                                       │
│  ┌─────────────────────┐   ┌──────────────────────────────────────┐  │
│  │   Shadow Simulation │   │   tuwunel-embed (in-process library) │  │
│  │   (Phase B)         │   │   (Phase A)                          │  │
│  │                     │   │                                      │  │
│  │  shadow.yaml        │   │  EmbeddedHomeserver {                │  │
│  │    +                │   │    server: Arc<Server>,              │  │
│  │  test-client binary │   │    base_url: String,                 │  │
│  │    +                │   │    _tempdir: TempDir,                │  │
│  │  tuwunel binary     │   │  }                                   │  │
│  └──────────┬──────────┘   └───────────────┬──────────────────────┘  │
│             │ TCP (simulated)               │ in-process TCP           │
└─────────────┼───────────────────────────────┼──────────────────────────┘
              │                               │
┌─────────────▼───────────────────────────────▼──────────────────────────┐
│                        tuwunel Binary / Library                         │
│                                                                         │
│  src/main ──► src/router ──► src/api ──► src/service ──► src/database  │
│  (exec/run)   (axum+tower)   (handlers)  (business)      (RocksDB)     │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Responsibilities

| Component | Responsibility | Location |
|-----------|----------------|----------|
| Shadow simulation harness | YAML topology definitions, wires tuwunel and test-client into a simulated network | `tests/shadow/` (new) |
| `tuwunel` binary | Stock server binary — unchanged from upstream | `src/main/` (existing) |
| Shadow test-client binary | Standalone Rust binary that exercises Matrix HTTP API; reads results to stdout/stderr for Shadow log capture | `tests/shadow/client/` (new) |
| `tuwunel-embed` crate | In-process embedded server API; wraps existing `src/main/lib.rs` lifecycle with a clean builder API | `src/embed/` (new) |
| Shadow YAML configs | Per-scenario topology files: server + N clients, configurable latency/loss | `tests/shadow/scenarios/` (new) |
| Result assertion scripts | Shell/Python scripts that grep Shadow's `shadow.data/hosts/*/stdout` files to assert test outcomes | `tests/shadow/assert/` (new) |

## Recommended Project Structure

```
tuwunel/
├── src/
│   ├── embed/                  # NEW: tuwunel-embed crate
│   │   ├── Cargo.toml
│   │   └── lib.rs              # EmbeddedHomeserver, Config builder, port-0 support
│   ├── main/                   # EXISTING (no changes for Phase B)
│   └── ... (other existing crates unchanged)
│
└── tests/
    ├── complement/             # EXISTING
    └── shadow/                 # NEW: Shadow simulation test infrastructure
        ├── Cargo.toml          # Workspace member for test-client binary
        ├── src/
        │   └── bin/
        │       └── matrix-test-client.rs   # Client binary run inside Shadow
        ├── scenarios/          # Shadow YAML topology files
        │   ├── smoke.yaml      # Minimal: 1 server + 1 client, auth+sync
        │   ├── messaging.yaml  # 1 server + 2 clients, send/receive messages
        │   ├── e2ee.yaml       # 1 server + 2 clients, E2EE key exchange
        │   └── latency.yaml    # Same as messaging but with network impairment
        ├── configs/            # Tuwunel TOML config templates for Shadow runs
        │   └── shadow-server.toml
        ├── assert/             # Post-run assertion scripts
        │   ├── check_smoke.sh
        │   └── check_messaging.sh
        └── run_tests.sh        # Top-level test runner script
```

### Structure Rationale

- **`src/embed/`:** New workspace member keeps the embed API completely separate from `src/main/`. Upstream rebases touch `src/main/` frequently; a separate crate minimizes merge conflicts. The embed crate depends on the same internal crates as `src/main/` — it is NOT a wrapper around the binary.

- **`tests/shadow/`:** Keeps all Shadow-specific infrastructure co-located. Shadow requires real binaries on disk with no whitespace in paths; having everything under `tests/shadow/` with a build step that copies artifacts there is the cleanest approach.

- **`tests/shadow/src/bin/`:** The test client is a Cargo binary compiled from this workspace member. Shadow intercepts its syscalls just like it does the server's, making network timing fully deterministic.

- **`tests/shadow/scenarios/`:** Each YAML file is a complete Shadow simulation with one or more test actions embedded as client process invocations. The server process has `expected_final_state: running`; client processes exit with code 0 on success.

- **`tests/shadow/assert/`:** Shadow captures process stdout/stderr to `shadow.data/hosts/<hostname>/<process>.stdout`. Post-run scripts grep these files to assert expected outcomes. This is simpler than trying to communicate results back through Shadow's simulated network.

## Architectural Patterns

### Pattern 1: Binary-as-Testee (Phase B)

**What:** Shadow runs the stock `tuwunel` binary unchanged, configured via a TOML file in the scenario directory. The test client binary is also compiled as a normal Rust binary and run under Shadow's syscall interception.

**When to use:** The primary pattern for all Shadow tests. Proves correctness without modifying the server.

**Trade-offs:**
- Pro: Zero changes to tuwunel server code
- Pro: Tests the real binary path including signal handling and tokio runtime startup
- Con: Cannot use `#[cfg(test)]` test modes (`fresh`/`cleanup`); the server TOML config must set `server_name = "localhost"` to satisfy the `delete_database_for_testing` guard
- Con: Binary must be compiled without `io_uring` feature for Shadow compatibility

**Example Shadow YAML:**
```yaml
general:
  stop_time: 60s
  seed: 1

network:
  graph:
    type: 1_gbit_switch

hosts:
  homeserver:
    network_node_id: 0
    processes:
    - path: /path/to/tuwunel
      args: --config ../../../configs/shadow-server.toml
      start_time: 0s
      expected_final_state: running

  client1:
    network_node_id: 0
    processes:
    - path: /path/to/matrix-test-client
      args: --server http://homeserver:6167 --test auth_and_sync
      start_time: 5s
      expected_final_state: exited
```

### Pattern 2: Embedded Homeserver (Phase A)

**What:** `tuwunel-embed` exposes a blocking `EmbeddedHomeserver::start()` function that constructs an `Arc<Server>`, builds a tokio runtime, opens RocksDB in a tempdir, starts the TCP listener on port 0 (OS-assigned), and returns the base URL. The caller uses any HTTP client against that URL. Shutdown is triggered on `Drop`.

**When to use:** In-process integration tests in the tuwunel workspace or in `matrix-rust-client`'s test harness as a replacement for Docker testcontainers.

**Trade-offs:**
- Pro: No Docker dependency; starts in ~100ms instead of 3-10s
- Pro: Multiple independent instances per test process (each gets its own tempdir and port)
- Con: All instances share the same process address space — a panic in one instance's tokio tasks can affect others
- Con: Requires port-0 support in `src/router/serve.rs` (a small addition)

**Example embed API:**
```rust
// tuwunel-embed/src/lib.rs
pub struct EmbeddedHomeserver {
    server: Arc<tuwunel::Server>,
    runtime: tuwunel::Runtime,
    base_url: String,
    _tempdir: tempfile::TempDir,
}

impl EmbeddedHomeserver {
    pub fn start() -> Result<Self> { ... }
    pub fn base_url(&self) -> &str { &self.base_url }
}

impl Drop for EmbeddedHomeserver {
    fn drop(&mut self) { /* send shutdown signal */ }
}
```

### Pattern 3: Config Construction Without Files

**What:** For both Shadow tests and the embed crate, server configuration is passed entirely via `-O key=value` command-line overrides (Shadow) or via `figment` programmatic merges (embed crate) — no TOML config file is required beyond minimal defaults.

**When to use:** Always in test contexts. The existing `Args::default_test` + `-O` mechanism already supports this pattern.

**Trade-offs:**
- Pro: Tests are self-contained; no file path management
- Pro: Existing `args::update` already handles key=value overrides via figment
- Con: Config values must be valid TOML expressions (e.g. `server_name="localhost"` not `server_name=localhost`)

**Key config overrides for test contexts:**
```
server_name = "localhost"
database_path = "/tmp/tuwunel-test-XYZ"
address = "0.0.0.0"
port = 6167            # or 0 for dynamic
allow_registration = true
listening = true
log = "warn"
allow_check_for_updates = false
```

## Data Flow

### Shadow Test Flow

```
cargo test (or run_tests.sh)
    │
    ├── cargo build --no-default-features (disables io_uring)
    │       produces: target/debug/tuwunel
    │                 target/debug/matrix-test-client
    │
    └── shadow tests/shadow/scenarios/smoke.yaml
            │
            ├── [t=0s] spawn homeserver process
            │       tuwunel --config shadow-server.toml
            │       → figment loads TOML + env overrides
            │       → Services::build + Services::start
            │       → TCP listener binds on simulated IP:6167
            │
            ├── [t=5s simulated] spawn client process
            │       matrix-test-client --server http://homeserver:6167 --test auth
            │       → reqwest/hyper makes HTTP requests
            │       → Shadow intercepts socket syscalls
            │       → packets routed through simulated topology
            │       → responses arrive with simulated latency
            │       → client writes PASS/FAIL to stdout
            │       → client exits 0 or 1
            │
            └── [t=stop_time] Shadow terminates all processes
                    → writes shadow.data/hosts/homeserver/tuwunel.stdout
                              shadow.data/hosts/client1/matrix-test-client.stdout

Post-run:
    assert/check_smoke.sh
        → grep shadow.data/hosts/client1/*.stdout for "PASS"
        → exit 0 if found, exit 1 if not
```

### Embed Test Flow

```
#[tokio::test]
async fn test_register() {
    let hs = EmbeddedHomeserver::start().unwrap();
    // hs.base_url() → "http://127.0.0.1:54321"
    //
    // EmbeddedHomeserver::start() internally:
    //   1. TempDir::new() → /tmp/tuwunel-abc123/
    //   2. Args::default_test(["smoke","fresh","cleanup"])
    //      + merge: database_path=/tmp/tuwunel-abc123
    //              port=0, address=127.0.0.1
    //   3. runtime::new() → tokio runtime
    //   4. Server::new() → Arc<tuwunel_core::Server>
    //   5. router::start() → Services::build + start
    //   6. serve::serve() → binds port 0 → gets assigned port
    //   7. return base_url

    let client = matrix_sdk::Client::builder()
        .homeserver_url(hs.base_url())
        .build().await.unwrap();

    client.register(...).await.unwrap();
    // ...

    // hs drops → shutdown signal → Services::stop() → RocksDB closes
    //          → TempDir drops → /tmp/tuwunel-abc123/ deleted
}
```

### Dependency and State Flow Between Components

```
tuwunel-core::Config  ←  figment (TOML + env + programmatic)
        ↓
tuwunel-core::Server  (holds config, runtime handle, shutdown channel)
        ↓
tuwunel-database::Database  (opens RocksDB at config.database_path)
        ↓
tuwunel-service::Services  (builds all ~40 Service instances, holds Arc<Database>)
        ↓
tuwunel-router  (builds axum Router, binds TCP listener, references Arc<Services>)
        ↓
HTTP clients  (test-client binary under Shadow, or matrix-sdk in embed tests)
```

## Scaling Considerations

| Scale | Architecture Adjustments |
|-------|--------------------------|
| 1 Shadow test scenario | Single `shadow.yaml` + 1 server + 1-2 clients; 1_gbit_switch topology |
| 5-10 scenarios | Shared YAML anchors for repeated host definitions; parameterized test runner script |
| Network impairment tests | GML graph with explicit latency/packet_loss on edges instead of `1_gbit_switch` |
| Load tests (10+ clients) | YAML anchor for client host definition; replicate N times; increase `stop_time` |
| Parallel embed test runs | Each `EmbeddedHomeserver` instance uses its own `TempDir` + port-0 binding — no coordination needed |

### Scaling Priorities

1. **First bottleneck (Shadow):** Shadow serializes time across all processes — wall-clock time grows with simulated time and process count. Keep `stop_time` as low as the test allows. Use `parallelism: 0` to let Shadow use all cores.

2. **First bottleneck (embed):** RocksDB startup is the dominant cost (~100ms per instance). Do not start/stop an `EmbeddedHomeserver` per test case — share one per test module with `once_cell::sync::Lazy` or `tokio::sync::OnceCell`.

## Anti-Patterns

### Anti-Pattern 1: Enabling `io_uring` for Shadow Builds

**What people do:** Compile tuwunel with its default feature set (which includes `io_uring`) and run it under Shadow.

**Why it's wrong:** Shadow intercepts syscalls at the ptrace/LD_PRELOAD level. `io_uring` uses ring-buffer shared memory between kernel and userspace that bypasses the syscall interception Shadow relies on. This causes Shadow to deadlock or produce incorrect results.

**Do this instead:** Build a dedicated Shadow target with `--no-default-features` and then re-enable only the features known to be Shadow-compatible. The `io_uring` feature is workspace-wide; `cargo build -p tuwunel --no-default-features --features jemalloc,brotli_compression,...` (everything except `io_uring`).

### Anti-Pattern 2: Asserting Timing in Test Clients

**What people do:** Write `sleep(Duration::from_secs(5))` in the test client and assert that a sync response has arrived by then.

**Why it's wrong:** Shadow time is simulated — wall-clock sleeps do not advance simulated time. `std::thread::sleep` blocks the simulated host forever. Use event-driven waiting: poll the endpoint with a timeout expressed as a simulated-time-aware loop (with small sleeps of simulated milliseconds) or use `reqwest`'s async retry logic.

**Do this instead:** Write test clients that loop with short `tokio::time::sleep(Duration::from_millis(100))` intervals, checking for expected state. Shadow advances simulated time proportionally, so these retries use simulated time correctly.

### Anti-Pattern 3: Global Static State in the Embed Crate

**What people do:** Use `lazy_static!` or `OnceLock` to store a single global `EmbeddedHomeserver` across the entire test binary, then call `.start()` once.

**Why it's wrong:** Multiple test binaries can run in the same process under `cargo test --test-threads=N`. A single shared server means test isolation depends on test execution order. Parallel tests that create/destroy rooms interfere with each other.

**Do this instead:** Each test or test module should own its own `EmbeddedHomeserver`. Port-0 binding ensures no conflicts. The ~100ms startup cost is acceptable once per module.

### Anti-Pattern 4: Putting the Shadow Harness in `matrix-rust-client`

**What people do:** Build the Shadow test infrastructure in the downstream consumer's repo because that's where the existing test-harness lives.

**Why it's wrong:** Shadow tests validate tuwunel's protocol behavior. They belong in the tuwunel repo where tuwunel is maintained, so they run in tuwunel's CI and catch regressions in tuwunel code changes. The `matrix-rust-client` test-harness validates the SDK against a working server — different concern.

**Do this instead:** Shadow harness lives in `tuwunel/tests/shadow/`. The `matrix-rust-client` can later use `tuwunel-embed` as a replacement for its Docker-based testcontainers setup — that is `matrix-rust-client`'s concern and a separate phase.

## Build Order

The components have clear dependency ordering:

```
Phase B (Shadow tests — stock binary, no codebase changes):
  1. Build tuwunel binary without io_uring
         cargo build -p tuwunel --no-default-features \
           --features jemalloc,brotli_compression,...
  2. Build matrix-test-client binary
         cargo build -p shadow-test-client
  3. Write Shadow YAML scenarios (no compilation)
  4. Write post-run assertion scripts (no compilation)
  5. Run first smoke scenario under Shadow
  6. Add remaining test scenarios incrementally

Phase A (embed crate — requires codebase addition):
  7. Add port-0 support to src/router/serve.rs
         (minor: return bound port from serve::serve)
  8. Add tuwunel-embed crate (new workspace member)
         Depends on: tuwunel-core, tuwunel-router (same as src/main)
  9. Write programmatic Config construction helpers in tuwunel-embed
 10. Write embed integration tests using matrix-sdk
```

**Why Phase B before Phase A:** Phase B establishes a behavioral baseline with zero code changes. If a test fails, the bug is in the test harness or Shadow config, not in tuwunel code. Phase A adds code; any new test failures can then be attributed to those changes.

**Within Phase B, ordering matters:**

1. **Smoke scenario first** (auth + `/_matrix/client/versions` check): Proves Shadow can run tuwunel at all before investing in complex scenarios.
2. **Messaging second**: Proves client-server request/response cycle works under simulated network.
3. **E2EE third**: Most complex; requires multiple sync rounds with Olm/Megolm key exchange. Build on proven smoke + messaging foundation.
4. **Network impairment last**: Only add latency/loss scenarios after deterministic tests pass — impairment tests verify resilience, not correctness.

## Integration Points

### External Services

| Service | Integration Pattern | Notes |
|---------|---------------------|-------|
| Shadow (process runner) | Binary invocation: `shadow scenario.yaml` | Requires no-whitespace paths; output in `shadow.data/` |
| RocksDB | Embedded via `rust-rocksdb` crate | One DB per server instance; `TempDir` handles cleanup |
| tokio runtime | Each `EmbeddedHomeserver` owns its runtime | `runtime::new()` in `src/main/runtime.rs` |
| figment config | Programmatic merge via `Figment::new().merge(...)` | No TOML file required for test configs |

### Internal Boundaries

| Boundary | Communication | Notes |
|----------|---------------|-------|
| `tuwunel-embed` ↔ `tuwunel` (main lib) | Direct Rust crate dependency | Embed crate calls `tuwunel::Server::new` + `tuwunel::exec` (or `async_start`/`async_run`/`async_stop` separately for lifetime control) |
| `tuwunel-embed` ↔ `src/router` | Direct call to `router::start(server)` | Same path as the binary; no new interface needed |
| `shadow test-client` ↔ `tuwunel binary` | HTTP over simulated TCP | `reqwest` in test-client; axum in server; Shadow intercepts both |
| `shadow test-client` ↔ post-run asserter | stdout/stderr files in `shadow.data/` | Client writes structured output (e.g., `PASS: auth`, `FAIL: sync timeout`); asserter greps |
| `tuwunel-embed` ↔ test code | Rust `Drop`-based lifecycle | `EmbeddedHomeserver` RAII: starts on `new()`, stops on `drop()` |

## Sources

- Direct inspection of `src/main/lib.rs`, `src/main/server.rs`, `src/main/args.rs` (HIGH confidence)
- Direct inspection of `src/router/mod.rs`, `src/router/run.rs` (HIGH confidence)
- Direct inspection of `src/database/engine/context.rs` (HIGH confidence — test mode `fresh`/`cleanup` behavior)
- Shadow 3.3.0 YAML examples at `~/src/monero/shadow/examples/` (HIGH confidence — local source)
- Shadow config documentation at https://shadow.github.io/docs/guide/shadow_config_spec.html (HIGH confidence)
- Direct inspection of `matrix-rust-client/testing/test-harness/src/lib.rs` (HIGH confidence — shows existing Docker harness to be replaced)
- `.planning/PROJECT.md` constraints section (HIGH confidence — io_uring, no-whitespace-paths, tempdir isolation)

---
*Architecture research for: tuwunel Shadow simulation testing + library embedding*
*Researched: 2026-03-25*
