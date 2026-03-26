# Feature Research

**Domain:** E2E test harness for a Rust Matrix homeserver (Shadow network simulation + embedded library crate)
**Researched:** 2026-03-25
**Confidence:** HIGH (Shadow docs, Complement source, tuwunel CI, complement-crypto; confirmed against official sources)

---

## Feature Landscape

### Table Stakes (Users Expect These)

Features without which the test harness is not usable. Missing these = developers can't write meaningful tests.

| Feature | Why Expected | Complexity | Notes |
|---------|--------------|------------|-------|
| Server lifecycle management | Tests need to start and stop a real tuwunel process (or in-process instance) | LOW | Already partially present in `src/main/tests/smoke.rs` via `Args::default_test` + `Server::new` + `tuwunel::exec` |
| Isolated per-test state | Each test must have its own tempdir and RocksDB instance; shared state produces interference | LOW | tuwunel already supports multiple RocksDB instances per process with separate paths |
| Programmatic configuration | Tests cannot depend on on-disk config files; all settings must be injectable at test time | LOW | figment-based config already supports programmatic override; needs `Args::default_test` coverage for all options |
| Dynamic port binding (port 0) | Tests must not hardcode ports; parallel test runs collide if ports are fixed | LOW | Standard `TcpListener::bind("127.0.0.1:0")` pattern; needs explicit tuwunel support for reporting bound port |
| Matrix Client-Server API coverage | auth (register/login), sync, room create/join/leave, send message — the four operations every Matrix client needs | MEDIUM | Complement already passes these for tuwunel (514/784 pass in `results.jsonl`); Shadow tests should validate the same |
| Deterministic simulation seed | Shadow simulations must be reproducible; same seed = same event order = same bug | LOW | Shadow `general.seed` (default: 1); must be explicit in YAML, never rely on wall clock |
| Stop-time control | Simulated time must have a hard ceiling so tests don't run forever | LOW | Shadow `general.stop_time` (required field); tests must reason about simulated duration |
| Per-host stdout/stderr capture | Tests need to inspect server logs to diagnose failures | LOW | Shadow writes per-process logs to `shadow.data/hosts/<host>/<proc>.<pid>.stdout`; test harness must expose path |
| Basic E2EE test coverage | key upload, one-time key claims, encrypted room messages — the minimum for matrix-rust-client's needs | HIGH | complement-crypto covers this with mitmproxy; Shadow approach replaces Docker but same scenarios apply |
| Test result reporting | Pass/fail with readable error output; must integrate with `cargo test` | LOW | Shadow exits with process exit codes; harness must map these to Rust test pass/fail assertions |
| Shadow YAML generation | Tests must programmatically generate valid Shadow config (not maintain hand-written YAML) | MEDIUM | Shadow config is documented at shadow.github.io; Rust struct -> serde_yaml is the right approach |

### Differentiators (Competitive Advantage over Docker-based Testing)

Features that give this test harness advantages that Complement + Docker + testcontainers cannot provide.

| Feature | Value Proposition | Complexity | Notes |
|---------|-------------------|------------|-------|
| Deterministic time (no wall-clock) | Eliminates the 6-sync / 200ms timing flakes in matrix-rust-client's existing Synapse tests | LOW | Shadow intercepts `clock_gettime`, `gettimeofday`; simulated time advances only when events fire |
| Configurable network conditions (latency, packet loss, bandwidth) | Tests can validate behavior under degraded networks — impossible with Docker networking | MEDIUM | Shadow `network_graph_spec` supports per-edge latency, packet loss, bandwidth; encode scenarios as named fixtures |
| Reproducible failure replay | Any test failure can be re-run identically by using the same seed — no flaky intermittents | LOW | Flows from deterministic seed; test harness should print `seed=N` on failure for reproduction |
| Sub-second server startup (in-process embed) | `tuwunel-embed` starts tuwunel in-process in ~10ms vs 3-10s for Docker Synapse | HIGH | Requires `tuwunel-embed` crate (Phase A); depends on programmatic config and port-0 support |
| No Docker dependency | CI environments without Docker (restricted Linux, Nix, bare metal) can run tests | LOW | Shadow uses LD_PRELOAD interposition; runs any dynamically-linked Linux binary natively |
| PCAP capture per test | Network-level debugging via Wireshark-compatible captures for free | LOW | Shadow `pcap_enabled: true` per host; test harness should preserve PCAP artifacts on failure |
| Multiple topology scenarios as named fixtures | "home office", "high latency federation", "packet loss" as reusable YAML templates | MEDIUM | Shadow YAML merge keys (`<<`) enable this; define topology library, compose in test |
| In-process HTTP transport (future) | Bypass TCP stack entirely for unit-style integration tests using extracted axum Router | HIGH | Requires extracting axum Router from tuwunel-router; deferred to later phase per PROJECT.md |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---------|---------------|-----------------|-------------|
| io_uring support in Shadow builds | io_uring is a tuwunel feature flag that improves I/O performance | Shadow's syscall interposition does not cover io_uring's ring buffer mechanism; enabling it deadlocks or panics under Shadow | Disable with `--no-default-features` or a dedicated `shadow` Cargo feature that excludes `io_uring` |
| IPv6 in Shadow topologies | Some tests want to exercise IPv6 | Shadow has no IPv6 support (documented incompatibility) | Use IPv4-only; this is not a real limitation for protocol testing |
| Federation testing between two tuwunel instances under Shadow | Validates server-server API end to end | Significant complexity: two Shadow hosts, two RocksDB paths, certificate exchange, federation handshake; high risk of flakes before single-server tests are stable | Defer to a later phase; single-server client tests first |
| UI / Wireshark integration test interface | Complement-crypto has an `--allow-ui-introspection` mode | Interactive tooling is orthogonal to CI; adds framework complexity | Provide PCAP artifacts on failure; developers open in Wireshark manually |
| Global test server shared across tests | Tempting for startup speed before `tuwunel-embed` exists | Shared state leaks between tests; DB corruption on test order changes | Per-test tempdir + per-test Shadow simulation; accept the startup cost |
| Mocking the Matrix spec behavior | Mock a "Matrix server" rather than running real tuwunel | Defeats the purpose — we are testing tuwunel's actual protocol correctness | Run real tuwunel under Shadow; use `matrix-sdk` as the real client |
| Network simulation without Shadow (manual tokio time) | `tokio::time::pause()` simulates time at the async level | Does not simulate network topology, bandwidth, or packet loss; cannot reproduce real-world timing bugs | Shadow for E2E scenarios; `tokio::time::pause()` is fine for unit tests only |

---

## Feature Dependencies

```
[Shadow YAML generation]
    └──requires──> [Programmatic configuration]
                       └──requires──> [Dynamic port binding (port 0)]

[Basic E2EE test coverage]
    └──requires──> [Matrix CS API coverage]
                       └──requires──> [Server lifecycle management]
                                          └──requires──> [Isolated per-test state]

[Deterministic failure replay]
    └──requires──> [Deterministic simulation seed]

[Configurable network conditions]
    └──requires──> [Shadow YAML generation]

[Multiple topology fixtures]
    └──requires──> [Shadow YAML generation]
    └──enhances──> [Configurable network conditions]

[In-process HTTP transport]
    └──requires──> [tuwunel-embed crate]
    └──conflicts──> [Shadow YAML generation] (in-process bypasses Shadow entirely)

[Sub-second startup (tuwunel-embed)]
    └──requires──> [Programmatic configuration]
    └──requires──> [Dynamic port binding (port 0)]
    └──conflicts──> [Shadow simulation] (embed runs in same process, no LD_PRELOAD)

[PCAP capture per test]
    └──requires──> [Shadow YAML generation]

[Per-host stdout/stderr capture]
    └──requires──> [Shadow YAML generation]
```

### Dependency Notes

- **Shadow YAML generation requires Programmatic configuration:** Shadow config must specify the tuwunel binary path and all flags; any file-based config breaks the isolation model. Programmatic config generation (Rust structs -> TOML -> tempfile) must exist before Shadow tests can be written.
- **tuwunel-embed conflicts with Shadow simulation:** Shadow's LD_PRELOAD shim only intercepts processes it spawns as child processes. An in-process embedded server runs in the same address space as the test harness, so Shadow cannot intercept its syscalls. The two approaches are complementary, not interchangeable: Shadow for network-aware E2E, embed for fast unit-style integration tests.
- **io_uring conflicts with Shadow:** Shadow intercepts POSIX syscalls via LD_PRELOAD. io_uring uses a kernel ring buffer (`io_uring_setup`, `io_uring_enter`) that Shadow does not emulate. These must be in different Cargo feature sets.

---

## MVP Definition

### Launch With — Phase B (Shadow tests on stock tuwunel binary)

Minimum to prove the concept: stock tuwunel binary runs under Shadow and responds to Matrix CS API requests.

- [ ] Shadow YAML generation (Rust struct -> serde_yaml -> tempfile) — prerequisite for all Shadow tests
- [ ] Server lifecycle management in Shadow (spawn binary, wait for HTTP readiness, assert clean exit) — without this no test can run
- [ ] Dynamic port binding (port 0) support in tuwunel — without this parallel test runs collide
- [ ] Programmatic TOML config generation (no on-disk dependency) — required by Shadow YAML generation
- [ ] Per-host log capture exposed to test assertions — required for diagnosing failures
- [ ] Deterministic seed + stop-time in all test YAML configs — required for reproducibility
- [ ] Basic CS API test: register → login → create room → send message → sync — validates the core path matrix-rust-client uses
- [ ] io_uring disabled in Shadow build profile — prerequisite for Shadow to not deadlock

### Add After Validation — Phase A (tuwunel-embed crate)

Once Shadow tests prove tuwunel works under simulation, build the embed crate for faster iteration.

- [ ] `tuwunel-embed` crate with `EmbeddedHomeserver` struct (start, base_url, shutdown) — requires working programmatic config
- [ ] In-process `reqwest::Client` pointed at embedded server — for test code that doesn't need network simulation
- [ ] E2EE scenarios under Shadow: key upload, one-time key claim, encrypted message, decrypt — the matrix-rust-client pain point
- [ ] Reproducible failure replay: print seed + log path on failure — quality-of-life for CI debugging

### Future Consideration

- [ ] Configurable network topology fixtures ("home office", "high latency", "packet loss") — valuable once basic tests are stable; avoids premature optimization
- [ ] Federation between two tuwunel instances under Shadow — high complexity; single-server tests must be solid first
- [ ] PCAP artifact preservation on test failure — nice debugging aid; implement after YAML generation is stable
- [ ] In-process HTTP transport via extracted axum Router — requires deep router extraction; only needed if embed-crate startup is still too slow

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---------|------------|---------------------|----------|
| Shadow YAML generation | HIGH | LOW | P1 |
| Programmatic TOML config + port 0 | HIGH | LOW | P1 |
| Server lifecycle management (Shadow) | HIGH | LOW | P1 |
| io_uring disabled in Shadow feature | HIGH | LOW | P1 |
| Basic CS API test (auth+sync+rooms+messages) | HIGH | MEDIUM | P1 |
| Per-host log capture | HIGH | LOW | P1 |
| Deterministic seed/stop-time enforcement | HIGH | LOW | P1 |
| tuwunel-embed crate (EmbeddedHomeserver) | HIGH | MEDIUM | P2 |
| E2EE test scenarios (key exchange, encrypted messages) | HIGH | HIGH | P2 |
| Reproducible failure replay (seed printing) | MEDIUM | LOW | P2 |
| Configurable network condition fixtures | MEDIUM | MEDIUM | P2 |
| PCAP artifact preservation | LOW | LOW | P3 |
| Federation multi-instance Shadow tests | MEDIUM | HIGH | P3 |
| In-process axum Router transport | MEDIUM | HIGH | P3 |

**Priority key:**
- P1: Must have for Phase B (Shadow baseline) to be usable
- P2: Required for Phase A (embed crate) or for closing the matrix-rust-client E2EE gap
- P3: Nice to have; defer until P1 and P2 are stable

---

## Competitor Feature Analysis

The "competitors" here are Docker-based test approaches for the same homeserver.

| Feature | Complement + Docker | matrix-rust-client testcontainers (Synapse) | Our Shadow Approach |
|---------|---------------------|---------------------------------------------|---------------------|
| Server startup time | 3-10s (Docker image pull + init) | 3-10s (Synapse image) | ~100ms (binary spawn under Shadow) |
| Time determinism | No — wall clock, inherently flaky | No — 6 syncs × 200ms hardcoded delays | Yes — Shadow intercepts clock syscalls |
| Network simulation | No — real Docker bridge | No — real Docker bridge | Yes — configurable latency, packet loss, bandwidth |
| Failure reproducibility | No — timing-dependent | No — timing-dependent | Yes — deterministic seed |
| Docker dependency | Yes — Docker Engine required | Yes — Docker Engine required | No — Shadow uses LD_PRELOAD only |
| Parallel test isolation | Fragile — port collision common | Fragile — port collision common | Strong — each Shadow sim has its own network namespace |
| Matrix protocol coverage | ~784 Complement tests (514 pass for tuwunel) | Subset of matrix-sdk integration tests | Starts with CS API basics; grows incrementally |
| E2EE test support | Yes — complement-crypto with mitmproxy | Partial — basic encrypted rooms | Planned Phase A (E2EE scenarios) |
| In-process embed | No — always out-of-process Docker | No — always out-of-process Docker | Yes — `tuwunel-embed` (Phase A) |
| CI without Docker | No | No | Yes |

---

## Sources

- [Complement README](https://github.com/matrix-org/complement/blob/main/README.md) — test categories and Docker-based architecture (MEDIUM confidence: GitHub scrape)
- [complement-crypto README](https://github.com/matrix-org/complement-crypto) — E2EE test scenarios, mitmproxy architecture, CI usage (MEDIUM confidence: GitHub scrape)
- [Shadow Config Specification](https://shadow.github.io/docs/guide/shadow_config_spec.html) — process config, network config, PCAP, seed options (HIGH confidence: official docs)
- [Shadow Compatibility Notes](https://shadow.github.io/docs/guide/compatibility_notes.html) — io_uring, IPv6, sendfile unsupported (HIGH confidence: official docs)
- [tuwunel `.github/workflows/test.yml`](../../.github/workflows/test.yml) — CI pipeline showing unit, integ, smoke, rust-sdk-integ, complement jobs (HIGH confidence: primary source)
- [tuwunel `tests/complement/results.jsonl`](../../tests/complement/results.jsonl) — 784 test results, 514 pass, 255 fail (HIGH confidence: primary source)
- [tuwunel `src/main/tests/smoke.rs`](../../src/main/tests/smoke.rs) — existing in-process lifecycle test pattern (HIGH confidence: primary source)
- [SyTest GitHub](https://github.com/matrix-org/sytest) — legacy Perl test runner, now deprecated in favor of Complement (MEDIUM confidence: WebSearch)
- PROJECT.md and TESTING.md — project constraints, existing test infrastructure (HIGH confidence: primary sources)

---

*Feature research for: E2E test harness (Shadow + tuwunel-embed) for Matrix homeserver*
*Researched: 2026-03-25*
