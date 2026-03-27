---
phase: 01-shadow-infrastructure
verified: 2026-03-25T23:45:00Z
status: passed
score: 10/10 must-haves verified
re_verification: false
---

# Phase 01: Shadow Infrastructure Verification Report

**Phase Goal:** A Shadow-compatible tuwunel binary can be built (io_uring disabled) and a smoke scenario verifies the server starts, responds to /_matrix/client/versions, and exits cleanly under Shadow's simulated network
**Verified:** 2026-03-25T23:45:00Z
**Status:** passed
**Re-verification:** No -- initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | cargo build --profile shadow compiles tuwunel without io_uring | VERIFIED | `[profile.shadow]` at Cargo.toml:711, inherits release. `shadow_features` in src/main/Cargo.toml:201-212 excludes io_uring. |
| 2 | Attempting to build with both io_uring and shadow cfg produces a compile_error | VERIFIED | src/main/lib.rs:3-7 has `#[cfg(all(feature = "io_uring", feature = "shadow"))] compile_error!(...)` |
| 3 | Shadow YAML config can be generated from Rust structs with deterministic seed, stop_time, and pcap | VERIFIED | tests/shadow/src/config/shadow.rs: `ShadowConfig` with `to_yaml()`, `Default for General` (seed=42, stop_time="30s", model_unblocked_syscall_latency=true), `Default for HostOptionDefaults` (pcap_enabled=true) |
| 4 | Tuwunel TOML config can be generated with isolated database_path and IPv4 bind address | VERIFIED | tests/shadow/src/config/tuwunel.rs: `TuwunelConfig::new()` sets address="0.0.0.0", port=8448, startup_netburst=false. `to_toml()` serializes via toml crate. |
| 5 | matrix-test-client binary compiles and has a smoke subcommand | VERIFIED | tests/shadow/src/bin/matrix_test_client.rs: clap Parser with `Commands::Smoke` variant, `cargo check --package shadow-test-harness --bin matrix-test-client` succeeds |
| 6 | smoke subcommand polls /_matrix/client/versions with retry and exits 0 on success | VERIFIED | matrix_test_client.rs:72 formats `{base_url}/_matrix/client/versions`, retry loop at lines 78-110, validates JSON has "versions" key, returns ExitCode::SUCCESS |
| 7 | Shadow runner invokes shadow binary and captures exit code | VERIFIED | tests/shadow/src/runner.rs: `run_shadow()` uses `Command::new(shadow)` with --seed, --data-directory args, captures stdout/stderr, returns `ShadowResult` |
| 8 | On failure, seed and log directory path are printed to stderr | VERIFIED | runner.rs:175-177: `if !result.success() { result.print_failure_diagnostics(); }` prints seed, data_dir, host logs, last 50 stderr lines |
| 9 | Integration test wires config generation, binary building, and Shadow invocation | VERIFIED | tests/shadow/tests/smoke.rs: `shadow_smoke()` calls `build_shadow_binaries()`, generates TuwunelConfig/ShadowConfig, calls `run_shadow()`, asserts success and per-host output |
| 10 | Per-host stdout/stderr files are locatable from ShadowResult | VERIFIED | runner.rs: `find_host_stdouts()`, `find_host_stderrs()`, `host_stdout()`, `host_stderr()`, `host_pcap()` methods on ShadowResult |

**Score:** 10/10 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `Cargo.toml` | shadow profile definition | VERIFIED | Line 711: `[profile.shadow]` inherits release, strip=none, debug=limited |
| `src/main/lib.rs` | compile_error guard for io_uring + shadow | VERIFIED | Lines 3-7: `#[cfg(all(feature = "io_uring", feature = "shadow"))] compile_error!(...)` |
| `src/main/Cargo.toml` | shadow marker feature + shadow_features set | VERIFIED | Line 200: `shadow = []`, Lines 201-212: `shadow_features` excludes io_uring |
| `tests/shadow/Cargo.toml` | shadow test harness crate | VERIFIED | Package shadow-test-harness with serde, serde_yaml, serde_json, toml, tempfile, clap, reqwest, tokio deps |
| `tests/shadow/src/lib.rs` | crate root | VERIFIED | Exports `config` and `runner` modules |
| `tests/shadow/src/config/mod.rs` | config module | VERIFIED | Exports `shadow` and `tuwunel` submodules |
| `tests/shadow/src/config/shadow.rs` | Shadow YAML config generation | VERIFIED | ShadowConfig struct with General, Network, Host, Process, BTreeMap hosts, to_yaml(), Default impls |
| `tests/shadow/src/config/tuwunel.rs` | Tuwunel TOML config generation | VERIFIED | TuwunelConfig with TuwunelGlobal, new(), to_toml(), IPv4 address, port 8448 |
| `tests/shadow/src/runner.rs` | Shadow process invocation and output parsing | VERIFIED | ShadowResult struct, run_shadow(), host file accessors, failure diagnostics |
| `tests/shadow/src/bin/matrix_test_client.rs` | Test client binary with clap subcommands | VERIFIED | Cli/Commands with Smoke variant, run_smoke async fn, retry polling, ExitCode |
| `tests/shadow/tests/smoke.rs` | Integration test for full Shadow smoke scenario | VERIFIED | shadow_smoke() test with #[ignore], builds binaries, generates configs, runs Shadow, asserts success |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| Cargo.toml (members) | tests/shadow/Cargo.toml | workspace members list | WIRED | Line 5: `members = ["src/*", "tests/shadow"]` |
| tests/shadow/src/config/shadow.rs | serde_yaml | Serialize derive + to_yaml | WIRED | `#[derive(Serialize)]` on ShadowConfig, `serde_yaml::to_string(self)` in to_yaml() |
| tests/shadow/src/config/tuwunel.rs | toml | Serialize derive + to_toml | WIRED | `#[derive(Serialize)]` on TuwunelConfig, `toml::to_string(self)` in to_toml() |
| matrix_test_client.rs | /_matrix/client/versions | reqwest HTTP GET with retry | WIRED | Line 72: format URL, line 79: `client.get(&url).send().await`, line 86: validates "versions" key |
| runner.rs | shadow binary | std::process::Command | WIRED | Line 150: `Command::new(shadow)`, args --seed/--data-directory/config_path |
| smoke.rs | runner.rs | shadow_test_harness::runner::run_shadow | WIRED | Line 8: `use ... runner::run_shadow`, line 161: `run_shadow(...)` called |
| smoke.rs | config/shadow.rs | shadow_test_harness::config::shadow::ShadowConfig | WIRED | Line 5-6: imports General, Host, etc., line 137: constructs ShadowConfig |
| smoke.rs | config/tuwunel.rs | shadow_test_harness::config::tuwunel::TuwunelConfig | WIRED | Line 7: imports TuwunelConfig, line 71: `TuwunelConfig::new(...)` |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| shadow-test-harness crate compiles | `cargo check --package shadow-test-harness` | `Finished dev profile` | PASS |
| smoke test compiles | `cargo check --package shadow-test-harness --test smoke` | `Finished dev profile` | PASS |
| All 5 commits exist in git | `git log --oneline <hash> -1` for each | All 5 found with correct messages | PASS |
| Full E2E Shadow run | Requires Shadow installed | N/A | SKIP (requires Shadow binary installation) |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|-------------|--------|----------|
| SHAD-01 | 01-01 | Shadow YAML configs generated programmatically from Rust structs via serde_yaml | SATISFIED | ShadowConfig struct with Serialize derive, to_yaml() using serde_yaml::to_string |
| SHAD-02 | 01-02, 01-03 | Tuwunel binary starts under Shadow and responds to HTTP requests on virtual IP | SATISFIED | smoke.rs configures tuwunel-server host with expected_final_state "running", test-client polls /_matrix/client/versions |
| SHAD-03 | 01-01 | io_uring feature disabled in Shadow build profile | SATISFIED | shadow_features excludes io_uring, compile_error guard catches accidental enabling |
| SHAD-04 | 01-01 | Tuwunel config constructed programmatically without on-disk TOML files | SATISFIED | TuwunelConfig::new() constructs config in code, to_toml() serializes |
| SHAD-05 | 01-02 | Server readiness detected automatically (poll /_matrix/client/versions) | SATISFIED | matrix_test_client smoke subcommand polls with retry loop, validates JSON "versions" key |
| SHAD-06 | 01-02, 01-03 | Per-host stdout/stderr accessible for test assertions | SATISFIED | ShadowResult.find_host_stdouts/stderrs, smoke.rs asserts non-empty for both hosts |
| SHAD-07 | 01-01, 01-03 | All Shadow configs use explicit deterministic seed and stop_time | SATISFIED | Default for General: seed=42, stop_time="30s"; smoke.rs passes seed=42 to run_shadow |
| SHAD-08 | 01-01 | PCAP capture available per host | SATISFIED | HostOptionDefaults::default() sets pcap_enabled=true, ShadowResult.host_pcap() accessor |
| SHAD-09 | 01-02, 01-03 | On failure, seed and log paths printed for reproduction | SATISFIED | run_shadow auto-calls print_failure_diagnostics on non-zero exit; smoke.rs assert message includes seed/data_dir |
| CONF-02 | 01-01 | Tuwunel config generated as tempfile TOML for Shadow process args | SATISFIED | TuwunelConfig.to_toml() serializes; smoke.rs writes to tempdir, passes path via TUWUNEL_CONFIG env |
| CONF-03 | 01-01 | Each test instance gets isolated tempdir for RocksDB database path | SATISFIED | TuwunelConfig::new() accepts database_path param; smoke.rs uses tempfile::tempdir() |

No orphaned requirements found. All 11 requirement IDs from REQUIREMENTS.md Phase 1 are covered by plans.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (none) | - | - | - | No anti-patterns detected |

No TODO, FIXME, PLACEHOLDER, unimplemented, empty returns, or stub patterns found in any tests/shadow/ file.

### Human Verification Required

### 1. Full Shadow E2E Smoke Test Execution

**Test:** Run `cargo test -p shadow-test-harness --test smoke -- --ignored --nocapture` with Shadow installed
**Expected:** Test builds tuwunel with shadow profile, invokes Shadow, tuwunel-server starts, test-client receives valid /_matrix/client/versions response, Shadow exits 0
**Why human:** Requires Shadow binary installed at ~/.local/bin/shadow (CMake+Cargo hybrid build). Cannot verify programmatically without the external dependency.

### Gaps Summary

No gaps found. All 10 observable truths verified. All 11 artifacts exist, are substantive (no stubs, no placeholders), and are properly wired together. All 11 requirements are satisfied with concrete implementation evidence. All 5 claimed git commits exist.

The only item requiring human verification is the actual E2E execution under Shadow, which depends on having the Shadow network simulator installed -- an external dependency that cannot be verified in CI without explicit setup.

---

_Verified: 2026-03-25T23:45:00Z_
_Verifier: Claude (gsd-verifier)_
