# Stack Research

**Domain:** Rust homeserver E2E testing with Shadow network simulation
**Researched:** 2026-03-25
**Confidence:** MEDIUM — Shadow+Rust is well-documented; Matrix client test patterns are inference from adjacents; no prior art for tuwunel+Shadow specifically

---

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| Shadow | 3.3.0 (installed `~/.local/bin/shadow`) | Network simulation orchestrator | The only tool that runs real unmodified Linux binaries under a deterministic discrete-event simulation with configurable latency/loss/bandwidth. No code changes to tuwunel needed. Runs stock tuwunel binary under syscall interception. |
| tuwunel (stock binary) | current HEAD | Server under test | Shadow requires a real Linux binary — you point Shadow at the compiled tuwunel binary. The existing `src/main/lib.rs` API (`exec`, `Server`, `Runtime`, `Args`) is the embed surface for Phase A; binary mode is all that's needed for Phase B. |
| Rust 1.94.0 (nightly) | pinned in `rust-toolchain.toml` | Test harness binary language | Test clients and orchestration scripts must be Rust binaries that Shadow can execute. Must match the workspace toolchain to share Cargo.lock and avoid duplicate compilation. |
| reqwest | 0.13 (already in workspace) | HTTP test client inside Shadow | The standard Rust HTTP client. Test client binaries use it to call tuwunel's Matrix Client-Server API from within Shadow's simulated network. Reuse the existing workspace dependency — no new version to negotiate. |
| YAML (Shadow config) | YAML 1.2 | Shadow simulation definition | Shadow's ONLY configuration interface. Defines network topology (GML graph), hosts (virtual nodes), and processes (binaries to run). No alternative — Shadow requires this format. |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile | 3.x (add to workspace) | Temporary database directories | Required for the embed crate (Phase A). Each embedded tuwunel instance needs its own `tempdir()` for RocksDB — multiple instances in the same process need separate paths. Shadow Phase B does not need this (each Shadow host gets its own filesystem namespace). |
| tokio | 1.50 (existing) | Async runtime for test clients | Test client binaries that make async HTTP calls. Reuse workspace version — no negotiation needed. |
| figment | 0.10 (existing) | Programmatic config construction | The embed crate (Phase A) needs to build `Config` without a file on disk. Figment's `Figment::from(...)` + merge/join API supports fully programmatic construction. Already used by tuwunel. |
| insta | 1.43 (existing) | Snapshot assertions | Already used in smoke tests. Use for asserting Matrix API response shapes across test runs. |
| serde_json | 1.0 (existing) | Parsing Matrix API responses | Test clients parse JSON responses from the Matrix C-S API. Already in workspace. |
| clap | 4.5 (existing) | Test binary CLI argument parsing | Shadow passes `args` to each process via its YAML config. Test binaries need `--server-url`, `--user-id`, `--scenario` flags. Already in workspace. |

### Development Tools

| Tool | Purpose | Notes |
|------|---------|-------|
| Shadow 3.3.0 | Run simulation from CLI | `~/.local/bin/shadow <config.yaml>`. Produces per-host log files in `shadow.data/hosts/<hostname>/`. |
| `cargo build --profile release` | Build tuwunel binary for Shadow | Shadow runs the release binary. Debug builds work but are slower and may have different performance characteristics. Use `--no-default-features` to disable `io_uring` for Shadow builds (see What NOT to Use). |
| `cargo build --bin <test-client>` | Build test client binaries | Each test scenario is a separate Rust binary in a `[[bin]]` section. Shadow's YAML points to the compiled binary path. |
| `cargo test` | Run in-process embed tests (Phase A) | The existing smoke test pattern (`Args::default_test(&["smoke", "fresh", "cleanup"])`) is the model for in-process tests. |
| `insta review` | Review snapshot changes | `cargo insta review` for interactive snapshot approval after test output changes. |

---

## Installation

```bash
# Shadow is already installed at ~/.local/bin/shadow (v3.3.0)
# Verify:
~/.local/bin/shadow --version

# Add tempfile to workspace Cargo.toml (for Phase A embed crate only)
# In [workspace.dependencies]:
# tempfile = "3"

# Build tuwunel without io_uring for Shadow compatibility:
cargo build --release --no-default-features \
  --features "brotli_compression,element_hacks,gzip_compression,jemalloc,jemalloc_conf,media_thumbnail,release_max_log_level,systemd,url_preview,zstd_compression"

# Build a test client binary:
cargo build --release --bin shadow-test-client
```

---

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| Shadow (binary simulation) | Turmoil / MadSim (in-process simulation) | Turmoil/MadSim are better when you control all code and want sub-millisecond test iteration. Shadow is better when you're testing a real server binary you don't fully control — it runs the actual tuwunel binary unchanged, validating the full stack including RocksDB I/O and real TCP. |
| Shadow (binary simulation) | testcontainers / Docker | Docker-based tests are already in matrix-rust-client and suffer from 3-10s startup times, timing flakes, and no network simulation. Shadow replaces this with deterministic time, configurable network conditions, and faster startup (no container overhead). |
| Shadow (binary simulation) | Complement (matrix-org/complement) | Complement is the official Matrix compliance suite but requires Docker and Go, tests against a running server over real HTTP, and is not deterministic. Use Complement for compliance verification against the Matrix spec; use Shadow for network-condition regression tests. |
| reqwest (async HTTP client) | hyper direct / matrix-sdk client | reqwest is idiomatic and already in the workspace. matrix-sdk is too high-level and brings too many dependencies for test binaries; direct hyper is too low-level. reqwest hits the sweet spot: standard Matrix C-S API calls without ceremony. |
| `[[bin]]` in new crate | `[[test]]` Rust test harness | Shadow requires real Linux binaries with `main()`. Rust's `#[test]` harness produces a special binary that expects `--test` flags and captures output. Use `[[bin]]` entries in a dedicated `tuwunel-shadow-tests` crate so Shadow can invoke them cleanly. |
| Cargo workspace binary | Shell scripts | Shell scripts have no type safety, can't use ruma types, and complicate dependency management. Rust binaries can import workspace crates directly if needed. |

---

## What NOT to Use

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `io_uring` feature in Shadow builds | Shadow intercepts syscalls via LD_PRELOAD + seccomp filter. `io_uring` bypasses traditional syscall hooks through its own submission queue interface — Shadow cannot reliably intercept `io_uring` operations. The `io_uring` feature is a tuwunel default but must be disabled for Shadow builds. | Build tuwunel with `--no-default-features` plus all features except `io_uring`. Standard epoll (tokio's default without `io_uring`) is fully supported by Shadow. |
| Statically linked tuwunel binary | Shadow requires `LD_PRELOAD` to inject its shim library. Statically linked binaries (musl targets) cannot be intercepted. | Use `x86_64-unknown-linux-gnu` (glibc) target for Shadow builds, not `x86_64-unknown-linux-musl`. |
| IPv6 addressing in Shadow configs | Shadow does not implement IPv6 (tracking issue #2216). | Configure tuwunel with `address = "0.0.0.0"` (IPv4) and use IPv4 addresses in Shadow's network graph. |
| `SO_REUSEADDR` socket options | Shadow does not support `SO_REUSEADDR`. Tuwunel does not set this by default on its listening sockets, so this is not a current concern — but avoid adding it in test infrastructure. | Assign fixed ports per host in Shadow's YAML; no port reuse is needed since each Shadow host is a separate virtual node. |
| `sendfile()` syscall | Shadow does not support `sendfile()`. Tuwunel uses RocksDB and hyper body streaming — neither calls `sendfile()` directly — so this is not currently a concern. | No action needed; document as a known watch-out if hyper/axum adds sendfile optimizations in future versions. |
| TCP_FASTOPEN | Shadow does not support TCP_FASTOPEN. Again, tuwunel does not enable this by default. | No action needed; do not enable `TCP_FASTOPEN` in test builds. |
| Whitespace in binary paths | Shadow's LD_PRELOAD shim requires that all paths (binary, config files, data dirs) contain no whitespace. | Keep all Shadow test infrastructure under paths like `/home/user/src/manymatrix/tuwunel/` — no spaces. |
| Port 0 (dynamic port) in Shadow | Shadow's simulated network assigns ports at process bind time but the simulation doesn't have a mechanism to discover what port a process chose after the fact. Test clients need to know the server's port before the simulation starts. | Assign a fixed port (e.g., 8008) in tuwunel's config via the `port` setting and reference it explicitly in the Shadow YAML and test client args. |
| `vfork()` in test infrastructure | Shadow implements `vfork()` as `fork()`, so child-modifies-parent-address-space patterns break. | Avoid `std::process::Command` patterns that rely on `vfork()` semantics inside Shadow processes. Use tokio's `spawn` or async patterns instead. |
| Rust `#[test]` harness binaries as Shadow processes | The standard Rust test harness binary parses `--test`, `--nocapture`, etc. and produces TAP-like output. Shadow can run these but the output parsing is awkward and the binary interface is unstable. | Use `[[bin]]` entries with a proper `main()` that exits with code 0 on success and non-zero on failure. Shadow's `expected_final_state: {exited: 0}` then gives a clean pass/fail signal. |
| Running Shadow with `--parallelism > 1` during initial debugging | Multi-threaded Shadow (parallel hosts) makes log output interleaved and harder to read. | Start with `parallelism: 1` (single-threaded simulation) during development; enable parallelism only after tests pass deterministically. |

---

## Stack Patterns by Variant

**Phase B (Shadow tests on stock tuwunel binary):**
- Build tuwunel release binary without `io_uring`, targeting glibc
- Write test client as `[[bin]]` in a new `tuwunel-shadow-tests` workspace crate
- Test client uses `reqwest` to make Matrix C-S API calls (register, login, sync, send message)
- Shadow YAML defines: 1 server host running tuwunel, 1+ client hosts running the test binary
- Pass server config via `TUWUNEL__<KEY>` environment variables in Shadow's `environment:` block
- Shadow's `expected_final_state: {exited: 0}` on the test client binary = test passed

**Phase A (embed crate for in-process testing):**
- New `tuwunel-embed` crate wraps `tuwunel::Server`, `tuwunel::runtime`, and `tuwunel::exec`
- Uses `tempfile::TempDir` for per-instance RocksDB isolation
- Config constructed programmatically via `figment::Figment` — no config file needed
- Server runs in background tokio task; client talks to it over localhost TCP
- Shutdown via `tuwunel_core::Server`'s broadcast channel
- Existing `Args::default_test(&["smoke", "fresh", "cleanup"])` pattern is the model

**If multiple concurrent embedded servers (in-process, Phase A):**
- Each `TempDir` is separate: RocksDB supports multiple instances in one process with distinct paths
- Use different ports (or port 0 + `listener.local_addr()` to discover) for each instance
- Separate tokio runtimes or shared runtime with separate task trees — shared runtime is simpler

**If Shadow federation tests (multi-homeserver, future phase):**
- Add a second tuwunel host to the Shadow YAML with a different `server_name` and `database_path`
- Shadow's network graph can introduce latency between the two hosts to test federation behavior
- Both hosts need unique `server_name` values and cross-host reachable addresses (Shadow assigns IPs per host)

---

## Version Compatibility

| Package | Compatible With | Notes |
|---------|-----------------|-------|
| Shadow 3.3.0 | tokio 1.50 | Shadow fixed EPOLLRDHUP for tokio TCP connections in recent releases. tokio's epoll-based I/O is fully supported when `io_uring` feature is disabled. |
| Shadow 3.3.0 | glibc (x86_64-unknown-linux-gnu) | LD_PRELOAD injection requires dynamic linking. musl targets are incompatible. |
| tuwunel (nightly Rust 1.94.0) | Shadow 3.3.0 | No conflicts observed; standard POSIX networking (bind, listen, accept, read, write) is well-supported by Shadow. |
| reqwest 0.13 | Shadow 3.3.0 | reqwest uses hyper + tokio, both epoll-based. No io_uring in reqwest's default feature set. Compatible. |
| RocksDB (bundled) | tempfile 3.x | Each `TempDir` path must be unique per process — RocksDB does not support two DB instances at the same path in the same process. Multiple distinct paths are fully supported. |

---

## Shadow YAML Config Pattern

The canonical structure for a tuwunel Shadow simulation:

```yaml
general:
  stop_time: 60s
  parallelism: 1
  log_level: info

network:
  graph:
    type: 1_gbit_switch  # simple flat topology; all hosts connected at 1Gbit with 0ms latency

hosts:
  server:
    network_node_id: 0
    processes:
      - path: /path/to/tuwunel/target/release/tuwunel
        args: ""
        start_time: 0s
        shutdown_time: 55s
        shutdown_signal: SIGTERM
        expected_final_state: running
        environment:
          TUWUNEL__SERVER_NAME: "localhost"
          TUWUNEL__DATABASE_PATH: "/tmp/tuwunel-shadow-db"
          TUWUNEL__ADDRESS: "0.0.0.0"
          TUWUNEL__PORT: "8008"
          TUWUNEL__ALLOW_REGISTRATION: "true"
          TUWUNEL__REGISTRATION_TOKEN: ""
          # io_uring must be disabled via compile-time feature flags, not runtime config

  client:
    network_node_id: 0
    processes:
      - path: /path/to/tuwunel/target/release/shadow-test-auth
        args: "--server-url http://server:8008 --scenario register-login-sync"
        start_time: 3s   # wait for server to be ready
        expected_final_state:
          exited: 0      # non-zero exit = test failure
```

Key points:
- `type: 1_gbit_switch` is the simplest topology — all hosts share a single switch node
- Use GML graph for latency/loss injection: `edge [latency "100ms", packet_loss 0.05]`
- `start_time: 3s` for clients gives the server startup time before clients connect
- Server's `expected_final_state: running` (Shadow stops it via SIGTERM at `shutdown_time`)
- Test binary `expected_final_state: {exited: 0}` gives pass/fail semantics

---

## Sources

- [Shadow official documentation](https://shadow.github.io/docs/guide/) — compatibility notes, config spec, design overview (HIGH confidence)
- [Shadow releases page](https://github.com/shadow/shadow/releases) — v3.3.0 confirmed as latest, io_uring status inferred from syscall interception design (MEDIUM confidence)
- [Shadow config spec](https://shadow.github.io/docs/guide/shadow_config_spec.html) — process options, YAML structure (HIGH confidence)
- [axum testing example](https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs) — tower ServiceExt oneshot pattern (HIGH confidence)
- [s2.dev DST post](https://s2.dev/blog/dst) — Turmoil/MadSim alternatives (MEDIUM confidence)
- tuwunel codebase — `src/main/lib.rs`, `src/main/args.rs`, `src/main/tests/smoke.rs` — existing embed API and test patterns (HIGH confidence, direct source inspection)
- Shadow limitations page — statically linked binaries, IPv6, vfork (HIGH confidence)
- WebSearch: io_uring + Shadow — no explicit "not supported" in official docs, but syscall interception architecture makes io_uring incompatible by design (MEDIUM confidence — treat as HIGH risk, verify before shipping Shadow + io_uring)

---

*Stack research for: tuwunel E2E testing with Shadow network simulation*
*Researched: 2026-03-25*
