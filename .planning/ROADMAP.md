# Roadmap: Tuwunel E2E Testing & Embedding

## Overview

Four phases that move from a working Shadow build profile to a full in-process embedding API. Phase 1 proves tuwunel runs at all under Shadow's syscall interception. Phase 2 writes the test scenarios that matter (auth, messaging, E2EE). Phase 3 adds the network condition and load tests that differentiate this from Docker-based testing. Phase 4 builds the embed crate for fast in-process testing in downstream consumers. Phases 1-3 require zero changes to tuwunel server code; Phase 4 makes minimal, rebaseable additions.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Shadow Infrastructure** - Get a Shadow-compatible tuwunel binary running and responding to HTTP on a virtual network
- [ ] **Phase 2: CS API and E2EE Tests** - Write test scenarios for auth, messaging, and E2EE key exchange under Shadow
- [ ] **Phase 3: Network Conditions and Load** - Add impairment fixtures and concurrent-client load scenarios
- [ ] **Phase 4: Embed Crate** - Build the tuwunel-embed crate with EmbeddedHomeserver API for in-process testing

## Phase Details

### Phase 1: Shadow Infrastructure
**Goal**: A Shadow-compatible tuwunel binary can be built (io_uring disabled) and a smoke scenario verifies the server starts, responds to /_matrix/client/versions, and exits cleanly under Shadow's simulated network
**Depends on**: Nothing (first phase)
**Requirements**: SHAD-01, SHAD-02, SHAD-03, SHAD-04, SHAD-05, SHAD-06, SHAD-07, SHAD-08, SHAD-09, CONF-02, CONF-03
**Success Criteria** (what must be TRUE):
  1. `cargo build --profile shadow` (or equivalent feature set) produces a tuwunel binary with io_uring absent, verified at build time
  2. Running `shadow smoke.yaml` completes without error; the matrix-test-client binary exits 0 after receiving a valid response from /_matrix/client/versions
  3. Per-host stdout and stderr from the Shadow run are readable as files under `shadow.data/hosts/`
  4. Re-running with the same seed produces identical output (deterministic reproduction confirmed)
  5. Test failure prints the seed and path to the host log directory
**Plans:** 3 plans
Plans:
- [x] 01-01-PLAN.md — Shadow build profile, io_uring compile guard, and test harness crate with config generation
- [x] 01-02-PLAN.md — matrix-test-client binary with smoke subcommand and Shadow runner module
- [ ] 01-03-PLAN.md — Integration test wiring configs, Shadow invocation, and smoke assertion
**Deferred**: CONF-01 (port 0 support) deferred to Phase 4 per D-07 -- Shadow virtual IPs eliminate port conflicts in Phase 1-3 scenarios.

### Phase 2: CS API and E2EE Tests
**Goal**: Shadow scenarios exist for the full Matrix Client-Server API path (register, login, create room, send message, sync) and for E2EE key exchange (key upload, one-time key claim, encrypted message, SAS verification), all passing under Shadow's simulated network
**Depends on**: Phase 1
**Requirements**: TEST-01, TEST-02, TEST-03, TEST-04, TEST-05, TEST-06, E2EE-01, E2EE-02, E2EE-03, E2EE-04, E2EE-05
**Success Criteria** (what must be TRUE):
  1. `cargo test` runs the Shadow smoke and CS API scenarios and reports pass/fail via Shadow process exit codes mapped to Rust assertions
  2. A two-client Shadow scenario (clients on separate virtual hosts) can register, create a room, send a message, and have the other client receive it via sync — all within the Shadow stop_time
  3. Two clients can complete E2EE key upload, one-time key claim, and exchange an encrypted message in a Shadow scenario without any wall-clock sleeps
  4. E2EE SAS verification between two simulated devices completes under Shadow without timing-dependent retry
**Plans:** 4 plans
Plans:
- [x] 02-01-PLAN.md — matrix-sdk dependency, scenario module scaffold, multi-host Shadow config builder, subcommand stubs
- [ ] 02-02-PLAN.md — CS API scenario (register, login, room, message, sync) with two-client Shadow integration test
- [ ] 02-03-PLAN.md — E2EE messaging scenario (key upload, claim, encrypted exchange) with Shadow integration test
- [ ] 02-04-PLAN.md — SAS verification scenario (automated device verification) with Shadow integration test

### Phase 3: Network Conditions and Load
**Goal**: Named network topology fixtures (latency, packet loss, bandwidth) exist as reusable YAML templates, the E2EE messaging scenario passes under 200ms latency and 2% packet loss, and a load scenario with 100 concurrent clients all register and send at least one message successfully
**Depends on**: Phase 2
**Requirements**: NET-01, NET-02, NET-03, NET-04, NET-05, LOAD-01, LOAD-02, LOAD-03
**Success Criteria** (what must be TRUE):
  1. A test can select a named topology fixture ("slow-mobile", "high-latency", "lossy-link") by name and Shadow applies the correct per-link impairment parameters
  2. The E2EE messaging scenario passes when the network topology applies 200ms RTT latency and 2% packet loss
  3. A Shadow simulation spawns 100 concurrent matrix-test-client processes; all 100 exit 0 after registering, logging in, and sending one message
  4. The server remains responsive throughout the 100-client load run (no client times out before Shadow's stop_time)
**Plans**: TBD

### Phase 4: Embed Crate
**Goal**: A tuwunel-embed crate exists as a new workspace member; EmbeddedHomeserver::start(config) launches an in-process tuwunel server and returns a base_url; multiple instances run concurrently in the same process without panics; stop() shuts down cleanly
**Depends on**: Phase 3
**Requirements**: EMBD-01, EMBD-02, EMBD-03, EMBD-04, EMBD-05, EMBD-06, EMBD-07, EMBD-08, EMBD-09, EMBD-10
**Success Criteria** (what must be TRUE):
  1. `cargo add tuwunel-embed` (workspace path) compiles; `EmbeddedHomeserver::start(config).await` returns a running server with a reachable base_url in under 5 seconds
  2. Two EmbeddedHomeserver instances start concurrently in the same test process without panicking (OnceLock guards working)
  3. EmbeddedHomeserver::stop() completes graceful shutdown; the process does not leak the RocksDB tempdir
  4. EmbeddedHomeserver::register_user() registers a Matrix user and returns credentials usable against base_url
**Plans**: TBD
**Note**: EMBD-10 (in-memory axum Router transport) is the most complex item in this phase. Research identified it as potentially v2 scope. It is mapped here as a v1 requirement per REQUIREMENTS.md but may be deferred at plan time if the axum Router extraction proves non-trivial.

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Shadow Infrastructure | 0/3 | Planned | - |
| 2. CS API and E2EE Tests | 0/4 | Planned | - |
| 3. Network Conditions and Load | 0/TBD | Not started | - |
| 4. Embed Crate | 0/TBD | Not started | - |
