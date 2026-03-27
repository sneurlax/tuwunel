# Phase 3: Network Conditions and Load - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Add named network topology fixtures (latency, packet loss, bandwidth limits) as reusable Shadow configurations, verify E2EE messaging works under degraded network conditions, and run a 100-client concurrent load test — all under Shadow's deterministic simulation. No tuwunel server code changes.

</domain>

<decisions>
## Implementation Decisions

### Topology Fixtures
- **D-01:** Topology fixtures are Rust builder functions in the shadow config module (e.g., `TopologyFixture::slow_mobile()`). Extends existing `ShadowConfig` pattern. Type-safe, composable, documented. Tests select by function call, not filename.
- **D-02:** Fixtures have sensible defaults but accept optional overrides via builder pattern (e.g., `.with_latency("300ms").with_loss(0.05)`). One-liner for common cases, tunable for experiments.
- **D-03:** Shadow inline GML graph format for custom topologies — replaces the built-in `1_gbit_switch` graph_type with an inline graph definition containing per-node bandwidth and per-edge latency/packet_loss.

### Impairment Values
- **D-04:** Named fixture values based on realistic mobile/network profiles:
  - `slow_mobile`: 150ms latency, 1% packet loss, 5 Mbit down / 1 Mbit up
  - `high_latency`: 500ms latency, 0% packet loss, 100 Mbit symmetric
  - `lossy_link`: 50ms latency, 5% packet loss, 10 Mbit symmetric

### Load Test Design
- **D-05:** One Shadow host per client — 100 client hosts (client-001 through client-100) plus 1 server host. Most realistic: each client gets own virtual IP and network stack.
- **D-06:** Minimal client flow per LOAD-02: register, login, join shared room, send one message. Start with this, expand if useful.
- **D-07:** Client-001 starts earliest, creates `#load-test:tuwunel-server`. Clients 002-100 start later and join by alias with retry. Same proven pattern as Phase 2's alice/bob coordination.
- **D-08:** Load test config builder generates 101 hosts programmatically (loop, not manual). Deterministic naming: `client-{NNN}` format.

### Pass/Fail Thresholds
- **D-09:** Binary pass/fail for all tests. NET-05: E2EE scenario completes (exit 0) under 200ms latency + 2% loss within Shadow stop_time. LOAD-03: all 100 clients exit 0 within stop_time. No partial success, no timing thresholds — either it works or it doesn't.
- **D-10:** Shadow stop_time values are at Claude's discretion — researcher should estimate based on operation count × impairment parameters. Load test needs generous stop_time for 100 sequential registrations under simulated time.

### Claude's Discretion
- Shadow stop_time values for each scenario (D-10)
- Whether to add more fixtures beyond the three named ones
- Internal implementation of the GML graph builder (struct layout, serialization approach)
- Whether the load test subcommand reuses existing `run_cs_api` logic or has its own simplified flow

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Shadow Network Configuration
- `~/src/monero/shadow/docs/shadow_config_spec.md` — Full Shadow YAML config spec including network.graph inline GML format with latency, packet_loss, host_bandwidth_down/up
- `~/src/monero/shadow/docs/network_graph_spec.md` — GML graph format specification for Shadow topologies
- `~/src/monero/shadow/docs/network_graph_overview.md` — Overview of Shadow network modeling

### Phase 1-2 Artifacts (foundation this phase builds on)
- `tests/shadow/src/config/shadow.rs` — Existing ShadowConfig, General, Network, Host structs; three_host_config() builder
- `tests/shadow/src/scenarios/common.rs` — MatrixClient with register_with_token, login_user, create_room, send_text_message, join_room_with_retry, sync
- `tests/shadow/src/scenarios/cs_api.rs` — CS API scenario (register, login, room, message, sync pattern)
- `tests/shadow/src/scenarios/e2ee_msg.rs` — E2EE messaging scenario (key upload, claim, encrypted exchange)
- `tests/shadow/src/runner.rs` — run_shadow(), ShadowResult with host stderr/stdout access
- `tests/shadow/tests/smoke.rs` — Integration test pattern with build_shadow_binaries()
- `tests/shadow/tests/common/mod.rs` — Shared build_shadow_binaries() helper

### Requirements
- `.planning/REQUIREMENTS.md` — NET-01 through NET-05, LOAD-01 through LOAD-03

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ShadowConfig` structs — extend Network/NetworkGraph for inline GML instead of built-in graph types
- `three_host_config()` — pattern for multi-host config builder; load test needs N-host variant
- `MatrixClient` — all CS API operations needed for load test clients (register, login, create_room, join, send_text_message)
- `run_cs_api()` alice/bob flows — load test client flow is a simplified version of this
- `build_shadow_binaries()` — shared test helper, reusable for all new integration tests

### Established Patterns
- Topology builder functions return `ShadowConfig` — new fixtures follow this pattern
- Integration tests use `#[test] #[ignore]` attribute for explicit opt-in
- Per-host stderr for result assertions via `ShadowResult::find_host_stderrs()`
- `tokio::time::sleep` for all Shadow-compatible timing

### Integration Points
- `tests/shadow/src/config/shadow.rs` — add GML graph builder and topology fixture functions
- `tests/shadow/src/bin/matrix_test_client.rs` — add `load-test` subcommand
- `tests/shadow/tests/` — add net_impairment.rs, load.rs integration tests

</code_context>

<specifics>
## Specific Ideas

- Load test uses the same `matrix_test_client` binary with a new `load-test` subcommand — consistent with existing architecture
- The 100-client config builder should be a function that takes client count as parameter (not hardcoded to 100) for flexibility
- NET-05 reuses the existing e2ee-messaging scenario under a degraded topology — tests the same flow, different network conditions

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-network-conditions-and-load*
*Context gathered: 2026-03-25*
