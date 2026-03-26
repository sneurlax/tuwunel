# Pitfalls Research

**Domain:** Embedded Rust homeserver E2E testing with Shadow network simulation
**Researched:** 2026-03-25
**Confidence:** HIGH (grounded in actual codebase, Shadow official docs, and verified Rust/Tokio patterns)

---

## Critical Pitfalls

### Pitfall 1: io_uring Enabled in Shadow Builds

**What goes wrong:**
The tuwunel binary is built with `io_uring` in its default feature set (confirmed in `src/main/Cargo.toml`). Shadow does not support io_uring syscalls and cannot implement them — the Shadow documentation explicitly states this is a hard limitation requiring significant effort to change. Running a default tuwunel binary under Shadow will cause io_uring syscalls to return `ENOSYS`, producing cryptic failures or a non-functional process instead of a clean error.

**Why it happens:**
`io_uring` is in the `[features] default` list. Anyone who does `cargo build` without explicitly disabling defaults gets a binary that Shadow cannot run. The failure mode is not an immediate crash on startup — it may only manifest when I/O paths that use io_uring are exercised, making it look like a test logic problem rather than a build problem.

**How to avoid:**
Build a dedicated Shadow target profile that explicitly disables `io_uring`:
```toml
# In Cargo.toml or a shadow-specific profile
[profile.shadow]
# ...
```
Or build with `--no-default-features` and re-enable only Shadow-compatible features. Document the required build flags in the Shadow test harness Makefile/script so CI cannot accidentally run a wrong binary. Confirm at Shadow test startup by checking that the binary was built without io_uring (embed the feature list in a build-time constant).

**Warning signs:**
- Tuwunel process exits quickly in Shadow with no HTTP responses
- Shadow log shows `ENOSYS` from the managed process
- io_uring-related errors in tuwunel's stderr within Shadow output directory

**Phase to address:**
Phase B (Shadow test infrastructure) — must be the very first gate before any test is written.

---

### Pitfall 2: `runtime::new()` Global OnceLock Conflicts Between Embedded Instances

**What goes wrong:**
`src/main/runtime.rs` uses process-global `OnceLock` statics (`WORKER_AFFINITY`, `GC_ON_PARK`, `GC_MUZZY`) and `AtomicUsize` counters (`CORES_OCCUPIED`, `THREAD_SPAWNS`). Calling `runtime::new()` a second time in the same process (e.g., in consecutive tests or when instantiating two `EmbeddedHomeserver` instances) will `panic!` on `OnceLock::set()` because the locks are already initialized. The `Args` struct also calls `Args::parse()` from the environment in its `Default` impl, which reads `std::env::args()` — during tests this returns the test harness argv, not tuwunel's.

**Why it happens:**
The runtime module was designed for a single-process, single-lifetime binary. These are reasonable design choices for a server but unsafe to reuse in an embedding context without modification. The `OnceLock::set().expect(...)` pattern treats second-initialization as an unrecoverable bug rather than a supported mode.

**How to avoid:**
The `tuwunel-embed` crate must not call `runtime::new()` more than once per process. Enforce this with a process-global `OnceLock<Runtime>` in the embed crate itself. Alternatively, refactor the OnceLocks out of the runtime module into per-instance state as part of the embed crate work. For tests that need multiple concurrent instances, use separate OS processes (which Shadow already provides) rather than in-process embedding.

**Warning signs:**
- `panicked at 'set WORKER_AFFINITY from program argument'` in test output
- Second `EmbeddedHomeserver::new()` call panics immediately
- Tests pass in isolation but fail when run together with `cargo test`

**Phase to address:**
Phase A (tuwunel-embed crate) — design the embed API around single-runtime constraint from the start.

---

### Pitfall 3: Tracing Subscriber Registered Multiple Times

**What goes wrong:**
`logging::init()` calls `tracing::subscriber::set_global_default()` (or equivalent via `tracing_subscriber::Registry`). In a process that starts more than one `Server` (multiple test runs, or two embedded instances), the second call returns an error because the global subscriber is already set. If this error is ignored or panicked on, subsequent server instances produce no logs and structured test assertions that depend on log output fail silently.

**Why it happens:**
`tracing` enforces a single global subscriber per process. The server's logging module was designed for a single-lifecycle binary and does not account for a scenario where it is initialized more than once in a process.

**How to avoid:**
In the embed crate, initialize logging at most once using a `OnceLock<()>` guard. Prefer a no-op subscriber (`NoSubscriber`) for subsequent instances, or accept the `AlreadyInitialized` error silently. For Shadow tests (separate processes), this is not an issue. For the in-process embed crate, expose a `with_logging(bool)` option on the builder.

**Warning signs:**
- `Error: a global default trace dispatcher has already been set` in stderr
- Second embedded homeserver starts but produces no tracing output
- Logs from two concurrent instances interleave under the first instance's identity

**Phase to address:**
Phase A (tuwunel-embed crate).

---

### Pitfall 4: Shadow Busy-Loop Deadlock from Tokio's Thread Parking

**What goes wrong:**
Shadow assumes CPUs are infinitely fast and only advances simulated time during blocking syscalls (`nanosleep`, `epoll_wait`, `futex`, etc.). Tokio's multi-thread runtime uses `park()`/`unpark()` via futex for its work-stealing idle logic, which Shadow does interpret as a blocking syscall. However, if a tokio worker thread enters a spin-wait loop (e.g., checking an `AtomicBool` without yielding), Shadow will not advance time and the simulation deadlocks permanently with no diagnostic output.

tuwunel uses a highly-tuned tokio configuration (`global_queue_interval=192`, `event_interval=512`, `max_io_events_per_tick=512`) that maximizes throughput at the cost of more aggressive spin behavior in lightly-loaded conditions — which is exactly the condition during Shadow tests with few simulated clients.

**Why it happens:**
The tokio configuration is tuned for production workloads. Shadow's discrete-event model requires that threads yield to the simulator frequently via blocking syscalls. Production tokio tuning and Shadow compatibility point in opposite directions.

**How to avoid:**
Create a Shadow-specific tokio configuration with lower queue intervals and shorter keep-alive timers to force more frequent syscall-based parking. Use `--model-unblocked-syscall-latency` in the Shadow YAML config as a safety net. Set `worker_threads = 1` or `2` for Shadow builds to reduce the spin-window surface area. Avoid `worker_affinity = true` under Shadow (CPU pinning via `sched_setaffinity` has low value in a simulated environment and adds syscall overhead).

**Warning signs:**
- Shadow simulation progress stalls at a fixed simulated time with no output
- Shadow reports `[shadow] maximum simulation time reached` prematurely
- Shadow log shows one process consuming 100% of host CPU without advancing

**Phase to address:**
Phase B (Shadow infrastructure) — must be validated with a minimal tuwunel health-check test before writing real tests.

---

### Pitfall 5: RocksDB Lock File Contention Between Test Instances

**What goes wrong:**
RocksDB acquires a file lock (`LOCK`) on the database directory when opened. If two test instances share a database path — even briefly, due to test parallelism or a previous test not cleaning up — the second open fails with `IO error: lock /path/LOCK: Resource temporarily unavailable`. Because tuwunel defaults `database_path` to `/var/lib/tuwunel`, any test that does not override this path will either fail to start or corrupt a shared database.

**Why it happens:**
The `Config` struct default for `database_path` is `/var/lib/tuwunel` (confirmed in `src/core/config/mod.rs:3229`). `Args::default_test()` only sets `server_name = "localhost"` — it does not set a unique `database_path`. If the embed crate or test harness does not explicitly provide a tempdir path, every test instance fights over the same global lock file.

**How to avoid:**
The embed crate API must require a database path or auto-provision one via `tempfile::TempDir`. The `EmbeddedHomeserver` struct should own a `TempDir` instance so cleanup is automatic on drop. For Shadow tests, generate a unique path per test run using the test name or a UUID as a subdirectory. Assert in CI that no test hardcodes `/var/lib/tuwunel`.

**Warning signs:**
- `IO error: lock .../LOCK: Resource temporarily unavailable` in test stderr
- Tests pass serially (`cargo test -- --test-threads=1`) but fail in parallel
- Database directory from previous test run present without cleanup

**Phase to address:**
Phase B (Shadow infrastructure) for the binary test path; Phase A (embed crate) for the in-process path.

---

### Pitfall 6: `server_name` Mismatch Causes Silent Database Corruption

**What goes wrong:**
If a test starts tuwunel with `server_name = "localhost"` and a tempdir, then a second test reuses the same tempdir (e.g., due to cleanup failure), tuwunel will start successfully but operate on data associated with the old server name. Matrix IDs embedded in events (user IDs, event IDs, room IDs) will be inconsistent. The server does not abort on server_name mismatch — it proceeds silently. This is documented as a known risk in CONCERNS.md.

**Why it happens:**
Test cleanup is not guaranteed when a test panics, tokio times out, or Shadow kills processes abnormally. `TempDir` on-drop cleanup can fail if RocksDB holds open file handles at test teardown. Tests that use a fixed path (not random) are especially vulnerable.

**How to avoid:**
Always use a random/unique path per test run. Never reuse a database directory across test runs. If RocksDB open fails on a tempdir, delete and recreate rather than retrying. In the embed crate, add a startup check that reads stored `server_name` from RocksDB and asserts it matches config (the recommendation in CONCERNS.md).

**Warning signs:**
- Tests produce unexpected Matrix IDs from a different domain than configured
- RocksDB open succeeds but operations fail with "not found" for known keys
- Test database directory is not fresh (non-zero pre-existing RocksDB files)

**Phase to address:**
Phase A and Phase B — embed crate startup validation and Shadow test harness cleanup.

---

## Technical Debt Patterns

| Shortcut | Immediate Benefit | Long-term Cost | When Acceptable |
|----------|-------------------|----------------|-----------------|
| Using `Args::default()` which calls `Args::parse()` in tests | No extra boilerplate | Reads test-harness `argv`; `--test` and `--bench` args leak into server config | Never — always use `Args::default_test()` |
| Hardcoding `server_name = "localhost"` for all tests | Simple setup | Tests share the same Matrix domain; user IDs collide across parallel tests | Only if each test gets a unique tempdir |
| Skipping `io_uring` disable in Shadow builds | One less build variant to maintain | Shadow silently fails I/O operations; tests appear to run but produce no data | Never for Shadow targets |
| Not setting `database_path` in embed crate config | Fewer required fields | Falls back to `/var/lib/tuwunel`; fails or corrupts on developer machines | Never — always require path |
| `runtime::shutdown_timeout` of 10s in tests | Same behavior as production | Shadow's simulated time is not wall-clock; 10s simulated time may never elapse under Shadow | Replace with a Shadow-appropriate shutdown sequence |
| Using the `release_max_log_level` feature in test builds | Faster binary | Debug and trace logs suppressed; test failures produce minimal diagnostic output | Acceptable for performance tests; disable for debug tests |

---

## Integration Gotchas

| Integration | Common Mistake | Correct Approach |
|-------------|----------------|------------------|
| Shadow YAML config | Using shell-style path expansion (`~/`) in `path:` fields — Shadow does NOT expand these in process arguments (only in `path:` top-level fields) | Use absolute paths constructed by the test harness script; no `~/` in process `args:` |
| Shadow YAML config | YAML arithmetic expressions like `"10 - 1"` for time values evaluate as invalid strings, not numbers | Compute time values in the generating script (Python/shell) before writing YAML |
| Shadow YAML config | Assigning the same IP address to two hosts causes a startup error with a confusing message | Use a deterministic IP allocation scheme (e.g., `11.0.0.1` for server, `11.0.0.2` for client) |
| figment config in tests | Calling `Config::load(empty_iterator)` causes figment to look for a default config file in the working directory | Always pass at least one `Figment::new()` with programmatic providers; use `config.merge(("server_name", "..."))` pattern |
| RocksDB in tests | Relying on OS to clean up the tempdir when a test panics | Use `TempDir` with explicit `DB::destroy()` before drop; add a `#[test]` teardown that calls destroy even on panic |
| tracing in tests | Calling `set_global_default` in every test | Initialize once with `std::sync::Once`; use `NoSubscriber` or redirect to test output with `tracing_subscriber::fmt::init()` called once |
| Shadow + TLS | Running tuwunel with TLS in Shadow adds certificate validation complexity and is not needed for test scenarios | Use `--no-tls` / unencrypted config for Shadow tests; TLS termination is irrelevant for correctness testing |

---

## Performance Traps

| Trap | Symptoms | Prevention | When It Breaks |
|------|----------|------------|----------------|
| Tokio runtime configured for high throughput (queue intervals 192/512) | Shadow simulation freezes or runs extremely slowly under low-load test conditions | Use a Shadow-specific tokio args profile with smaller intervals (32/64) and fewer worker threads | Any Shadow test with fewer than ~10 concurrent clients |
| jemalloc with per-thread arenas under Shadow | jemalloc's background decay thread makes syscalls that advance Shadow time unpredictably | Disable jemalloc background threads for Shadow builds via `--features jemalloc --no-default-features` or env config | Any simulation with >2 worker threads |
| RocksDB compaction during test | Test assertions race with background compaction; database read latency spikes mid-test | Disable background compaction threads in test config (`rocksdb_parallelism = 1`, disable background jobs) | Tests with large datasets or long-running scenarios |
| Shadow `--model-unblocked-syscall-latency` | Simulation runs correctly but 10x slower than expected | Only enable as a fallback; prefer fixing busy loops at the source | Large test suites run in CI with tight time budgets |
| State compressor loading full state sets | Tests with rooms containing many members consume large amounts of Shadow-managed memory | Keep test rooms small (<10 members); avoid federation fan-out in initial tests | Tests simulating rooms with >100 members |

---

## Security Mistakes

These are relevant in the test harness context — misconfigured test servers that leak into non-test environments.

| Mistake | Risk | Prevention |
|---------|------|------------|
| Setting `allow_invalid_tls_certificates = true` in Shadow config | Test config bleeds into non-test builds; outgoing federation validates nothing | Always scope this config to Shadow/test builds explicitly; never commit to a shared config file |
| Using a predictable `server_name` like `"localhost"` in integration tests run against a real network | Matrix events from the test server get federated or discovered | Shadow tests are fully network-isolated; no risk there. For embed crate tests, use a non-routable domain like `"test.invalid"` |
| Leaving `jwt.enable = true` without a key (disables signature validation) | Any JWT is accepted as valid; auth tests produce false positives | Explicitly set `jwt.enable = false` in all test configs unless testing JWT specifically |
| Not clearing JWT/signing keys between embedded test instances | Second server instance re-uses keys from the first | Use a fresh config per test instance; keys are derived from config, so unique `server_name` + tempdir ensures uniqueness |

---

## "Looks Done But Isn't" Checklist

- [ ] **Shadow binary build:** Binary compiled without `io_uring` feature — verify with `cargo metadata` or a build-time assertion, not by assuming `--no-default-features` was passed
- [ ] **Shadow process start:** Tuwunel process actually listening before test client connects — verify by polling the health endpoint (`GET /_matrix/client/v3/versions`) rather than sleeping a fixed interval
- [ ] **Shadow test determinism:** Running the same test twice produces the same Shadow log and same assertion outcomes — verify by running twice and diffing output before declaring a test "stable"
- [ ] **Embed crate cleanup:** `EmbeddedHomeserver::stop()` completes RocksDB flush and releases the file lock before returning — verify with a test that starts a second instance immediately after stopping the first
- [ ] **Database path isolation:** Every test uses a unique path — verify with `cargo test -- --test-threads=8` and confirm no lock contention
- [ ] **Graceful shutdown under Shadow:** Tokio's 10s `shutdown_timeout` does not block Shadow completion (Shadow's simulated time ≠ wall-clock time) — verify by inspecting Shadow's final simulated timestamp against the `general.stop_time`
- [ ] **Tracing subscriber initialization:** Multiple tests in the same `cargo test` binary each call `Server::new()` without panicking on subscriber re-registration — verify by running the full test suite without `--test-threads=1`
- [ ] **Config path handling:** No `~/` expansion in Shadow YAML process `args:` fields — verify by grepping generated YAML files for `~`
- [ ] **E2EE test completeness:** Key exchange round-trip verified (not just that the endpoint returned 200) — verify by actually decrypting a message, not just checking HTTP status

---

## Recovery Strategies

| Pitfall | Recovery Cost | Recovery Steps |
|---------|---------------|----------------|
| io_uring enabled in Shadow build | LOW | Rebuild with `--no-default-features --features <shadow-set>`; re-run Shadow tests |
| OnceLock panic in embed tests | MEDIUM | Identify all process-global statics in `src/main/runtime.rs` and `src/main/sentry.rs`; refactor into per-instance state or add single-init guards in embed crate |
| Tracing subscriber double-init | LOW | Wrap logging init in `Once::call_once`; accept `AlreadyInitialized` error silently |
| Shadow simulation deadlock (busy loop) | MEDIUM | Add `--model-unblocked-syscall-latency: true` to Shadow config; then diagnose root-cause thread; reduce tokio queue intervals for Shadow profile |
| RocksDB lock contention between tests | LOW | Delete stale tempdir; implement `TempDir` + `DB::destroy()` pattern in harness |
| server_name database mismatch | HIGH | Delete database directory entirely; cannot recover data — this is why tempdir per test is mandatory |
| Shadow path whitespace failure | LOW | Rename workspace directory to remove spaces; update all references |

---

## Pitfall-to-Phase Mapping

| Pitfall | Prevention Phase | Verification |
|---------|------------------|--------------|
| io_uring enabled in Shadow builds | Phase B: Shadow infrastructure | CI build step explicitly lists features; Shadow smoke test completes without `ENOSYS` |
| `runtime::new()` OnceLock conflicts | Phase A: tuwunel-embed crate | Two consecutive `EmbeddedHomeserver` test instances start and stop without panic |
| Tracing subscriber double-registration | Phase A: tuwunel-embed crate | `cargo test` with multiple test functions calling `Server::new()` passes without subscriber error |
| Shadow busy-loop deadlock | Phase B: Shadow infrastructure | Shadow health-check test advances simulated time past T=5s without freezing |
| RocksDB lock contention | Phase B (binary) / Phase A (embed) | `cargo test -- --test-threads=8` produces no `LOCK: Resource temporarily unavailable` |
| server_name/database mismatch | Phase A + Phase B | Startup assertion added; test that reuses a stale path triggers the assertion and fails fast |
| Shadow YAML path/arithmetic gotchas | Phase B: Shadow infrastructure | CI generates YAML and validates it with `shadow --validate` before running |
| Tokio tuning vs. Shadow determinism | Phase B: Shadow infrastructure | Shadow simulation completes in <5 minutes wall-clock for a basic auth+sync test |
| jemalloc background threads under Shadow | Phase B: Shadow infrastructure | Shadow test binary built with jemalloc background threads disabled; verified in build script |

---

## Sources

- Shadow official limitations documentation: https://shadow.github.io/docs/guide/limitations.html (HIGH confidence — official docs)
- Shadow config specification: https://shadow.github.io/docs/guide/shadow_config_spec.html (HIGH confidence — official docs)
- Shadow design overview: https://shadow.github.io/docs/guide/design_2x.html (HIGH confidence — official docs)
- Shadow GitHub issue #1839 re: statically linked binaries and LD_PRELOAD: https://github.com/shadow/shadow/issues/1839 (HIGH confidence)
- Shadow GitHub discussion #1675 re: CPU usage, simulation time, and busy loops: https://github.com/shadow/shadow/discussions/1675 (HIGH confidence)
- Tokio runtime "cannot start a runtime from within a runtime" discussion: https://github.com/tokio-rs/tokio/discussions/3857 (HIGH confidence — official repo)
- tuwunel codebase: `src/main/runtime.rs`, `src/main/server.rs`, `src/main/args.rs`, `src/main/logging.rs` — OnceLock statics, runtime construction, Args::parse() in Default (HIGH confidence — direct code inspection)
- tuwunel `src/main/Cargo.toml` — `io_uring` in default features (HIGH confidence — direct code inspection)
- tuwunel `src/core/config/mod.rs:3229` — default `database_path = "/var/lib/tuwunel"` (HIGH confidence — direct code inspection)
- `.planning/codebase/CONCERNS.md` — server_name mismatch, unsafe State raw pointer, config manager transmute, RocksDB single instance (HIGH confidence — direct analysis)
- RocksDB multiple instances discussion: https://github.com/facebook/rocksdb/issues/942 (MEDIUM confidence — community)

---

*Pitfalls research for: embedded Rust homeserver E2E testing with Shadow network simulation*
*Researched: 2026-03-25*
