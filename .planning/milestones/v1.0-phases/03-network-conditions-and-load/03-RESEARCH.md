# Phase 3: Network Conditions and Load - Research

**Researched:** 2026-03-25
**Domain:** Shadow network simulation -- GML graph topologies, network impairment, multi-host load testing
**Confidence:** HIGH

## Summary

Phase 3 extends the existing Shadow test infrastructure (Phase 1-2) with network impairment and load testing. The core technical challenges are: (1) replacing the built-in `1_gbit_switch` graph type with inline GML graphs containing per-edge latency/packet_loss and per-node bandwidth, (2) building topology fixture functions as Rust builder patterns, (3) generating a 101-host Shadow config programmatically for the load test, and (4) adding a `load-test` subcommand to the existing `matrix_test_client` binary.

The existing codebase provides strong foundations. The `ShadowConfig`, `Network`, `NetworkGraph` structs need extending to support `type: gml` with an `inline:` field. The `MatrixClient` wrapper already has all CS API methods needed for the load test client flow (register, login, create_room, join_room_with_retry, send_text_message). The `three_host_config()` builder pattern provides the template for a parameterized N-host config builder.

**Primary recommendation:** Extend `NetworkGraph` to support inline GML, add topology fixture builder functions, create a load-test config builder with parameterized client count, and add a `load-test` subcommand to `matrix_test_client` -- all following existing patterns.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Topology fixtures are Rust builder functions in the shadow config module (e.g., `TopologyFixture::slow_mobile()`). Extends existing `ShadowConfig` pattern. Type-safe, composable, documented. Tests select by function call, not filename.
- **D-02:** Fixtures have sensible defaults but accept optional overrides via builder pattern (e.g., `.with_latency("300ms").with_loss(0.05)`). One-liner for common cases, tunable for experiments.
- **D-03:** Shadow inline GML graph format for custom topologies -- replaces the built-in `1_gbit_switch` graph_type with an inline graph definition containing per-node bandwidth and per-edge latency/packet_loss.
- **D-04:** Named fixture values based on realistic mobile/network profiles:
  - `slow_mobile`: 150ms latency, 1% packet loss, 5 Mbit down / 1 Mbit up
  - `high_latency`: 500ms latency, 0% packet loss, 100 Mbit symmetric
  - `lossy_link`: 50ms latency, 5% packet loss, 10 Mbit symmetric
- **D-05:** One Shadow host per client -- 100 client hosts (client-001 through client-100) plus 1 server host. Most realistic: each client gets own virtual IP and network stack.
- **D-06:** Minimal client flow per LOAD-02: register, login, join shared room, send one message. Start with this, expand if useful.
- **D-07:** Client-001 starts earliest, creates `#load-test:tuwunel-server`. Clients 002-100 start later and join by alias with retry. Same proven pattern as Phase 2's alice/bob coordination.
- **D-08:** Load test config builder generates 101 hosts programmatically (loop, not manual). Deterministic naming: `client-{NNN}` format.
- **D-09:** Binary pass/fail for all tests. NET-05: E2EE scenario completes (exit 0) under 200ms latency + 2% loss within Shadow stop_time. LOAD-03: all 100 clients exit 0 within stop_time. No partial success, no timing thresholds.
- **D-10:** Shadow stop_time values are at Claude's discretion.

### Claude's Discretion
- Shadow stop_time values for each scenario (D-10)
- Whether to add more fixtures beyond the three named ones
- Internal implementation of the GML graph builder (struct layout, serialization approach)
- Whether the load test subcommand reuses existing `run_cs_api` logic or has its own simplified flow

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| NET-01 | Tests can specify per-link latency in Shadow network topology | GML inline graph with `edge.latency` attribute; NetworkGraph struct extension |
| NET-02 | Tests can specify packet loss rates in Shadow network topology | GML inline graph with `edge.packet_loss` attribute; same struct extension |
| NET-03 | Tests can specify bandwidth limits per host or link | GML `node.host_bandwidth_down/up` attributes; per-host `bandwidth_down/up` override in Host struct |
| NET-04 | Named topology fixtures exist as reusable configs | TopologyFixture builder pattern with slow_mobile(), high_latency(), lossy_link() |
| NET-05 | E2EE messaging succeeds under 200ms latency and 2% packet loss | Reuse existing e2ee-messaging scenario with impaired topology config |
| LOAD-01 | Shadow simulation can spawn 100 concurrent test client processes | N-host config builder generating 101 hosts programmatically |
| LOAD-02 | All 100 clients can register, login, and send at least one message | load-test subcommand with minimal client flow (register, login, join, send) |
| LOAD-03 | Server remains responsive under concurrent load | Binary pass/fail: all 100 client processes exit 0 within stop_time |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| serde_yaml | 0.9 | Serialize Shadow YAML config with inline GML | Already in use for Shadow config generation |
| serde | 1.0 | Derive Serialize on new topology structs | Already in use throughout |
| clap | 4.5 | Add `load-test` subcommand to matrix_test_client | Already in use for CLI parsing |
| reqwest | 0.13 | HTTP client for load test Matrix API calls | Already in use in MatrixClient |
| tokio | 1.50 | Async runtime for load test client | Already in use (current_thread) |
| tempfile | (workspace) | Temp directories for test isolation | Already in use |

### Supporting
No new dependencies needed. All required functionality exists in the current workspace.

**Installation:** No new packages needed.

## Architecture Patterns

### Recommended Project Structure
```
tests/shadow/src/
  config/
    shadow.rs          # MODIFY: extend NetworkGraph, add GML builder, add topology fixtures, add N-host config builder
  scenarios/
    common.rs          # REUSE: MatrixClient already has all needed methods
    load_test.rs       # NEW: load test client flow (register, login, join, send)
  bin/
    matrix_test_client.rs  # MODIFY: add LoadTest subcommand
tests/shadow/tests/
  net_impairment.rs    # NEW: integration tests for NET-01 through NET-05
  load.rs              # NEW: integration test for LOAD-01 through LOAD-03
  common/mod.rs        # REUSE: build_shadow_binaries()
```

### Pattern 1: Inline GML Network Graph

**What:** Extend `NetworkGraph` to support `type: gml` with an `inline` field containing the GML graph string.

**Current state:** `NetworkGraph` has only `graph_type: String`. The existing `1_gbit_switch` value is a shortcut that Shadow expands internally.

**Required change:** The YAML output must look like:
```yaml
network:
  graph:
    type: gml
    inline: |
      graph [
        directed 0
        node [
          id 0
          host_bandwidth_down "5 Mbit"
          host_bandwidth_up "1 Mbit"
        ]
        edge [
          source 0
          target 0
          latency "150 ms"
          packet_loss 0.01
        ]
      ]
```

**Implementation:** Replace `NetworkGraph` with an enum or add an optional `inline` field. Since serde_yaml needs to produce the right shape, use either:
- **Option A (recommended):** `NetworkGraph { graph_type: String, inline: Option<String> }` with `#[serde(skip_serializing_if = "Option::is_none")]` on `inline`. Simple, backward-compatible.
- **Option B:** Tagged enum. More type-safe but bigger refactor.

Option A is recommended because it preserves backward compatibility with existing `1_gbit_switch` usage (where `inline` is `None`).

### Pattern 2: Topology Fixture Builder

**What:** Builder functions that return a `Network` (or the relevant sub-structs) with preconfigured impairment values.

**Example:**
```rust
pub struct TopologyFixture {
    latency_ms: u32,
    packet_loss: f64,
    bandwidth_down: String,
    bandwidth_up: String,
}

impl TopologyFixture {
    pub fn slow_mobile() -> Self {
        Self {
            latency_ms: 150,
            packet_loss: 0.01,
            bandwidth_down: "5 Mbit".to_owned(),
            bandwidth_up: "1 Mbit".to_owned(),
        }
    }

    pub fn high_latency() -> Self { /* 500ms, 0%, 100 Mbit sym */ }
    pub fn lossy_link() -> Self { /* 50ms, 5%, 10 Mbit sym */ }

    pub fn with_latency(mut self, ms: u32) -> Self {
        self.latency_ms = ms;
        self
    }

    pub fn with_loss(mut self, loss: f64) -> Self {
        self.packet_loss = loss;
        self
    }

    /// Build the GML graph string for this topology.
    pub fn to_gml(&self) -> String { /* format GML */ }

    /// Build a complete Network struct.
    pub fn to_network(&self) -> Network { /* wrap GML in NetworkGraph */ }
}
```

### Pattern 3: N-Host Config Builder for Load Test

**What:** A function that generates a `ShadowConfig` with 1 server + N client hosts, following the `three_host_config()` pattern.

**Key considerations from existing code:**
- All hosts currently use `network_node_id: 0` (single-node topology)
- Client hosts get unique names via BTreeMap keys
- Server uses `expected_final_state: "running"`, clients use `"exited"`
- Client start times must be staggered: client-001 starts first (creates room), others start later

**Example signature:**
```rust
pub fn load_test_config(
    tuwunel_bin: &Path,
    client_bin: &Path,
    config_path: &Path,
    data_dir: &Path,
    client_count: u32,
    topology: &TopologyFixture,
    stop_time: &str,
    seed: u32,
) -> ShadowConfig
```

### Pattern 4: Load Test Client Flow

**What:** New subcommand `load-test` for `matrix_test_client` binary.

**Two roles per D-07:**
- `--role creator` (client-001): register, login, create `#load-test:tuwunel-server`, send one message
- `--role joiner` (client-002 through client-100): register, login, join `#load-test:tuwunel-server` with retry, send one message

**Recommendation:** Create a separate `load_test.rs` scenario module with its own `run_load_test()` function, similar to how `run_cs_api()` and `run_e2ee_messaging()` are structured. The flow is simpler than existing scenarios (no E2EE, no sync verification), so a dedicated module keeps it clean.

**Client naming:** Each client process gets its username from a `--client-id` argument (e.g., `--client-id 001`). Username becomes `loaduser-001`.

### Anti-Patterns to Avoid
- **Hardcoded 100 in the config builder:** Make client count a parameter. D-08 says deterministic naming, not hardcoded count.
- **Using std::thread::sleep in load test client:** Must use `tokio::time::sleep` for Shadow time advancement. Already established in Phase 1-2.
- **All clients starting at the same time:** Client-001 must start before others to create the room. Others need staggered or grouped starts.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| GML string formatting | Manual string concatenation | `format!()` with a template | GML format is simple enough for format!, but must include all required attributes |
| Matrix registration flow | New HTTP code | Existing `MatrixClient::register_with_token()` | Already handles UIAA two-step flow correctly |
| Room join coordination | Custom signaling | Existing `join_room_with_retry()` | Already proven in Phase 2 for alice/bob |
| YAML serialization | Manual YAML | serde_yaml with derive(Serialize) | Already established pattern |

## Common Pitfalls

### Pitfall 1: Shadow Latency Is One-Way, RTT Is Double
**What goes wrong:** Setting `edge.latency: "200 ms"` gives 400ms RTT, not 200ms.
**Why it happens:** Shadow's `edge.latency` is per-hop, one-direction. For a self-loop edge (source == target, single-node topology), a packet traverses the edge twice (once in each direction).
**How to avoid:** For the NET-05 requirement of "200ms RTT latency", set edge latency to `100 ms` (half of desired RTT). The D-04 fixture values specify one-way latency, so `slow_mobile` at 150ms means 300ms RTT.
**Warning signs:** Tests timing out much earlier than expected; HTTP requests taking 2x expected time.
**Confidence:** MEDIUM -- Shadow docs say "latency added to packets traversing this edge" and Dijkstra uses it as weight. For a self-loop the path is source->target (same node), so latency is applied once per direction = 2x per round trip. Verify empirically.

### Pitfall 2: 100 Hosts Need Adequate stop_time
**What goes wrong:** Shadow terminates before all 100 clients finish.
**Why it happens:** Each client does: server_wait (polling with sleep) + register (2 HTTP round trips for UIAA) + login (1 RT) + join_with_retry (multiple RTs) + send_message (1 RT). Under simulated time with latency, this adds up.
**How to avoid:** Calculate: 100 clients each doing ~10 HTTP round trips. Under default topology (1ms latency), each RT takes ~2ms simulated. But server processing time under Shadow's simulated time is the real bottleneck -- RocksDB operations, key derivation, etc. With 100 sequential registrations, budget generously. Recommended: `600s` (10 minutes simulated time) for the load test, `300s` for NET-05 E2EE under impairment.
**Warning signs:** Shadow exits 0 but client processes have `expected_final_state: exited` failures.

### Pitfall 3: All Clients on Same Network Node Means No Inter-Node Latency Modeling
**What goes wrong:** With a single-node GML graph, all hosts share the same node. Edge latency only applies via the self-loop. This is correct for modeling uniform latency but means there's no per-client differentiation.
**Why it happens:** The single-node topology is simplest and matches the existing Phase 1-2 pattern.
**How to avoid:** For Phase 3 this is fine -- all clients share the same impairment profile. A multi-node topology would be needed for federation (Phase v2). Keep the single-node model.
**Warning signs:** N/A for this phase.

### Pitfall 4: BTreeMap Host Ordering Affects Shadow IP Assignment
**What goes wrong:** Shadow assigns IPs alphabetically by hostname. `client-001` through `client-100` will get IPs 11.0.0.1 through 11.0.0.101 (after sorting). The server hostname `tuwunel-server` sorts after all `client-*` names.
**Why it happens:** Shadow docs: "Automatic addresses begin at 11.0.0.1, assigned to hosts in alphabetical order."
**How to avoid:** This is fine -- clients reference the server by hostname (`tuwunel-server`), not IP. Shadow DNS resolution handles it. But be aware for debugging PCAP files.

### Pitfall 5: Client-001 Start Time Must Allow Server Readiness
**What goes wrong:** Client-001 starts before the server is ready and exhausts retries.
**Why it happens:** Under impaired network, server startup takes longer and readiness polling is slower.
**How to avoid:** Client-001 should start at `5s` (same as Phase 2). The `wait_for_server()` function already retries 60 times with 500ms intervals = 30s budget. Under 150ms one-way latency, each poll takes ~300ms RT + server processing. Budget: 5s start + 30s max poll = 35s. Remaining clients start at `10s` to give client-001 time to create the room.

### Pitfall 6: packet_loss Attribute Is Required on Edges
**What goes wrong:** Omitting `packet_loss` from GML edge causes Shadow to reject the graph.
**Why it happens:** Shadow's GML spec marks `edge.packet_loss` as Required (Type: Float).
**How to avoid:** Always include `packet_loss 0.0` even for topologies with no intended loss.

## Code Examples

### GML Graph String Generation
```rust
// Source: Shadow docs shadow_config_spec.md + network_graph_spec.md
fn build_gml(
    bandwidth_down: &str,
    bandwidth_up: &str,
    latency: &str,
    packet_loss: f64,
) -> String {
    format!(
        r#"graph [
  directed 0
  node [
    id 0
    host_bandwidth_down "{bandwidth_down}"
    host_bandwidth_up "{bandwidth_up}"
  ]
  edge [
    source 0
    target 0
    latency "{latency}"
    packet_loss {packet_loss}
  ]
]"#
    )
}
```

### Extended NetworkGraph for Inline GML
```rust
// Extends existing shadow.rs NetworkGraph
#[derive(Serialize, Clone, Debug)]
pub struct NetworkGraph {
    #[serde(rename = "type")]
    pub graph_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inline: Option<String>,
}
```

### Expected YAML Output for Impaired Topology
```yaml
network:
  graph:
    type: gml
    inline: "graph [\n  directed 0\n  node [\n    id 0\n    host_bandwidth_down \"5 Mbit\"\n    host_bandwidth_up \"1 Mbit\"\n  ]\n  edge [\n    source 0\n    target 0\n    latency \"150 ms\"\n    packet_loss 0.01\n  ]\n]\n"
```

Note: serde_yaml will serialize the multi-line string. The `|` block scalar style in YAML is preferred for readability but serde_yaml may use quoted style. Both are valid YAML and Shadow accepts both.

### Load Test Config Builder Pattern
```rust
// Pattern for generating 101 hosts programmatically
pub fn load_test_config(
    tuwunel_bin: &Path,
    client_bin: &Path,
    config_path: &Path,
    data_dir: &Path,
    client_count: u32,
    topology: &TopologyFixture,
    stop_time: &str,
    seed: u32,
) -> ShadowConfig {
    let mut hosts = BTreeMap::new();

    // Server host
    hosts.insert("tuwunel-server".to_owned(), /* server host */);

    // Client-001: creator role, starts at 5s
    hosts.insert("client-001".to_owned(), Host {
        network_node_id: 0,
        processes: vec![Process {
            path: client_bin_str,
            args: Some("load-test --server-url http://tuwunel-server:8448 --role creator --client-id 001".to_owned()),
            start_time: Some("5s".to_owned()),
            expected_final_state: Some("exited".to_owned()),
            ..
        }],
        ..
    });

    // Clients 002-N: joiner role, start at 10s
    for i in 2..=client_count {
        let name = format!("client-{i:03}");
        hosts.insert(name, Host {
            network_node_id: 0,
            processes: vec![Process {
                args: Some(format!(
                    "load-test --server-url http://tuwunel-server:8448 --role joiner --client-id {i:03}"
                )),
                start_time: Some("10s".to_owned()),
                expected_final_state: Some("exited".to_owned()),
                ..
            }],
            ..
        });
    }

    ShadowConfig {
        network: topology.to_network(),
        hosts,
        ..
    }
}
```

## stop_time Estimates (D-10)

### NET-05: E2EE Under Impairment (200ms RTT + 2% loss)
- Server startup: ~5s simulated
- Client startup delay: 5s (alice), 15s (bob)
- Each HTTP round trip: ~200ms (100ms one-way latency x2) + server processing (~100ms under Shadow) = ~300ms
- Alice flow: wait_for_server (~10 polls x 800ms = 8s) + register (2 RT = 0.6s) + login (0.3s) + key upload (0.3s) + create_room (0.3s) + invite (0.3s) + wait_for_bob (polls, ~30s max) + sync (0.3s) + send_encrypted (0.3s) = ~40s
- Bob flow: similar ~40s from bob's start time
- Total: 15s (bob start) + 40s (bob flow) + margin = ~90s
- With 2% packet loss causing retransmissions: multiply by ~1.5x
- **Recommendation: `180s` stop_time** for NET-05

### LOAD-01/02/03: 100 Concurrent Clients (default 1 Gbit topology)
- Server startup: ~5s
- Client-001 at 5s: wait + register + login + create_room + send = ~15s
- Clients 002-100 at 10s: wait + register + login + join_with_retry + send
- Each registration is sequential from client perspective but 99 clients hitting server concurrently
- Server must handle 99 concurrent UIAA flows: ~3s each under load = sequential bottleneck
- Join with retry: room alias available after client-001 creates it (~second 20)
- With 1ms default latency: each RT ~2-10ms, but server processing dominates
- 100 registrations: even if each takes 1s server-side, that's 100s sequential wall-clock
- Under Shadow simulated time, all 99 joiners start at same time but server serializes
- **Recommendation: `600s` stop_time** for load test (generous, can tune down after first run)

### Network Impairment Tests (non-E2EE)
- Simple topology application tests (NET-01 through NET-04 verification): `60s`
- These just verify the config is accepted and a basic request works under impairment

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `1_gbit_switch` built-in | Inline GML graph | Phase 3 | Enables per-link impairment |
| `three_host_config()` only | Parameterized N-host builder | Phase 3 | Enables load testing at scale |
| No network impairment | Named topology fixtures | Phase 3 | Reusable network profiles |

## Open Questions

1. **serde_yaml block scalar rendering**
   - What we know: serde_yaml 0.9 serializes multi-line strings. Shadow accepts both quoted and block scalar (`|`) YAML strings for the inline graph.
   - What's unclear: Whether serde_yaml produces `|` block scalars or quoted strings for the `inline` field. Either works.
   - Recommendation: Test with a simple round-trip. If quoted, it still works. No action needed unless Shadow rejects it.

2. **Shadow RTT vs one-way latency semantics**
   - What we know: Shadow docs say "latency added to packets traversing this edge." For a self-loop, a round trip crosses the edge twice.
   - What's unclear: Whether the NET-05 requirement "200ms RTT" means we should set 100ms edge latency.
   - Recommendation: Set edge latency to `100 ms` for 200ms RTT. Verify in the first test run. The D-04 fixture values appear to be one-way values.

3. **Load test client staggering**
   - What we know: D-07 says client-001 creates the room, others join. All 99 joiners could start at the same time.
   - What's unclear: Whether 99 simultaneous `wait_for_server` + registration attempts cause Shadow performance issues (real-world wall clock, not simulated time).
   - Recommendation: Start all joiners at `10s` (same time). Shadow handles concurrent hosts natively. If wall-clock performance is an issue, stagger in groups, but try simple approach first.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Shadow | All tests | Yes | 3.3.0 | -- |
| Rust (nightly) | Build | Yes | 1.94.0 | -- |
| cargo | Build | Yes | (via rustup) | -- |

**Missing dependencies:** None.

## Sources

### Primary (HIGH confidence)
- `~/src/monero/shadow/docs/shadow_config_spec.md` -- Full Shadow YAML config including network.graph inline GML format
- `~/src/monero/shadow/docs/network_graph_spec.md` -- GML attribute specification (latency, packet_loss, host_bandwidth_down/up)
- `~/src/monero/shadow/docs/network_graph_overview.md` -- Shadow network routing model overview

### Secondary (HIGH confidence -- project source code)
- `tests/shadow/src/config/shadow.rs` -- Existing ShadowConfig, NetworkGraph, Host structs
- `tests/shadow/src/scenarios/common.rs` -- MatrixClient with all CS API operations
- `tests/shadow/src/scenarios/e2ee_msg.rs` -- E2EE messaging scenario to reuse under impairment
- `tests/shadow/src/bin/matrix_test_client.rs` -- Existing CLI with subcommand pattern
- `tests/shadow/tests/cs_api.rs` -- Integration test pattern with three_host_config()
- `tests/shadow/tests/e2ee.rs` -- E2EE integration test pattern

## Project Constraints (from CLAUDE.md)

- **Hard tabs** for indentation (`hard_tabs = true`)
- **Max line width 98** characters
- **Edition 2024** Rust
- **Import grouping:** `StdExternalCrate` style
- **No unwrap()** outside tests (clippy `unwrap_used = "warn"`)
- **Checked arithmetic** (clippy `arithmetic_side_effects = "warn"`)
- **Snake_case** for all function/method names
- **Tests inside `#[cfg(test)]` module** or test files
- **Integration tests use `#[test] #[ignore]`** for explicit opt-in (established pattern)
- **GSD workflow required** -- use `/gsd:execute-phase` entry point

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all verified in existing code
- Architecture: HIGH -- patterns directly extend existing Phase 1-2 code with well-documented Shadow features
- Pitfalls: MEDIUM -- RTT vs one-way latency semantics need empirical verification; stop_time estimates are educated guesses

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable -- Shadow 3.3.0 and codebase unlikely to change significantly)
