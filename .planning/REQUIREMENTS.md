# Requirements: Tuwunel E2E Testing & Embedding

**Defined:** 2026-03-25
**Core Value:** Deterministic, reproducible E2E tests that verify tuwunel's Matrix protocol behavior under realistic network conditions

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Shadow Infrastructure

- [x] **SHAD-01**: Shadow YAML configs can be generated programmatically from Rust structs via serde_yaml
- [x] **SHAD-02**: Tuwunel binary starts under Shadow and responds to HTTP requests on a virtual IP
- [x] **SHAD-03**: io_uring feature is disabled in Shadow build profile (dedicated Cargo feature set or build flag)
- [x] **SHAD-04**: Tuwunel config can be constructed programmatically via figment without any on-disk TOML files
- [x] **SHAD-05**: Server readiness is detected automatically (poll /_matrix/client/versions or parse stdout)
- [x] **SHAD-06**: Per-host stdout/stderr from Shadow is accessible for test assertions and failure diagnosis
- [x] **SHAD-07**: All Shadow configs use explicit deterministic seed and stop_time for reproducibility
- [x] **SHAD-08**: PCAP capture is available per host for network-level debugging
- [x] **SHAD-09**: On test failure, seed and log paths are printed for deterministic reproduction

### Port & Config

- [ ] **CONF-01**: Tuwunel supports port 0 (OS-assigned) and exposes the actual bound port
- [x] **CONF-02**: Tuwunel config can be generated as a tempfile TOML from Rust structs for Shadow process args
- [x] **CONF-03**: Each test instance gets an isolated tempdir for RocksDB database path

### Test Scenarios — Basic CS API

- [x] **TEST-01**: User can register an account via the Matrix registration API under Shadow
- [x] **TEST-02**: User can login with username/password and receive an access token under Shadow
- [x] **TEST-03**: User can create a room and receive a room_id under Shadow
- [x] **TEST-04**: User can send a text message to a room and another user receives it via sync under Shadow
- [x] **TEST-05**: Two clients on separate Shadow hosts can exchange messages through the server
- [x] **TEST-06**: Test results integrate with `cargo test` — Shadow process exit codes map to pass/fail assertions

### Test Scenarios — E2EE

- [ ] **E2EE-01**: User can upload device keys and one-time keys to the server under Shadow
- [ ] **E2EE-02**: User can claim another user's one-time keys for establishing an Olm session under Shadow
- [ ] **E2EE-03**: Two users can exchange encrypted messages in an E2EE room under Shadow
- [ ] **E2EE-04**: E2EE key exchange completes deterministically without timing-dependent retry loops
- [x] **E2EE-05**: SAS verification between two devices completes under Shadow simulation

### Test Scenarios — Network Conditions

- [ ] **NET-01**: Tests can specify per-link latency in Shadow network topology (e.g., 200ms RTT)
- [ ] **NET-02**: Tests can specify packet loss rates in Shadow network topology (e.g., 2% loss)
- [ ] **NET-03**: Tests can specify bandwidth limits per host or link (e.g., 1 Mbit upload)
- [ ] **NET-04**: Named topology fixtures exist as reusable configs ("slow mobile", "high latency", "lossy link")
- [ ] **NET-05**: E2EE messaging succeeds under 200ms latency and 2% packet loss

### Test Scenarios — Load

- [x] **LOAD-01**: Shadow simulation can spawn 100 concurrent test client processes against one server
- [x] **LOAD-02**: All 100 clients can register, login, and send at least one message successfully
- [x] **LOAD-03**: Server remains responsive under concurrent load (sync responses within stop_time)

### Embed Crate

- [x] **EMBD-01**: `tuwunel-embed` crate exists as a new workspace member with `EmbeddedHomeserver` struct
- [x] **EMBD-02**: `EmbeddedHomeserver::start(config)` starts a tuwunel server in-process and returns when ready
- [x] **EMBD-03**: `EmbeddedHomeserver::base_url()` returns the URL to connect to (with actual bound port)
- [x] **EMBD-04**: `EmbeddedHomeserver::stop()` performs graceful shutdown via the server's broadcast channel
- [x] **EMBD-05**: `EmbeddedHomeserver` uses an auto-provisioned tempdir for RocksDB (no manual path needed)
- [x] **EMBD-06**: Multiple `EmbeddedHomeserver` instances can run concurrently in the same process
- [x] **EMBD-07**: Tracing/logging initialization is guarded against double-registration panics
- [x] **EMBD-08**: OnceLock statics in runtime.rs are handled safely for embed use (no panic on re-init)
- [x] **EMBD-09**: `EmbeddedHomeserver::register_user()` convenience method registers a user via the Matrix API
- [x] **EMBD-10**: In-memory HTTP transport available via extracted axum Router (no TCP socket needed)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Federation

- **FED-01**: Two tuwunel instances running under Shadow can federate via server-server API
- **FED-02**: Users on different federated servers can exchange messages under Shadow
- **FED-03**: Federation works under degraded network conditions (high latency, packet loss)

### Advanced Scenarios

- **ADV-01**: Server restart during active client sync recovers gracefully under Shadow
- **ADV-02**: Network partition and reconnection recovery tested under Shadow
- **ADV-03**: Bandwidth-limited media upload tested under Shadow

## Out of Scope

| Feature | Reason |
|---------|--------|
| io_uring in Shadow builds | Incompatible with Shadow's LD_PRELOAD syscall interception |
| IPv6 in Shadow topologies | Shadow has no IPv6 support |
| Mocking Matrix spec behavior | Defeats the purpose — we test real tuwunel |
| Global shared test server | State leaks between tests; per-test isolation required |
| Docker-based test infrastructure | Lives in matrix-rust-client, not this repo |
| Modifying Matrix protocol behavior | Fork must be protocol-identical to upstream |
| UI/Wireshark integration interface | PCAP artifacts suffice; manual Wireshark usage |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| SHAD-01 | Phase 1 | Complete |
| SHAD-02 | Phase 1 | Complete |
| SHAD-03 | Phase 1 | Complete |
| SHAD-04 | Phase 1 | Complete |
| SHAD-05 | Phase 1 | Complete |
| SHAD-06 | Phase 1 | Complete |
| SHAD-07 | Phase 1 | Complete |
| SHAD-08 | Phase 1 | Complete |
| SHAD-09 | Phase 1 | Complete |
| CONF-01 | Phase 1 | Pending |
| CONF-02 | Phase 1 | Complete |
| CONF-03 | Phase 1 | Complete |
| TEST-01 | Phase 2 | Complete |
| TEST-02 | Phase 2 | Complete |
| TEST-03 | Phase 2 | Complete |
| TEST-04 | Phase 2 | Complete |
| TEST-05 | Phase 2 | Complete |
| TEST-06 | Phase 2 | Complete |
| E2EE-01 | Phase 2 | Pending |
| E2EE-02 | Phase 2 | Pending |
| E2EE-03 | Phase 2 | Pending |
| E2EE-04 | Phase 2 | Pending |
| E2EE-05 | Phase 2 | Complete |
| NET-01 | Phase 3 | Pending |
| NET-02 | Phase 3 | Pending |
| NET-03 | Phase 3 | Pending |
| NET-04 | Phase 3 | Pending |
| NET-05 | Phase 3 | Pending |
| LOAD-01 | Phase 3 | Complete |
| LOAD-02 | Phase 3 | Complete |
| LOAD-03 | Phase 3 | Complete |
| EMBD-01 | Phase 4 | Complete |
| EMBD-02 | Phase 4 | Complete |
| EMBD-03 | Phase 4 | Complete |
| EMBD-04 | Phase 4 | Complete |
| EMBD-05 | Phase 4 | Complete |
| EMBD-06 | Phase 4 | Complete |
| EMBD-07 | Phase 4 | Complete |
| EMBD-08 | Phase 4 | Complete |
| EMBD-09 | Phase 4 | Complete |
| EMBD-10 | Phase 4 | Complete |

**Coverage:**
- v1 requirements: 41 total
- Mapped to phases: 41
- Unmapped: 0

---
*Requirements defined: 2026-03-25*
*Last updated: 2026-03-25 after roadmap creation*
