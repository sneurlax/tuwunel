# Phase 1: Shadow Infrastructure - Research

**Researched:** 2026-03-25
**Domain:** Shadow network simulation, Cargo build profiles, Shadow YAML config generation, Matrix smoke testing
**Confidence:** HIGH

## Summary

Phase 1 establishes the Shadow simulation infrastructure for deterministic E2E testing of tuwunel. Shadow v3.3.0 is already installed at `~/.local/bin/shadow` on the development machine. The core work involves: (1) creating a Cargo build profile that disables `io_uring`, (2) building a test client binary that makes HTTP requests to verify server readiness, (3) generating Shadow YAML configs programmatically from Rust, and (4) wiring everything together so `shadow smoke.yaml` runs tuwunel under simulated networking with deterministic seed control.

Shadow runs real Linux binaries as separate processes on virtual hosts with simulated networking. Each host gets its own virtual IP address, and processes communicate over Shadow's simulated TCP/IP stack. This means tuwunel runs as a normal binary -- the Shadow harness does NOT embed the server in-process. The test infrastructure generates config files on disk, writes a Shadow YAML config, and invokes `shadow` as a subprocess.

**Primary recommendation:** Build a `tests/shadow/` workspace member containing: (a) a `matrix-test-client` binary with clap subcommands, (b) Shadow YAML config generation via serde_yaml, (c) TOML config generation for tuwunel instances, and (d) a Cargo integration test that invokes `shadow` as a subprocess and asserts on output.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Test client is a Rust binary using matrix-sdk as the HTTP/Matrix client library. This gives full E2EE support out of the box for Phase 2 scenarios.
- **D-02:** Single binary with clap subcommands (e.g., `matrix-test-client smoke`, `matrix-test-client auth`). Shadow YAML references the same binary with different args.
- **D-04:** Primary: Poll `/_matrix/client/versions` endpoint in a retry loop with simulated-time backoff. Secondary: Parse Shadow's captured stdout for error diagnostics if polling fails.
- **D-05:** Dedicated Cargo profile `[profile.shadow]` in workspace `Cargo.toml` (inherits from release). Paired with explicit feature set excluding `io_uring`.
- **D-06:** Build-time enforcement via `compile_error!` -- if both `io_uring` and a `shadow` cfg marker are active simultaneously, compilation fails.

### Claude's Discretion
- Test client crate location (D-03) -- Claude picks based on workspace conventions and build system constraints
- Port 0 implementation approach and phase placement (D-07) -- evaluate Shadow's virtual network model first
- Port exposure mechanism if port 0 is kept in Phase 1 -- log + file write vs shared state

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| SHAD-01 | Shadow YAML configs can be generated programmatically from Rust structs via serde_yaml | serde_yaml 0.9 already in workspace deps; Shadow YAML format fully documented in spec |
| SHAD-02 | Tuwunel binary starts under Shadow and responds to HTTP on virtual IP | Shadow assigns virtual IPs (starting at 11.0.0.1); tuwunel binds to configured address/port; test client polls `/_matrix/client/versions` |
| SHAD-03 | io_uring feature disabled in Shadow build profile | `io_uring` feature cascades through all 6 crates from `src/main/Cargo.toml`; database crate enables `rust-rocksdb/io-uring` |
| SHAD-04 | Tuwunel config constructed programmatically via figment without on-disk TOML | Shadow runs separate processes, so config must be on disk as TOML file; generate TOML from Rust structs using `toml::to_string()`, write to tempdir, pass via `-c` arg or `TUWUNEL_CONFIG` env |
| SHAD-05 | Server readiness detected automatically | Test client polls `/_matrix/client/versions` in retry loop with backoff; Shadow's simulated time makes busy-wait safe with `model_unblocked_syscall_latency: true` |
| SHAD-06 | Per-host stdout/stderr accessible for assertions | Shadow writes to `shadow.data/hosts/<hostname>/<procname>.<pid>.stdout` and `.stderr` |
| SHAD-07 | All configs use explicit deterministic seed and stop_time | `general.seed` (default 1) and `general.stop_time` (required) in Shadow YAML |
| SHAD-08 | PCAP capture available per host | `host_option_defaults.pcap_enabled: true` and `pcap_capture_size` in Shadow YAML; output at `shadow.data/hosts/<hostname>/eth0.pcap` |
| SHAD-09 | On failure, seed and log paths printed | Test harness captures Shadow exit code, prints seed value and `shadow.data/hosts/` path on non-zero exit |
| CONF-01 | Port 0 (OS-assigned) with exposed actual port | Shadow's virtual networking assigns each host its own IP; hardcoded port 8448 avoids conflicts; port 0 deferred to Phase 4 |
| CONF-02 | Config generated as tempfile TOML for Shadow process args | Generate TOML via `toml::to_string()` of config struct, write to tempdir within Shadow's `data_directory` |
| CONF-03 | Each test instance gets isolated tempdir for RocksDB | Shadow runs each host in its own working directory under `shadow.data/hosts/<hostname>/`; database_path set to relative path within host dir |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_yaml | 0.9 | Shadow YAML config generation | Already in workspace deps; direct serialization of Rust structs to Shadow YAML format |
| toml | 1.0 | Tuwunel TOML config generation | Already in workspace deps; serialize config structs to TOML files for Shadow processes |
| clap | 4.5 | Test client CLI with subcommands | Already in workspace deps; matches tuwunel's own CLI pattern |
| matrix-sdk | 0.16 | HTTP/Matrix client for test scenarios | Locked decision D-01; provides E2EE for Phase 2; uses reqwest internally |
| serde | 1.0 | Serialization for config structs | Already in workspace deps |
| tempfile | 3.x | Temporary directories for configs and databases | Standard Rust temp directory management |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| reqwest | 0.13 | Direct HTTP calls if matrix-sdk is too heavy for smoke | Already in workspace; matrix-sdk uses it internally |
| tokio | 1.50 | Async runtime for test client | Already in workspace; needed for matrix-sdk |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| matrix-sdk for Phase 1 smoke | Plain reqwest GET | Simpler for just `/_matrix/client/versions`, but D-01 locks matrix-sdk; the binary is reused in Phase 2 |
| serde_yaml for Shadow config | Handwritten YAML strings | Fragile, no type safety; serde_yaml gives compile-time structure |

**Installation:**
```bash
# matrix-sdk added to new test crate's Cargo.toml
# Other deps already in workspace
cargo add --path tests/shadow matrix-sdk tempfile
```

## Architecture Patterns

### Recommended Project Structure
```
tests/shadow/
    Cargo.toml              # New workspace member
    src/
        lib.rs              # Shared test infrastructure
        config/
            mod.rs          # Config generation module
            shadow.rs       # Shadow YAML config structs + generation
            tuwunel.rs      # Tuwunel TOML config generation
        runner.rs           # Shadow process invocation + output parsing
    src/bin/
        matrix-test-client.rs  # Test client binary (clap subcommands)
    tests/
        smoke.rs            # cargo test integration test invoking shadow
```

### Pattern 1: Shadow Config as Typed Rust Structs
**What:** Define Shadow YAML schema as Rust structs with `#[derive(Serialize)]`, generate YAML via `serde_yaml::to_string()`.
**When to use:** Every Shadow test scenario.
**Example:**
```rust
// Shadow YAML config structs
#[derive(Serialize)]
struct ShadowConfig {
    general: General,
    network: Network,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_option_defaults: Option<HostOptionDefaults>,
    hosts: BTreeMap<String, Host>,
}

#[derive(Serialize)]
struct General {
    stop_time: String,        // e.g. "30s"
    seed: u32,                // deterministic seed
    model_unblocked_syscall_latency: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    log_level: Option<String>,
}

#[derive(Serialize)]
struct Network {
    graph: NetworkGraph,
}

#[derive(Serialize)]
struct NetworkGraph {
    #[serde(rename = "type")]
    graph_type: String,       // "1_gbit_switch"
}

#[derive(Serialize)]
struct Host {
    network_node_id: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    host_options: Option<HostOptions>,
    processes: Vec<Process>,
}

#[derive(Serialize)]
struct Process {
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    expected_final_state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    environment: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shutdown_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shutdown_signal: Option<String>,
}

#[derive(Serialize)]
struct HostOptionDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pcap_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pcap_capture_size: Option<String>,
}
```

### Pattern 2: Tuwunel Config Generation via TOML
**What:** Generate minimal tuwunel config as TOML, write to temp file, pass to binary via `-c` flag or `TUWUNEL_CONFIG` env var.
**When to use:** Every Shadow test that runs a tuwunel instance.
**Example:**
```rust
use std::collections::BTreeMap;

/// Generate minimal tuwunel TOML config for Shadow testing
fn generate_tuwunel_config(
    server_name: &str,
    database_path: &str,
    address: &str,
    port: u16,
) -> String {
    // Build TOML manually for clarity and control
    format!(
        r#"[global]
server_name = "{server_name}"
database_path = "{database_path}"
address = "{address}"
port = {port}
allow_registration = true
registration_token = "shadow_test_token"
log = "info"
"#
    )
}
```

### Pattern 3: Shadow Runner with Output Capture
**What:** Invoke `shadow` binary as subprocess, capture exit code, parse output directory for per-host stdout/stderr/pcap.
**When to use:** Every integration test.
**Example:**
```rust
use std::process::Command;
use std::path::{Path, PathBuf};

struct ShadowResult {
    exit_code: i32,
    data_dir: PathBuf,
    seed: u32,
}

impl ShadowResult {
    fn host_stdout(&self, hostname: &str, process: &str, pid: u32) -> PathBuf {
        self.data_dir
            .join("hosts")
            .join(hostname)
            .join(format!("{process}.{pid}.stdout"))
    }

    fn host_stderr(&self, hostname: &str, process: &str, pid: u32) -> PathBuf {
        self.data_dir
            .join("hosts")
            .join(hostname)
            .join(format!("{process}.{pid}.stderr"))
    }
}

fn run_shadow(config_path: &Path, seed: u32) -> ShadowResult {
    let data_dir = config_path.parent().unwrap().join("shadow.data");
    let output = Command::new("shadow")
        .arg(config_path)
        .arg("--seed")
        .arg(seed.to_string())
        .arg("--data-directory")
        .arg(&data_dir)
        .output()
        .expect("failed to execute shadow");

    if !output.status.success() {
        eprintln!("Shadow failed with seed={seed}");
        eprintln!("Log directory: {}", data_dir.display());
        eprintln!("Shadow stderr: {}", String::from_utf8_lossy(&output.stderr));
    }

    ShadowResult {
        exit_code: output.status.code().unwrap_or(-1),
        data_dir,
        seed,
    }
}
```

### Pattern 4: Test Client with Retry Polling
**What:** Test client binary polls `/_matrix/client/versions` with retry loop and backoff.
**When to use:** Smoke test and all subsequent scenarios.
**Example:**
```rust
// In matrix-test-client binary
async fn wait_for_server(base_url: &str, max_retries: u32) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    let url = format!("{base_url}/_matrix/client/versions");

    for attempt in 0..max_retries {
        match client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let body = resp.text().await?;
                // Verify it's valid versions response
                let _: serde_json::Value = serde_json::from_str(&body)?;
                eprintln!("Server ready after {attempt} retries");
                return Ok(());
            }
            Ok(resp) => {
                eprintln!("Attempt {attempt}: status {}", resp.status());
            }
            Err(e) => {
                eprintln!("Attempt {attempt}: {e}");
            }
        }
        // In Shadow's simulated time, sleep advances deterministically
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    Err("Server did not become ready".into())
}
```

### Anti-Patterns to Avoid
- **Embedding tuwunel in-process for Shadow tests:** Shadow runs real binaries as separate processes. Do not try to link tuwunel into the test binary. Shadow intercepts syscalls via LD_PRELOAD, so the binary must be a standalone executable.
- **Using `std::time::Instant` for timing in test client:** Shadow controls simulated time. Use `tokio::time::sleep()` which Shadow intercepts. `Instant::now()` may not advance as expected.
- **Hardcoding absolute paths in Shadow YAML:** Shadow does path resolution relative to the config file and each host's working directory. Use relative paths where possible.
- **Setting `io_uring` feature in Shadow builds:** io_uring uses syscalls Shadow cannot intercept. The `compile_error!` guard (D-06) prevents this at build time.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML serialization | String formatting | serde_yaml | Shadow YAML has complex nesting (hosts, processes, network graph); type safety prevents malformed configs |
| TOML generation | String templates | toml crate's `to_string()` | Proper escaping, type correctness |
| HTTP client for testing | Raw TCP/hyper | reqwest (via matrix-sdk) | Connection pooling, timeout handling, TLS; matrix-sdk wraps it for Matrix-specific APIs |
| Temp directory management | Manual mkdir + cleanup | tempfile crate | RAII cleanup, unique names, cross-platform |
| CLI argument parsing | Manual argv parsing | clap derive | Already the project standard; typed, documented, extensible |

**Key insight:** Shadow handles all networking, time, and process isolation. The test infrastructure's job is config generation and output assertion -- not simulating anything.

## Common Pitfalls

### Pitfall 1: Busy Loops Causing Shadow Deadlock
**What goes wrong:** Programs with busy-wait loops (spinning without syscalls) can deadlock Shadow because Shadow only advances simulated time on syscall interception.
**Why it happens:** tokio's event loop and some internal polling can spin without making syscalls.
**How to avoid:** Always set `model_unblocked_syscall_latency: true` in Shadow config. This adds small simulated latency to non-blocking syscalls, allowing time to advance.
**Warning signs:** Shadow simulation hangs, never reaches `stop_time`.

### Pitfall 2: OnceLock Panic in Runtime
**What goes wrong:** `runtime.rs` has `WORKER_AFFINITY`, `GC_ON_PARK`, and `GC_MUZZY` as `OnceLock` statics that `.set().expect()` on first init. If somehow initialized twice (not a concern for Shadow since each host is a separate process), it panics.
**Why it happens:** The code uses `.expect("set X from program argument")` which panics on second call.
**How to avoid:** Not a problem for Shadow (separate processes), but relevant context for Phase 4 (embed crate).
**Warning signs:** Panic with message "set WORKER_AFFINITY from program argument".

### Pitfall 3: Shadow IPv6 Limitation
**What goes wrong:** Shadow does not support IPv6. Tuwunel's default bind address includes `::1` (IPv6 localhost).
**Why it happens:** Shadow's network simulation only handles IPv4.
**How to avoid:** Always configure tuwunel with `address = "0.0.0.0"` (or a specific IPv4 address) in Shadow configs. Never use the default `["127.0.0.1", "::1"]`.
**Warning signs:** Bind failure errors in tuwunel's Shadow stdout.

### Pitfall 4: Shadow Working Directory Structure
**What goes wrong:** Assuming Shadow runs processes in a predictable CWD.
**Why it happens:** Shadow creates `shadow.data/hosts/<hostname>/` as each host's working directory. File paths in process args are relative to this.
**How to avoid:** Use absolute paths for binaries and config files in Shadow YAML `path` and `args` fields. The tuwunel config TOML file should be written to an absolute path and referenced absolutely. The `database_path` in tuwunel config can be relative (it resolves from the host's CWD, which is `shadow.data/hosts/<hostname>/`).
**Warning signs:** "file not found" errors in Shadow process output.

### Pitfall 5: Shadow Requires Dynamically Linked Binaries
**What goes wrong:** Statically linked binaries cannot be intercepted by Shadow's LD_PRELOAD shim.
**Why it happens:** Shadow uses `LD_PRELOAD` to intercept libc syscalls. Static binaries don't load shared libraries.
**How to avoid:** Use the default Rust build target (`x86_64-unknown-linux-gnu`) which links glibc dynamically. Do NOT use musl targets for Shadow builds. The shadow profile should not enable static linking.
**Warning signs:** Shadow processes execute but networking doesn't work (packets never arrive).

### Pitfall 6: Port 0 Not Needed Under Shadow
**What goes wrong:** Spending effort implementing port 0 support when Shadow's virtual networking makes it unnecessary.
**Why it happens:** In normal testing, multiple servers on localhost need unique ports. Under Shadow, each host has its own virtual IP, so port 8448 works for every server without conflict.
**How to avoid:** Use a fixed port (e.g., 8448) in Shadow configs. Defer CONF-01 (port 0) to Phase 4 where it is needed for the embed crate's in-process use.
**Warning signs:** N/A -- this is a design decision, not a runtime error.

### Pitfall 7: Shadow Process PID in Output Filenames
**What goes wrong:** Assuming a fixed PID when reading Shadow output files.
**Why it happens:** Shadow output files are named `<procname>.<pid>.stdout`. The PID is assigned by Shadow and starts at 1000 for the first process on each host.
**How to avoid:** Use glob patterns to find output files, or know that Shadow assigns PIDs starting at 1000 in order of process start_time.
**Warning signs:** "File not found" when trying to read specific PID-stamped output files.

### Pitfall 8: Workspace Member Location
**What goes wrong:** Placing the test crate under `src/` makes it part of the main tuwunel workspace build, slowing down normal development builds.
**Why it happens:** `members = ["src/*"]` in root `Cargo.toml` auto-includes everything under `src/`.
**How to avoid:** Place the test crate under `tests/shadow/` and explicitly add it to workspace members: `members = ["src/*", "tests/shadow"]`. This keeps it in the workspace for shared deps but outside the `src/*` glob.
**Warning signs:** `cargo build` compiles matrix-sdk and test client unnecessarily.

## Code Examples

### Shadow YAML for Smoke Test
```yaml
# Generated by tests/shadow/src/config/shadow.rs
general:
  stop_time: 30s
  seed: 42
  model_unblocked_syscall_latency: true
  log_level: info

network:
  graph:
    type: 1_gbit_switch

host_option_defaults:
  pcap_enabled: true

hosts:
  tuwunel-server:
    network_node_id: 0
    processes:
    - path: /absolute/path/to/target/shadow/tuwunel
      args: -c /absolute/path/to/tuwunel-config.toml
      start_time: 1s
      expected_final_state: running
      environment:
        TUWUNEL_LOG: info

  test-client:
    network_node_id: 0
    processes:
    - path: /absolute/path/to/target/shadow/matrix-test-client
      args: smoke --server-url http://tuwunel-server:8448
      start_time: 5s
```

### Tuwunel Config for Shadow
```toml
# Generated by tests/shadow/src/config/tuwunel.rs
[global]
server_name = "tuwunel-server"
database_path = "data"
address = "0.0.0.0"
port = 8448
allow_registration = true
registration_token = "shadow_test_token"
log = "info"
# Disable startup netburst (federation pings) since Shadow has no external servers
startup_netburst = false
```

### Cargo Profile for Shadow
```toml
# In workspace Cargo.toml
[profile.shadow]
inherits = "release"
strip = "none"           # Keep symbols for debugging Shadow issues
debug = "limited"        # Some debug info for crash diagnostics
```

### compile_error Guard for io_uring + Shadow
```rust
// In src/main/lib.rs or a dedicated build guard module
#[cfg(all(feature = "io_uring", shadow))]
compile_error!(
    "io_uring feature is incompatible with Shadow builds. \
     Use `cargo build --profile shadow --no-default-features --features <shadow-features>` \
     to build without io_uring."
);
```

### Build Command
```bash
# Build tuwunel without io_uring for Shadow
cargo build --profile shadow --no-default-features --features \
  brotli_compression,element_hacks,gzip_compression,jemalloc,jemalloc_conf,\
  media_thumbnail,release_max_log_level,url_preview,zstd_compression

# Build test client (it lives in same workspace)
cargo build --profile shadow --package shadow-test-harness --bin matrix-test-client
```

### Integration Test Runner
```rust
// tests/shadow/tests/smoke.rs
#[test]
fn shadow_smoke() {
    // 1. Build paths to binaries
    let tuwunel_bin = env!("CARGO_BIN_EXE_tuwunel"); // from cargo test
    let client_bin = env!("CARGO_BIN_EXE_matrix-test-client");

    // 2. Create temp directory for this test run
    let tmp = tempfile::tempdir().unwrap();

    // 3. Generate tuwunel config
    let config_path = tmp.path().join("tuwunel.toml");
    std::fs::write(&config_path, generate_tuwunel_config(...)).unwrap();

    // 4. Generate Shadow YAML
    let shadow_yaml = tmp.path().join("shadow.yaml");
    std::fs::write(&shadow_yaml, generate_shadow_config(
        tuwunel_bin, client_bin, &config_path, seed: 42,
    )).unwrap();

    // 5. Run Shadow
    let result = run_shadow(&shadow_yaml, 42);

    // 6. Assert
    assert_eq!(result.exit_code, 0,
        "Shadow smoke test failed.\n\
         Seed: {}\n\
         Logs: {}",
        result.seed,
        result.data_dir.display(),
    );

    // 7. Verify determinism (optional in smoke, required for SHAD-07)
    let result2 = run_shadow(&shadow_yaml, 42);
    // Compare stdout files for identical output
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Docker Complement tests | Shadow deterministic simulation | This project | Deterministic, reproducible, no Docker dependency |
| Manual Shadow YAML writing | Programmatic YAML generation via serde | This project | Type-safe, composable test scenarios |

## Open Questions

1. **Tokio busy-loop behavior under Shadow**
   - What we know: `model_unblocked_syscall_latency: true` mitigates most busy loops. Tokio's default `global_queue_interval` (192) and `event_interval` (512) may spin.
   - What's unclear: Whether tuwunel's tokio runtime with its custom event intervals needs tuning for Shadow, or if `model_unblocked_syscall_latency` is sufficient.
   - Recommendation: Start with `model_unblocked_syscall_latency: true` and default intervals. Tune only if Shadow hangs.

2. **Shadow stop_time for smoke test**
   - What we know: Tuwunel needs to start, initialize RocksDB, bind port, and respond to one HTTP request.
   - What's unclear: How long (simulated time) this takes under Shadow. Real-world startup is a few seconds, but Shadow's simulated time advances differently.
   - Recommendation: Start with `30s` stop_time. Increase if needed. The test client's retry loop handles variable startup time.

3. **matrix-sdk weight for Phase 1**
   - What we know: D-01 locks matrix-sdk. Phase 1 only needs a GET to `/_matrix/client/versions`.
   - What's unclear: Whether pulling in matrix-sdk's full dependency tree is acceptable for the test client binary size under Shadow.
   - Recommendation: Use matrix-sdk in the crate dependency but for Phase 1 smoke command, use just reqwest (which matrix-sdk depends on anyway). The full matrix-sdk Client is used starting Phase 2.

4. **CONF-01 (Port 0) deferral**
   - What we know: Shadow gives each host its own virtual IP. Port 8448 works on every host without collision.
   - What's unclear: Whether any future Phase 1 scenario needs port 0.
   - Recommendation: Defer CONF-01 to Phase 4. Use fixed port 8448 in all Shadow configs. Document this as a known simplification.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| shadow | Shadow simulation runner | Yes | 3.3.0 | -- |
| cargo | Build system | Yes | 1.94.0 | -- |
| rustc | Compiler | Yes | 1.94.0 (nightly) | -- |
| cmake | Shadow build (if rebuilding) | Yes | 3.28.3 | Not needed; Shadow already installed |

**Missing dependencies with no fallback:** None.

**Missing dependencies with fallback:** None.

## Project Constraints (from CLAUDE.md)

- Hard tabs, max width 98, edition 2024
- `group_imports = "StdExternalCrate"`, `imports_granularity = "Crate"`
- `unwrap_used = "warn"` (no unwrap outside tests)
- `str_to_string = "warn"` (prefer `.to_owned()`)
- `as_conversions = "warn"` (explicit casts need `#[expect]`)
- Workspace uses `members = ["src/*"]` -- new crate at `tests/shadow/` needs explicit addition
- GSD workflow enforcement -- work through GSD commands
- io_uring must be disabled for Shadow builds
- No whitespace in paths (Shadow LD_PRELOAD requirement)
- RocksDB needs separate tempdir per instance

## Sources

### Primary (HIGH confidence)
- Shadow source code at `~/src/monero/shadow` -- YAML config format, examples, compatibility notes
- Shadow config spec: `~/src/monero/shadow/docs/shadow_config_spec.md` -- all YAML options documented
- Shadow compatibility notes: `~/src/monero/shadow/docs/compatibility_notes.md` -- known application issues
- Shadow determinism testing: `~/src/monero/shadow/docs/testing_determinism.md` -- seed-based reproducibility
- Tuwunel source: `src/main/Cargo.toml` lines 55-196 -- feature flags including `io_uring` cascade
- Tuwunel source: `src/core/config/mod.rs` lines 3079-3231 -- `ListeningPort`, `get_bind_addrs()`
- Tuwunel source: `src/router/serve.rs` -- TCP binding via `axum_server::bind()`
- Tuwunel source: `src/main/runtime.rs` -- OnceLock statics
- Tuwunel source: `src/main/args.rs` -- CLI args and `default_test()`
- Tuwunel source: `src/main/tests/smoke.rs` -- existing smoke test pattern

### Secondary (MEDIUM confidence)
- matrix-sdk 0.16.0 on crates.io -- latest version, uses reqwest 0.12+
- Shadow v3.3.0 installed at `~/.local/bin/shadow` -- verified with `shadow --version`

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- all libraries verified in workspace deps or crates.io
- Architecture: HIGH -- Shadow's process model is well-understood from docs and examples; tuwunel's config system is thoroughly documented in source
- Pitfalls: HIGH -- Shadow compatibility notes document known issues; io_uring incompatibility confirmed by project constraints; IPv6 limitation documented

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable -- Shadow 3.3.0 and tuwunel workspace are pinned)
