# Phase 2: CS API and E2EE Tests - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Write Shadow test scenarios covering the full Matrix Client-Server API path (register, login, create room, send message, sync) and E2EE key exchange (key upload, one-time key claim, encrypted messaging, SAS verification). All scenarios run under Shadow's simulated network with deterministic time. Two-client scenarios use separate Shadow hosts. Test results integrate with `cargo test` via Shadow exit codes.

</domain>

<decisions>
## Implementation Decisions

### Client Library
- **D-01:** Use matrix-sdk for all new test scenarios (CS API and E2EE). The SDK provides built-in E2EE, sync, and room management. Carries forward from Phase 1 D-01.
- **D-02:** Keep the existing smoke subcommand as raw reqwest — it's a lightweight readiness check that doesn't need the full SDK. New subcommands use matrix-sdk.
- **D-03:** matrix-sdk version alignment with the workspace's ruma git fork is at Claude's discretion. May require a [patch] section, a compatible git rev, or a matrix-sdk fork. Researcher should evaluate compatibility.

### Scenario Structure
- **D-04:** One subcommand per flow — each subcommand runs a complete end-to-end scenario. e.g., `cs-api` runs register→login→room→message→sync; `e2ee-messaging` runs key upload→claim→encrypt→send; `sas-verify` runs SAS protocol.
- **D-05:** Each scenario is self-contained — registers its own users, creates its own rooms. No shared state between processes, no setup phase.
- **D-06:** Results via exit code + stderr log, consistent with smoke subcommand. Exit 0 on full success, non-zero on first failure. Integration test reads Shadow's per-host stderr files for detailed assertions.

### Two-Client Topology
- **D-07:** Two test clients (alice and bob) run on separate Shadow hosts with their own virtual IPs. More realistic — traffic goes through Shadow's network simulation.
- **D-08:** Deterministic naming for coordination — alice always registers as @alice:tuwunel-server, bob as @bob:tuwunel-server. Room alias is pre-agreed (e.g., #test-room:tuwunel-server). Bob joins by alias. No runtime coordination needed.
- **D-09:** Timing between alice and bob is at Claude's discretion. Options include Shadow start_time offsets or bob polling for room existence. Researcher should evaluate based on how long operations take under simulated time.

### E2EE Determinism
- **D-10:** Support both in-memory and SQLite crypto stores. In-memory is the default for speed; SQLite in a tempdir is available as an option for more realistic testing. Both must work.
- **D-11:** E2EE key exchange orchestration approach is at Claude's discretion — natural SDK sync loop vs explicit step-by-step. Researcher should evaluate based on matrix-sdk's API surface.
- **D-12:** SAS verification is automated — both clients auto-accept the emoji match. Proves the protocol works end-to-end under Shadow without human interaction.

### Claude's Discretion
- matrix-sdk version alignment with ruma fork (D-03)
- Timing approach between alice and bob (D-09) — start_time offset vs polling
- E2EE key exchange orchestration (D-11) — SDK sync loop vs explicit steps
- Shadow stop_time values for each scenario — researcher evaluates based on operation durations

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase 1 Artifacts (foundation this phase builds on)
- `tests/shadow/src/bin/matrix_test_client.rs` — existing binary with smoke subcommand; new subcommands added here
- `tests/shadow/src/config/shadow.rs` — ShadowConfig structs for YAML generation; extended for multi-host topologies
- `tests/shadow/src/config/tuwunel.rs` — TuwunelConfig for server TOML generation
- `tests/shadow/src/runner.rs` — Shadow runner with ShadowResult; used by integration tests
- `tests/shadow/tests/smoke.rs` — existing smoke integration test; new scenario tests follow this pattern
- `.planning/phases/01-shadow-infrastructure/01-CONTEXT.md` — Phase 1 decisions (D-01 through D-07)
- `.planning/phases/01-shadow-infrastructure/01-RESEARCH.md` — Shadow pitfalls, patterns, and anti-patterns

### Tuwunel Server
- `Cargo.toml` (workspace root) — workspace deps, shared versions, shadow profile
- `src/main/Cargo.toml` — feature flags including shadow_features
- `tuwunel-example.toml` — server config reference for registration_token, allow_registration, etc.

### Matrix Protocol
- matrix-sdk documentation — Client, SyncSettings, encryption module, crypto store backends
- ruma API types — registration, login, room creation, message sending, key upload, key claim

### Existing Test Infrastructure
- `.planning/codebase/TESTING.md` — test framework patterns, run commands
- `tests/complement/` — existing complement test infrastructure for reference

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `shadow_test_harness::config::shadow::ShadowConfig` — extend for multi-host topologies (alice, bob, server)
- `shadow_test_harness::config::tuwunel::TuwunelConfig` — reuse for server config, may need allow_registration and encryption settings
- `shadow_test_harness::runner::run_shadow` — reuse for all integration tests
- `ShadowResult::find_host_stdouts/stderrs` — reuse for per-host assertion checking
- Smoke subcommand's retry/polling pattern — reusable for readiness checks in new subcommands

### Established Patterns
- Single binary with clap subcommands — add new Commands variants for cs-api, e2ee-messaging, sas-verify
- Integration tests in `tests/shadow/tests/` — add smoke_cs_api.rs, smoke_e2ee.rs, etc.
- Shadow YAML generation with BTreeMap hosts — extend for 3-host configs (server + alice + bob)
- Exit code + stderr logging for results
- `#[ignore]` attribute on Shadow tests for explicit opt-in

### Integration Points
- `tests/shadow/Cargo.toml` — needs matrix-sdk dependency addition
- `Commands` enum in matrix_test_client.rs — add new variants
- TuwunelConfig — may need additional fields for E2EE-related server settings

</code_context>

<specifics>
## Specific Ideas

- Both crypto store backends (in-memory and SQLite) should be configurable, probably via a CLI flag on the E2EE subcommands
- SAS verification auto-accepts emoji match — test proves protocol completes, not UX
- Deterministic naming: @alice:tuwunel-server, @bob:tuwunel-server, #test-room:tuwunel-server

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-cs-api-and-e2ee-tests*
*Context gathered: 2026-03-25*
