# Project Research Summary

**Project:** tuwunel E2E test harness (Shadow network simulation + embedded library crate)
**Domain:** Rust Matrix homeserver integration testing with deterministic network simulation
**Researched:** 2026-03-25
**Confidence:** HIGH

## Executive Summary

The tuwunel project needs a two-phase test harness that replaces Docker-based integration testing with a deterministic, network-simulation-aware approach. Phase B (Shadow binary tests) runs the stock tuwunel binary unchanged under Shadow 3.3.0's syscall interception layer, providing deterministic time, configurable network conditions, and reproducible failures — none of which the existing Complement + Docker approach offers. Phase A (tuwunel-embed crate) layers on top of that foundation to provide sub-100ms in-process server startup for faster test iteration in both the tuwunel repo and downstream in matrix-rust-client. The two phases are complementary: Shadow for network-aware E2E correctness, embed for fast unit-style integration tests.

The critical architectural constraint that shapes everything is Shadow's `io_uring` incompatibility. The tuwunel binary enables `io_uring` by default, but Shadow intercepts syscalls at the LD_PRELOAD level and cannot handle `io_uring`'s ring-buffer I/O. This is not a soft limitation — it causes silent I/O failures and non-functional processes. Every build pipeline, CI gate, and Cargo feature set must be designed around this constraint from day one. A dedicated `shadow` build profile that disables `io_uring` while re-enabling all other features is mandatory before any Shadow test can be written.

The main implementation risks are concentrated in Phase A. The existing `src/main/runtime.rs` uses process-global `OnceLock` statics that panic on second initialization — embedding multiple server instances per process requires careful design around this constraint. The tracing subscriber has the same single-initialization limitation. These are known, bounded problems with clear mitigations (single-runtime guard in the embed crate, `Once`-protected logging init), but they must be designed in from the start, not retrofitted.

## Key Findings

### Recommended Stack

Shadow 3.3.0 is already installed at `~/.local/bin/shadow` and is the correct tool for this project. It runs real unmodified Linux binaries under deterministic discrete-event simulation with no code changes to tuwunel required — unlike Turmoil or MadSim which require owning all code, or Docker which provides no network simulation and suffers from 3-10s startup costs. The workspace already contains all other dependencies needed (tokio 1.50, reqwest 0.13, figment 0.10, serde_json 1.0, insta 1.43, clap 4.5); only `tempfile` needs to be added to the workspace for the embed crate.

**Core technologies:**
- Shadow 3.3.0: Network simulation orchestrator — only tool that runs real Linux binaries under deterministic time with configurable latency/loss/bandwidth
- tuwunel stock binary (glibc target, no `io_uring`): Server under test — Shadow requires dynamically-linked binaries; `io_uring` must be disabled at build time
- Rust `[[bin]]` test clients in new `tuwunel-shadow-tests` crate: Test scenario executors — Shadow needs binaries with `main()`, not `#[test]` harness binaries
- reqwest 0.13 (existing workspace dep): HTTP client inside Shadow simulations — already present, epoll-based, Shadow-compatible
- figment 0.10 (existing workspace dep): Programmatic config construction for embed crate — no on-disk TOML required in test contexts
- tempfile 3.x (add to workspace): Per-test RocksDB isolation for embed crate — mandatory; default database path `/var/lib/tuwunel` causes lock contention

### Expected Features

**Must have (Phase B — Shadow baseline):**
- Shadow YAML generation (Rust structs to serde_yaml to tempfile) — prerequisite for all Shadow tests
- io_uring disabled in dedicated Shadow build profile — without this, Shadow tests silently fail I/O operations
- Server lifecycle management under Shadow (spawn, HTTP readiness poll, assert clean exit) — without this no test can run
- Dynamic port binding (port 0) with bound-port discovery — without this parallel runs collide
- Programmatic TOML config generation without on-disk file dependency — required by Shadow YAML generation
- Per-host stdout/stderr log capture for test assertions — required for diagnosing failures
- Deterministic seed + explicit stop-time in all Shadow YAML configs — required for reproducibility
- Basic CS API test: register → login → create room → send message → sync — validates the core path matrix-rust-client uses

**Should have (Phase A — embed crate):**
- `tuwunel-embed` crate with `EmbeddedHomeserver` struct (start, base_url, shutdown) — sub-100ms startup vs 3-10s Docker
- E2EE scenarios under Shadow: key upload, one-time key claim, encrypted message, decrypt — the matrix-rust-client pain point
- Configurable network topology fixtures ("home office", "high latency", "packet loss") — named YAML templates
- Reproducible failure replay: print seed + log path on failure — quality-of-life for CI debugging
- PCAP artifact preservation on test failure — Shadow provides this for free; just expose the path

**Defer (v2+):**
- Federation between two tuwunel instances under Shadow — high complexity; single-server tests must be solid first
- In-process axum Router HTTP transport (bypass TCP stack entirely) — requires deep router extraction; only needed if embed startup is still too slow
- UI/Wireshark integration interface — orthogonal to CI; provide PCAP artifacts instead

### Architecture Approach

The test infrastructure follows two parallel patterns that never intersect. Shadow tests wire the stock tuwunel binary and a purpose-built `matrix-test-client` binary into a simulated network via YAML topology files; results are read from `shadow.data/hosts/*/stdout` by post-run assertion scripts. The embed crate wraps the existing `src/main/lib.rs` lifecycle (`Server::new`, `exec`, `async_start`/`async_stop`) behind a RAII `EmbeddedHomeserver` struct that owns its own `TempDir` and tokio runtime. All new infrastructure lives under `tests/shadow/` (Shadow scenarios, client binary, assertion scripts) and `src/embed/` (embed crate) — no changes to existing tuwunel server code are required for Phase B.

**Major components:**
1. `tests/shadow/` — Shadow simulation harness: YAML topology definitions, client binary source, assertion scripts, and a top-level runner script
2. `tests/shadow/src/bin/matrix-test-client.rs` — Standalone Rust binary compiled as a workspace `[[bin]]`; uses reqwest to exercise the Matrix CS API; exits 0 on success, non-zero on failure
3. `tests/shadow/scenarios/*.yaml` — Per-scenario Shadow configs (smoke, messaging, e2ee, latency); Shadow's `expected_final_state: {exited: 0}` on the client binary gives pass/fail semantics
4. `src/embed/` — `tuwunel-embed` crate: `EmbeddedHomeserver` RAII struct; depends on same internal crates as `src/main/`; adds port-0 support to `src/router/serve.rs`
5. tuwunel binary (no changes for Phase B) — configured via environment variables or TOML in Shadow context; configured programmatically via figment in embed context

### Critical Pitfalls

1. **io_uring enabled in Shadow builds** — Shadow cannot intercept io_uring ring-buffer I/O; tuwunel silently fails I/O operations rather than crashing cleanly. Mitigation: build a dedicated Shadow feature profile with `--no-default-features` plus all features except `io_uring`; verify at CI time with a build-time constant asserting the feature is absent.

2. **`runtime::new()` OnceLock conflicts in embed crate** — `src/main/runtime.rs` uses process-global `OnceLock` statics that panic on second call; `Args::Default` reads test harness argv. Mitigation: the embed crate must enforce single-runtime-per-process with its own `OnceLock<Runtime>` guard; never call `Args::default()` in test context, only `Args::default_test()`.

3. **Tracing subscriber double-registration** — `logging::init()` calls `set_global_default()` which panics or silently drops logs on second call. Mitigation: wrap logging initialization in `std::sync::Once` in the embed crate; silently accept `AlreadyInitialized`.

4. **Shadow busy-loop deadlock from tokio's high-throughput configuration** — tuwunel's production tokio tuning (`global_queue_interval=192`, `event_interval=512`) causes spin-waiting under low-load Shadow conditions where simulated time stalls. Mitigation: Shadow-specific tokio args profile with lower intervals (32/64) and `worker_threads = 1` or `2`; validate with a minimal health-check test that advances simulated time past T=5s before writing real tests.

5. **RocksDB lock file contention between test instances** — default `database_path = "/var/lib/tuwunel"` causes all test instances to fight over the same lock file. Mitigation: `EmbeddedHomeserver` must own a `TempDir`; Shadow tests must generate a unique path per run; CI must assert no test hardcodes the default path.

6. **server_name/database mismatch causes silent data corruption** — if a test reuses a stale tempdir (due to cleanup failure or panic), tuwunel starts on data from a previous server_name. Mitigation: always use random paths; add a startup assertion that reads stored server_name from RocksDB and compares against config.

## Implications for Roadmap

### Phase 1: Shadow Infrastructure Foundation

**Rationale:** Zero code changes to tuwunel; establishes the build pipeline and proves Shadow can run tuwunel at all. This is the gating step — if Shadow and tuwunel are incompatible for any reason (tokio tuning, jemalloc background threads, unexpected syscalls), discovery happens here before any test logic is invested. If Phase B works, Phase A has a proven behavioral baseline to build on.

**Delivers:** A Shadow-compatible tuwunel binary build profile; a minimal smoke scenario (health check + CS API versions endpoint); validated Shadow YAML generation infrastructure.

**Addresses:** Shadow YAML generation, io_uring build profile, deterministic seed/stop-time, per-host log capture, Shadow YAML path/arithmetic gotchas.

**Avoids:** io_uring pitfall (Pitfall 1), Shadow busy-loop deadlock (Pitfall 4), jemalloc background thread interference, path whitespace issues.

**Research flag:** Standard patterns — Shadow docs are comprehensive and the smoke scenario is well-trodden ground. No deeper research phase needed.

### Phase 2: Core CS API Test Scenarios

**Rationale:** With Shadow infrastructure proven, add the test scenarios that validate the behavior matrix-rust-client depends on. The ordering within this phase matters: smoke (health check) → auth (register+login) → messaging (create room, send, sync) → E2EE (key exchange, encrypted messages). Each step builds on the previous.

**Delivers:** A comprehensive Shadow test suite covering auth, room management, messaging, and E2EE key exchange under simulated network conditions.

**Addresses:** Basic CS API test (register/login/create room/send/sync), E2EE scenarios (key upload, one-time key claim, encrypted message), per-host log capture for assertions, deterministic failure replay.

**Avoids:** Timing-based assertions (anti-pattern: use polling retry loops, not wall-clock sleeps), global shared server state (each scenario gets its own simulation), io_uring in build.

**Research flag:** E2EE scenario design may benefit from reviewing complement-crypto test cases for key exchange round-trip verification patterns. The HTTP polling retry pattern inside Shadow also needs one spike to confirm simulated-time behavior before writing multiple scenarios.

### Phase 3: tuwunel-embed Crate

**Rationale:** Once Shadow tests provide a behavioral baseline proving tuwunel correctness, build the embed crate for fast iteration. Phase A code changes can only regress what Phase B tests already cover. The embed crate has bounded, known risks (OnceLock, tracing subscriber) that are straightforward to solve in isolation.

**Delivers:** `tuwunel-embed` crate with `EmbeddedHomeserver` RAII API; port-0 support in `src/router/serve.rs`; in-process integration tests runnable without Shadow or Docker.

**Addresses:** tuwunel-embed crate (EmbeddedHomeserver), programmatic config construction, dynamic port binding (port 0), sub-second server startup, no Docker dependency in CI.

**Avoids:** OnceLock conflict (Pitfall 2) via single-runtime guard; tracing double-registration (Pitfall 3) via Once guard; RocksDB lock contention (Pitfall 5) via owned TempDir; server_name mismatch (Pitfall 6) via startup assertion.

**Research flag:** The specific refactoring needed in `src/main/runtime.rs` to support per-instance or single-instance-guarded OnceLocks should be spiked as the first task of this phase — it determines whether the embed API is clean or requires process-level constraints exposed to callers.

### Phase 4: Network Condition Fixtures and Advanced Scenarios

**Rationale:** Once both Shadow and embed infrastructure are stable, add the differentiating features that make this test harness distinctly better than Complement + Docker. Network condition fixtures and PCAP capture are Shadow-native capabilities that require only YAML authoring once the harness exists.

**Delivers:** Named topology fixtures (home-office, high-latency, packet-loss); PCAP artifact preservation on failure; federation multi-server Shadow tests (if Phase 2 is solid).

**Addresses:** Configurable network topology fixtures, PCAP artifact preservation, reproducible failure replay with seed printing.

**Avoids:** Premature federation testing before single-server correctness is confirmed.

**Research flag:** Federation Shadow tests (two tuwunel instances with certificate exchange) are genuinely novel; no prior art exists for this specific combination. If federation tests are in scope, this phase needs a research spike on Matrix federation handshake behavior under simulated network conditions.

### Phase Ordering Rationale

- Phase B (Shadow, Phases 1-2) before Phase A (embed, Phase 3): Phase B requires zero tuwunel code changes; any test failure in Phase B is in the harness, not tuwunel. Phase A adds code; regressions in Phase A can be attributed to those additions. This is the cleanest debugging model.
- Within Phase B, smoke before messaging before E2EE: each layer adds protocol complexity; a failure at a lower layer would mask failures at higher layers.
- Network fixtures after CS API tests: adding latency/loss to broken tests obscures root causes; apply impairment only to scenarios proven to pass under ideal conditions.
- Embed crate after Shadow baseline: the embed crate is intended to run the same scenarios faster; those scenarios must exist and be validated before the embed crate has meaningful test coverage.

### Research Flags

Phases needing deeper research during planning:
- **Phase 3 (embed crate):** The OnceLock refactoring in `src/main/runtime.rs` needs a spike before the API can be designed. The exact surface to expose from `src/main/lib.rs` (whether to use `exec`, `async_start`/`async_run`/`async_stop` separately, or wrap at a higher level) determines the embed crate's complexity.
- **Phase 4 (federation):** If in scope, federation Shadow tests have no existing prior art for this specific toolchain combination. Needs a dedicated research spike on Matrix federation handshake + certificate requirements under simulated networking.

Phases with standard patterns (skip research-phase):
- **Phase 1 (Shadow infrastructure):** Shadow docs are authoritative and comprehensive; the smoke scenario pattern is well-documented. Build the feature, don't research it.
- **Phase 2 (CS API tests):** Matrix CS API is fully specified; the test scenarios follow Complement's established patterns. Complement's `results.jsonl` (514/784 passing) provides a known baseline for what should pass.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Shadow 3.3.0 is installed and version-verified; all other dependencies exist in the workspace; io_uring incompatibility is confirmed by Shadow's architecture documentation |
| Features | HIGH | Grounded in tuwunel CI pipeline (`test.yml`), Complement results (`results.jsonl`), and complement-crypto source; anti-features confirmed against Shadow official compatibility notes |
| Architecture | HIGH | Based on direct codebase inspection of `src/main/lib.rs`, `src/router/`, `src/main/runtime.rs`; Shadow YAML examples from local Shadow installation at `~/src/monero/shadow/examples/` |
| Pitfalls | HIGH | io_uring limitation from official Shadow docs; OnceLock conflicts from direct runtime.rs code inspection; RocksDB default path from direct config/mod.rs inspection; tokio tuning problem grounded in Shadow's discrete-event design |

**Overall confidence:** HIGH

### Gaps to Address

- **io_uring + Shadow exact failure mode:** Official Shadow docs state io_uring is unsupported; the exact failure mode (ENOSYS vs. silent hang vs. panic) under Shadow 3.3.0 + tokio 1.50 has not been empirically verified. Low risk — treat as HIGH risk and disable by default, but validate in Phase 1 smoke test.
- **tokio queue interval tuning for Shadow:** The recommended Shadow-compatible tokio configuration (smaller queue intervals, fewer worker threads) is derived from Shadow's design documentation, not from empirical testing with tuwunel's specific workload. The Phase 1 smoke test will surface this; treat the tuning values as starting points, not final answers.
- **Port-0 support in `src/router/serve.rs`:** The embed crate requires the bound port to be discoverable after `serve()` binds port 0. Direct inspection of `src/router/serve.rs` was not performed; this is inferred from standard TCP listener patterns. Validate at Phase 3 start.
- **Matrix federation handshake under Shadow:** Entirely uncharted. Defer until Phase 4 is scheduled; flag for dedicated research if federation tests are added to scope.

## Sources

### Primary (HIGH confidence)
- tuwunel codebase: `src/main/lib.rs`, `src/main/runtime.rs`, `src/main/args.rs`, `src/main/logging.rs`, `src/main/Cargo.toml`, `src/core/config/mod.rs` — direct inspection for OnceLock statics, runtime construction, default database path, io_uring feature flag
- tuwunel codebase: `src/router/mod.rs`, `src/router/run.rs` — direct inspection for serve lifecycle
- tuwunel `.github/workflows/test.yml` — CI pipeline showing existing test categories
- tuwunel `tests/complement/results.jsonl` — 784 test results, 514 pass baseline
- tuwunel `src/main/tests/smoke.rs` — existing in-process lifecycle test pattern
- Shadow 3.3.0 config specification: https://shadow.github.io/docs/guide/shadow_config_spec.html
- Shadow 3.3.0 compatibility notes: https://shadow.github.io/docs/guide/compatibility_notes.html
- Shadow 3.3.0 limitations: https://shadow.github.io/docs/guide/limitations.html
- Shadow 3.3.0 design overview: https://shadow.github.io/docs/guide/design_2x.html
- Shadow local examples: `~/src/monero/shadow/examples/`
- matrix-rust-client `testing/test-harness/src/lib.rs` — existing Docker harness to be replaced
- `.planning/codebase/CONCERNS.md` — server_name mismatch, RocksDB single instance, config manager transmute

### Secondary (MEDIUM confidence)
- Complement README — Docker-based architecture and test categories
- complement-crypto README — E2EE test scenarios and mitmproxy architecture
- s2.dev DST post — Turmoil/MadSim context for alternatives comparison
- Shadow GitHub issue #1839 — statically linked binaries and LD_PRELOAD
- Shadow GitHub discussion #1675 — CPU usage, simulation time, and busy-loop behavior
- WebSearch: io_uring + Shadow — confirms architectural incompatibility; exact failure mode unverified empirically

### Tertiary (LOW confidence)
- RocksDB multiple instances discussion — community confirmation that separate paths work; not tested in this specific process configuration

---
*Research completed: 2026-03-25*
*Ready for roadmap: yes*
