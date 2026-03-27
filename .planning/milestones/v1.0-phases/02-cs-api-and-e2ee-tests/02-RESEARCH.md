# Phase 2: CS API and E2EE Tests - Research

**Researched:** 2026-03-25
**Domain:** Matrix Client-Server API testing, E2EE key exchange/SAS verification, matrix-sdk integration, Shadow multi-host topologies
**Confidence:** HIGH

## Summary

Phase 2 builds on Phase 1's Shadow infrastructure to add full Client-Server API tests (register, login, create room, send message, sync) and E2EE tests (key upload, key claim, encrypted messaging, SAS verification). The existing `matrix-test-client` binary gets new clap subcommands, each running a complete scenario. Two-client scenarios (alice and bob) use separate Shadow hosts with deterministic naming for coordination.

The primary technical challenge is integrating `matrix-sdk 0.16.0` into the test binary. matrix-sdk provides batteries-included E2EE support (Olm/Megolm via vodozemac), SAS verification, sync management, and transparent encryption -- avoiding the need to hand-roll any crypto operations. The SDK's `Client` type handles registration (via UIAA with `RegistrationToken`), login, room creation, message sending, and encrypted message exchange.

A secondary concern is dependency compatibility: matrix-sdk uses upstream `ruma/ruma` while the workspace uses `matrix-construct/ruma` fork. Since the test binary is a separate executable that never shares types with tuwunel crates, these coexist safely. However, workspace-level `[patch.crates-io]` entries (hyper-util, event-listener, etc.) apply to all members, so matrix-sdk's transitive dependencies will use the patched forks. This should work but needs build verification.

**Primary recommendation:** Add matrix-sdk 0.16.0 (with e2e-encryption, no sqlite default -- use feature flags for store selection) to the shadow-test-harness crate. Implement three new subcommands: `cs-api` (register/login/room/message/sync), `e2ee-messaging` (key upload/claim/encrypted exchange), and `sas-verify` (automated SAS verification). Each subcommand is self-contained. Two-client scenarios use Shadow start_time offsets (alice starts first, bob starts later) with bob polling for room existence.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Use matrix-sdk for all new test scenarios (CS API and E2EE). The SDK provides built-in E2EE, sync, and room management. Carries forward from Phase 1 D-01.
- **D-02:** Keep the existing smoke subcommand as raw reqwest -- it's a lightweight readiness check that doesn't need the full SDK. New subcommands use matrix-sdk.
- **D-03:** matrix-sdk version alignment with the workspace's ruma git fork is at Claude's discretion. May require a [patch] section, a compatible git rev, or a matrix-sdk fork. Researcher should evaluate compatibility.
- **D-04:** One subcommand per flow -- each subcommand runs a complete end-to-end scenario. e.g., `cs-api` runs register->login->room->message->sync; `e2ee-messaging` runs key upload->claim->encrypt->send; `sas-verify` runs SAS protocol.
- **D-05:** Each scenario is self-contained -- registers its own users, creates its own rooms. No shared state between processes, no setup phase.
- **D-06:** Results via exit code + stderr log, consistent with smoke subcommand. Exit 0 on full success, non-zero on first failure. Integration test reads Shadow's per-host stderr files for detailed assertions.
- **D-07:** Two test clients (alice and bob) run on separate Shadow hosts with their own virtual IPs. More realistic -- traffic goes through Shadow's network simulation.
- **D-08:** Deterministic naming for coordination -- alice always registers as @alice:tuwunel-server, bob as @bob:tuwunel-server. Room alias is pre-agreed (e.g., #test-room:tuwunel-server). Bob joins by alias. No runtime coordination needed.
- **D-09:** Timing between alice and bob is at Claude's discretion. Options include Shadow start_time offsets or bob polling for room existence. Researcher should evaluate based on how long operations take under simulated time.
- **D-10:** Support both in-memory and SQLite crypto stores. In-memory is the default for speed; SQLite in a tempdir is available as an option for more realistic testing. Both must work.
- **D-11:** E2EE key exchange orchestration approach is at Claude's discretion -- natural SDK sync loop vs explicit step-by-step. Researcher should evaluate based on matrix-sdk's API surface.
- **D-12:** SAS verification is automated -- both clients auto-accept the emoji match. Proves the protocol works end-to-end under Shadow without human interaction.

### Claude's Discretion
- matrix-sdk version alignment with ruma fork (D-03)
- Timing approach between alice and bob (D-09) -- start_time offset vs polling
- E2EE key exchange orchestration (D-11) -- SDK sync loop vs explicit steps
- Shadow stop_time values for each scenario -- researcher evaluates based on operation durations

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEST-01 | User can register an account via the Matrix registration API under Shadow | matrix-sdk `Client::register()` with UIAA `RegistrationToken` auth; tuwunel config `allow_registration = true` + `registration_token` |
| TEST-02 | User can login with username/password and receive an access token under Shadow | matrix-sdk `client.matrix_auth().login_username(user, password).send().await` |
| TEST-03 | User can create a room and receive a room_id under Shadow | matrix-sdk `client.create_room(request)` with room alias in creation request |
| TEST-04 | User can send a text message and another user receives it via sync | matrix-sdk `room.send(RoomMessageEventContent::text_plain(...))` + `client.sync_once()` on receiver |
| TEST-05 | Two clients on separate Shadow hosts can exchange messages through the server | Shadow multi-host topology with alice-host and bob-host on separate network_node_ids; D-07/D-08 naming conventions |
| TEST-06 | Test results integrate with cargo test -- Shadow exit codes map to pass/fail | Existing `run_shadow()` + `ShadowResult::success()` pattern; new integration test files per scenario |
| E2EE-01 | User can upload device keys and one-time keys to the server | matrix-sdk handles key upload automatically after login when e2e-encryption feature is enabled |
| E2EE-02 | User can claim another user's one-time keys for Olm session | matrix-sdk handles key claim transparently when sending first encrypted message to a user |
| E2EE-03 | Two users can exchange encrypted messages in an E2EE room | `room.enable_encryption().await` + `room.send(content)` -- SDK transparently encrypts |
| E2EE-04 | E2EE key exchange completes deterministically without timing-dependent retry | Shadow's deterministic simulated time + polling sync loop; no wall-clock sleeps needed |
| E2EE-05 | SAS verification between two devices completes under Shadow | matrix-sdk `SasVerification` API: accept -> keys_exchanged -> confirm flow, auto-accepting emoji match |
</phase_requirements>

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| matrix-sdk | 0.16.0 | Matrix client with E2EE, sync, room management | Locked decision D-01; batteries-included E2EE via vodozemac; handles UIAA registration, key upload/claim, SAS verification |
| matrix-sdk-sqlite | 0.16.0 | SQLite crypto store backend | D-10 requires SQLite option; matrix-sdk feature flag `sqlite` |
| clap | 4.5 (workspace) | CLI subcommands for test scenarios | Already in workspace; established pattern from Phase 1 |
| tokio | 1.50 (workspace) | Async runtime for matrix-sdk | Already in workspace; matrix-sdk requires it |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| reqwest | 0.13 (workspace) | Existing smoke test HTTP client | Keep for smoke subcommand only (D-02) |
| serde_yaml | 0.9 (workspace) | Shadow YAML generation | Reuse from Phase 1 for new scenario configs |
| serde_json | 1.0 (workspace) | JSON parsing for assertions | Reuse from Phase 1 |
| tempfile | 3.x | Temp directories for SQLite crypto store | Already in test crate |
| tracing-subscriber | 0.3 | Structured logging in test client | Better debugging output than raw eprintln |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| matrix-sdk for registration | Raw reqwest POST to `/_matrix/client/v3/register` | Simpler but must hand-roll UIAA flow; D-01 locks matrix-sdk |
| matrix-sdk E2EE | Raw ruma types + manual Olm/Megolm | Extremely complex; defeats purpose of using SDK |
| In-memory crypto store | Always SQLite | In-memory is faster for CI; SQLite available for realism |

**Installation:**
```bash
# In tests/shadow/Cargo.toml, add:
matrix-sdk = { version = "0.16", default-features = false, features = ["e2e-encryption", "rustls-tls"] }
# For SQLite crypto store (optional feature):
matrix-sdk = { version = "0.16", default-features = false, features = ["e2e-encryption", "rustls-tls", "sqlite"] }
```

**Version verification:** matrix-sdk 0.16.0 is the latest release on crates.io (published 2025-12-04). It depends on ruma from `ruma/ruma` git (not `matrix-construct/ruma`), but this is acceptable because the test binary is a separate executable.

## Architecture Patterns

### Recommended Project Structure
```
tests/shadow/
    Cargo.toml              # Add matrix-sdk dependency
    src/
        lib.rs              # Re-export modules
        config/
            mod.rs
            shadow.rs       # Extended for multi-host topologies
            tuwunel.rs      # Extended with encryption config fields
        runner.rs           # Reuse existing Shadow runner
        scenarios/          # NEW: scenario implementations
            mod.rs
            cs_api.rs       # Register/login/room/message/sync flow
            e2ee_msg.rs     # Key upload/claim/encrypted messaging
            sas_verify.rs   # SAS verification automation
            common.rs       # Shared helpers (wait_for_server, create_sdk_client)
    src/bin/
        matrix_test_client.rs  # Extended with new subcommands
    tests/
        smoke.rs            # Existing (Phase 1)
        cs_api.rs           # NEW: CS API integration test
        e2ee.rs             # NEW: E2EE integration test
        sas_verify.rs       # NEW: SAS verification integration test
        common/             # NEW: shared test setup helpers
            mod.rs          # build_shadow_binaries, create_multi_host_config
```

### Pattern 1: SDK Client Construction with Server Readiness
**What:** Build a matrix-sdk `Client` pointing at the Shadow-hosted tuwunel server, with TLS disabled and retry polling for readiness.
**When to use:** Every new subcommand.
**Example:**
```rust
use matrix_sdk::{Client, config::RequestConfig};
use std::time::Duration;

async fn create_client(server_url: &str) -> Result<Client, Box<dyn std::error::Error>> {
    // Wait for server readiness first (reuse smoke pattern)
    wait_for_server(server_url, 60, 500).await?;

    let client = Client::builder()
        .homeserver_url(server_url)
        .disable_ssl_verification()
        .request_config(
            RequestConfig::default()
                .timeout(Duration::from_secs(10))
        )
        .build()
        .await?;

    Ok(client)
}
```

### Pattern 2: Registration with Token via UIAA
**What:** Register a user with matrix-sdk using the registration token UIAA flow.
**When to use:** Every scenario that creates users.
**Example:**
```rust
use matrix_sdk::ruma::api::client::{
    account::register::v3::Request as RegistrationRequest,
    uiaa::{AuthData, RegistrationToken},
};

async fn register_user(
    client: &Client,
    username: &str,
    password: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut request = RegistrationRequest::new();
    request.username = Some(username.to_owned());
    request.password = Some(password.to_owned());

    // First attempt triggers UIAA
    let response = client.register(request.clone()).await;

    // Handle UIAA: provide registration token
    if let Err(e) = response {
        // Extract session from UIAA response, retry with token auth
        let auth = AuthData::RegistrationToken(
            RegistrationToken::new(token.to_owned())
        );
        request.auth = Some(auth);
        client.register(request).await?;
    }

    Ok(())
}
```
**Note:** The exact UIAA error handling may need adjustment based on how matrix-sdk surfaces 401 responses. The pattern is: first request -> 401 with session -> second request with auth data including session.

### Pattern 3: Two-Client Shadow Topology
**What:** Shadow config with server + alice-host + bob-host, each on separate virtual IPs.
**When to use:** TEST-05, E2EE-03, E2EE-05.
**Example:**
```rust
// Three hosts: server (node 0), alice (node 0), bob (node 0)
// All on same network switch. Shadow assigns virtual IPs:
// tuwunel-server -> 11.0.0.1
// alice-host -> 11.0.0.2
// bob-host -> 11.0.0.3
let mut hosts = BTreeMap::new();

hosts.insert("tuwunel-server".to_owned(), Host {
    network_node_id: 0,
    processes: vec![server_process],
    ..
});

hosts.insert("alice-host".to_owned(), Host {
    network_node_id: 0,
    processes: vec![Process {
        path: client_bin.to_str().to_owned(),
        args: Some("cs-api --server-url http://tuwunel-server:8448 --role alice".to_owned()),
        start_time: Some("5s".to_owned()),  // After server starts
        expected_final_state: Some("exited".to_owned()),
        ..
    }],
    ..
});

hosts.insert("bob-host".to_owned(), Host {
    network_node_id: 0,
    processes: vec![Process {
        path: client_bin.to_str().to_owned(),
        args: Some("cs-api --server-url http://tuwunel-server:8448 --role bob".to_owned()),
        start_time: Some("8s".to_owned()),  // After alice creates room
        expected_final_state: Some("exited".to_owned()),
        ..
    }],
    ..
});
```

### Pattern 4: E2EE Flow via SDK Natural Sync
**What:** Use matrix-sdk's natural sync loop for E2EE key management. The SDK automatically uploads device keys on first sync, handles key claim on first encrypted message, and transparently encrypts/decrypts.
**When to use:** E2EE-01 through E2EE-04.
**Example:**
```rust
// Alice: create encrypted room, send message
let room = alice_client.create_room(create_request).await?;
room.enable_encryption().await?;
// SDK sync distributes keys
alice_client.sync_once(SyncSettings::default()).await?;
room.send(RoomMessageEventContent::text_plain("secret message")).await?;

// Bob: join room, sync to receive encrypted message
let room = bob_client.join_room_by_id_or_alias(room_alias, &[]).await?;
// SDK handles key claim and Megolm session setup
bob_client.sync_once(SyncSettings::default()).await?;
// Message is automatically decrypted in sync response
```

### Pattern 5: Automated SAS Verification
**What:** Both clients register event handlers that auto-accept verification requests and auto-confirm emoji matches.
**When to use:** E2EE-05.
**Example:**
```rust
use matrix_sdk::encryption::verification::{
    SasVerification, SasState, VerificationRequest,
};

// Register handler to auto-accept and auto-confirm
client.add_event_handler(|ev: ToDeviceKeyVerificationRequestEvent, client: Client| async move {
    let request = client.encryption()
        .get_verification_request(&ev.sender, &ev.content.transaction_id)
        .await;
    if let Some(request) = request {
        request.accept().await.ok();
        // Transition to SAS
        if let Some(sas) = request.start_sas().await.ok().flatten() {
            sas.accept().await.ok();
            // Wait for keys exchanged state
            let mut stream = sas.changes();
            while let Some(state) = stream.next().await {
                match state {
                    SasState::KeysExchanged { .. } => {
                        // Auto-confirm (D-12: no human interaction)
                        sas.confirm().await.ok();
                    }
                    SasState::Done { .. } => break,
                    SasState::Cancelled(_) => break,
                    _ => {}
                }
            }
        }
    }
});
```

### Anti-Patterns to Avoid
- **Sharing state between alice and bob processes:** These are separate OS processes under Shadow. No shared memory, no IPC. Coordinate only via Matrix protocol (room aliases, sync).
- **Using wall-clock sleeps for timing:** `tokio::time::sleep` advances Shadow simulated time. Never use `std::thread::sleep` -- Shadow cannot intercept it.
- **Manually implementing Olm/Megolm:** matrix-sdk handles all crypto transparently. Do not import vodozemac directly.
- **Starting all processes at the same Shadow time:** Server needs startup time. Alice needs server ready. Bob needs alice to have created the room. Use staggered start_time values.
- **Using multi-threaded tokio runtime:** Shadow has quirks with multi-threaded runtimes. Use `new_current_thread()` as established in Phase 1 smoke test.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| UIAA registration flow | Manual HTTP requests with session tracking | matrix-sdk `Client::register()` | UIAA is a multi-step stateful protocol; SDK handles it |
| E2EE key upload/claim | Manual POST to `/_matrix/client/v3/keys/upload` | matrix-sdk automatic key management | Key management has complex state machine; SDK syncs keys automatically |
| Olm session establishment | Manual one-time key claim + session creation | matrix-sdk transparent encryption | Olm has complex ratchet state; SDK handles session lifecycle |
| Megolm key rotation | Manual outbound/inbound session management | matrix-sdk room encryption | Rotation rules (100 msgs / 1 week) are built into SDK |
| SAS verification protocol | Manual m.key.verification.* event exchange | matrix-sdk `SasVerification` API | Multi-step protocol with state machine; SDK provides stream-based API |
| Sync token management | Manual `since` token tracking | matrix-sdk `sync_once()` / `sync()` | SDK manages sync tokens, timeline, and state updates |

**Key insight:** matrix-sdk exists precisely to avoid hand-rolling Matrix protocol complexity. Every requirement in this phase maps to a high-level SDK method.

## Common Pitfalls

### Pitfall 1: matrix-sdk Dependency Conflicts with Workspace Patches
**What goes wrong:** Adding matrix-sdk to the workspace may cause compilation errors because workspace `[patch.crates-io]` entries (hyper-util, event-listener, async-channel) override matrix-sdk's transitive dependencies with matrix-construct forks.
**Why it happens:** Cargo applies workspace patches to ALL workspace members. matrix-sdk depends on hyper, which depends on hyper-util. The patched hyper-util fork may have API differences.
**How to avoid:** Build-test the matrix-sdk addition immediately after adding it to Cargo.toml. If patches cause issues, options: (a) move shadow-test-harness out of the workspace, (b) use matrix-sdk as a git dependency with a commit compatible with the patched versions, (c) conditionalize patches.
**Warning signs:** Compilation errors in matrix-sdk or its transitive deps mentioning type mismatches or missing methods.

### Pitfall 2: UIAA Registration Requires Two-Step Flow
**What goes wrong:** First registration attempt returns 401 (UIAA required) which looks like a failure.
**Why it happens:** Tuwunel requires registration_token. The UIAA protocol mandates: (1) first request without auth -> server returns 401 with session + available flows, (2) second request with auth data including session and token.
**How to avoid:** Handle the UiaaResponse error from the first register call, extract the session, and retry with `RegistrationToken` auth data including the session.
**Warning signs:** "User registration failed: 401 Unauthorized" in test client stderr.

### Pitfall 3: Room Alias Timing Between Alice and Bob
**What goes wrong:** Bob tries to join `#test-room:tuwunel-server` before alice has created it, getting a 404.
**Why it happens:** Separate Shadow processes start at specified times. If bob starts too early, the room doesn't exist yet.
**How to avoid:** Two complementary strategies: (1) Use Shadow start_time offset to give alice a head start (e.g., alice at 5s, bob at 15s). (2) Bob's code polls room alias with retry loop (same pattern as server readiness check).
**Warning signs:** "Room not found" or 404 errors in bob's stderr.

### Pitfall 4: E2EE Key Upload Timing
**What goes wrong:** Alice sends an encrypted message before bob has uploaded device keys, so the message can't be encrypted for bob's device.
**Why it happens:** matrix-sdk uploads keys during sync. If alice sends before bob has synced, bob's keys aren't on the server yet.
**How to avoid:** Both alice and bob must complete at least one sync before alice sends encrypted messages. The flow should be: alice creates room -> alice syncs -> bob joins -> bob syncs -> alice syncs again (to see bob) -> alice sends encrypted message -> bob syncs (receives).
**Warning signs:** "Unable to encrypt message: missing device keys for user" or undecryptable messages.

### Pitfall 5: Shadow stop_time Too Short for E2EE
**What goes wrong:** Shadow simulation ends before the full E2EE flow completes.
**Why it happens:** E2EE involves multiple HTTP round trips: key upload (2 per user), room creation, room join, key claim, message send, sync. Each round trip takes simulated time. SAS verification adds even more steps.
**How to avoid:** Start with generous stop_time values: 60s for CS API, 120s for E2EE messaging, 180s for SAS verification. Tune down after empirical testing shows actual durations.
**Warning signs:** Shadow exits with processes still in "running" state; incomplete test output in stderr files.

### Pitfall 6: Single-Threaded Runtime with matrix-sdk Sync
**What goes wrong:** matrix-sdk's `sync()` method blocks the current task, preventing other work.
**Why it happens:** Phase 1 uses `new_current_thread()` runtime. `sync()` is a long-running future.
**How to avoid:** Use `sync_once()` for discrete sync operations rather than the infinite `sync()` loop. Or use `tokio::spawn` with `sync()` on a background task and `sync_once()` for foreground operations.
**Warning signs:** Test client appears to hang after calling `sync()`.

### Pitfall 7: matrix-sdk Default Features Pull In sqlite
**What goes wrong:** matrix-sdk's default features include `sqlite`, which adds libsqlite3 build dependency.
**Why it happens:** Default features: `["e2e-encryption", "automatic-room-key-forwarding", "sqlite", "rustls-tls"]`.
**How to avoid:** Use `default-features = false` and explicitly enable needed features. For in-memory crypto store, only need `["e2e-encryption", "rustls-tls"]`. Add `"sqlite"` feature when testing SQLite crypto store.
**Warning signs:** Build errors about missing sqlite3 dev libraries.

## Code Examples

### Registration with Token (UIAA Flow)
```rust
// Source: matrix-sdk docs + ruma UIAA types
use matrix_sdk::{Client, ruma::api::client::account::register::v3::Request as RegRequest};

async fn register_with_token(
    client: &Client,
    username: &str,
    password: &str,
    token: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use matrix_sdk::ruma::api::client::uiaa;

    let mut req = RegRequest::new();
    req.username = Some(username.to_owned());
    req.password = Some(password.to_owned());

    // Step 1: Initial request (may succeed or return UIAA 401)
    match client.register(req.clone()).await {
        Ok(_) => return Ok(()),
        Err(e) => {
            // Extract session from UIAA response
            // matrix-sdk wraps this in HttpError::UiaaError
            let session = extract_uiaa_session(&e)?;
            let mut token_auth = uiaa::RegistrationToken::new(token.to_owned());
            token_auth.session = Some(session);
            req.auth = Some(uiaa::AuthData::RegistrationToken(token_auth));
            client.register(req).await?;
        }
    }
    Ok(())
}
```

### Two-Client CS API Flow (Alice Role)
```rust
// Source: matrix-sdk Client API
async fn run_cs_api_alice(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = create_client(server_url).await?;

    // TEST-01: Register
    register_with_token(&client, "alice", "alice_pass", "shadow_test_token").await?;
    eprintln!("Alice registered");

    // TEST-02: Login
    client.matrix_auth()
        .login_username("alice", "alice_pass")
        .send().await?;
    eprintln!("Alice logged in");

    // TEST-03: Create room with alias
    use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
    let mut create_req = CreateRoomRequest::new();
    create_req.room_alias_name = Some("test-room".to_owned());
    let room = client.create_room(create_req).await?;
    eprintln!("Alice created room: {}", room.room_id());

    // TEST-04: Send message
    use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
    room.send(RoomMessageEventContent::text_plain("Hello from Alice")).await?;
    eprintln!("Alice sent message");

    Ok(())
}
```

### Two-Client CS API Flow (Bob Role)
```rust
async fn run_cs_api_bob(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = create_client(server_url).await?;

    register_with_token(&client, "bob", "bob_pass", "shadow_test_token").await?;
    client.matrix_auth()
        .login_username("bob", "bob_pass")
        .send().await?;
    eprintln!("Bob logged in");

    // TEST-05: Join room by alias (with retry for timing)
    use matrix_sdk::ruma::RoomOrAliasId;
    let alias = <&RoomOrAliasId>::try_from("#test-room:tuwunel-server")?;

    let room = retry_join(|| client.join_room_by_id_or_alias(alias, &[]), 30, 1000).await?;
    eprintln!("Bob joined room");

    // Sync to receive messages
    use matrix_sdk::config::SyncSettings;
    let response = client.sync_once(SyncSettings::default()).await?;
    eprintln!("Bob synced, checking for messages...");

    // Verify message received (check room timeline)
    // The sync response contains timeline events for joined rooms
    Ok(())
}
```

### E2EE Encrypted Message Exchange
```rust
async fn run_e2ee_alice(server_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .homeserver_url(server_url)
        .disable_ssl_verification()
        .build().await?;

    register_with_token(&client, "alice", "alice_pass", "shadow_test_token").await?;
    client.matrix_auth().login_username("alice", "alice_pass").send().await?;

    // E2EE-01: First sync uploads device keys automatically
    client.sync_once(SyncSettings::default()).await?;
    eprintln!("Alice keys uploaded");

    // Create encrypted room
    let room = client.create_room(create_req).await?;
    room.enable_encryption().await?;
    client.sync_once(SyncSettings::default()).await?;

    // Wait for bob to join (poll room members)
    // ...

    // E2EE-03: Send encrypted message (SDK handles Megolm transparently)
    room.send(RoomMessageEventContent::text_plain("encrypted secret")).await?;
    eprintln!("Alice sent encrypted message");

    Ok(())
}
```

## Discretion Recommendations

### D-03: matrix-sdk Version Alignment (Recommendation: Use crates.io 0.16.0)
**Evidence:** matrix-sdk 0.16.0 uses upstream `ruma/ruma` git dependency. The workspace uses `matrix-construct/ruma` fork. These are separate crates in the dependency graph because they come from different git sources. Since the test binary never passes ruma types to tuwunel code (they are separate executables), there is no type incompatibility.
**Risk:** Workspace `[patch.crates-io]` entries may affect matrix-sdk's transitive deps (hyper-util, event-listener). Mitigation: build-test immediately.
**Recommendation:** Start with crates.io `matrix-sdk = "0.16"` with `default-features = false`. If workspace patches cause build failures, fall back to adding the test crate to a separate workspace or using `[patch]` overrides.
**Confidence:** MEDIUM -- the approach is sound but workspace patch interaction needs empirical verification.

### D-09: Alice/Bob Timing (Recommendation: Start_time Offset + Bob Polling)
**Evidence:** Phase 1 smoke test uses start_time: "5s" for the client (after server at "1s"). In simulated time, registration + room creation takes multiple HTTP round trips. Conservative estimate: 5-10 seconds of simulated time for alice to complete setup.
**Recommendation:** Alice starts at 5s (after server readiness). Bob starts at 15s. Additionally, bob's code includes a retry loop for joining the room by alias (same polling pattern as server readiness). This provides defense in depth: start_time offset handles the typical case, polling handles edge cases where alice is slow.
**Confidence:** HIGH -- this pattern is proven in Phase 1 and defensive polling costs nothing under simulated time.

### D-11: E2EE Orchestration (Recommendation: SDK Natural Sync)
**Evidence:** matrix-sdk automatically uploads device keys during sync, automatically claims one-time keys when sending to a new device, and transparently encrypts/decrypts room messages. Manually calling key management APIs is unnecessary and error-prone.
**Recommendation:** Use `client.sync_once()` at strategic points to let the SDK manage keys. The flow: login -> sync (uploads keys) -> create/join room -> enable encryption -> sync -> send message -> sync (receive). Each `sync_once` call drives the SDK's internal state machine forward.
**Confidence:** HIGH -- this is the intended usage pattern per matrix-sdk documentation.

### Shadow stop_time Values (Recommendation: Generous Defaults)
**Evidence:** Phase 1 smoke test uses 30s for a single GET request with retry. CS API flow has ~10 HTTP round trips. E2EE adds key management overhead. SAS verification has ~8 protocol messages.
**Recommendation:**
- CS API scenario (single client): 60s
- CS API two-client: 90s
- E2EE messaging: 120s
- SAS verification: 180s

Tune down after empirical testing. Under simulated time, larger stop_time does NOT mean longer wall-clock test duration -- it just sets the maximum simulation window.
**Confidence:** MEDIUM -- values are estimates. Empirical tuning needed.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| matrix-sdk 0.7-0.9 with separate Room types (Joined, Left, Invited) | matrix-sdk 0.16 with unified Room type | 2024-2025 | Single Room struct with state methods; no Joined/Left/Invited wrappers |
| Manual olm/megolm via libolm | vodozemac (pure Rust) via matrix-sdk | 2023+ | No C dependency; matrix-sdk bundles it |
| matrix-sdk MemoryStore for state | Default persistent store | 0.16 | Explicit opt-in for in-memory; sqlite is default |

**Deprecated/outdated:**
- `matrix-sdk 0.2-0.3` had `Joined` room type -- removed in favor of unified `Room`
- `libolm` C library -- replaced by `vodozemac` pure Rust implementation

## Open Questions

1. **Workspace patch compatibility with matrix-sdk**
   - What we know: Workspace patches (hyper-util, event-listener, async-channel) apply to all members
   - What's unclear: Whether matrix-sdk's transitive deps compile correctly with these patched forks
   - Recommendation: First task should be adding matrix-sdk to Cargo.toml and verifying build succeeds. If it fails, extract test crate to separate workspace.

2. **UIAA error extraction from matrix-sdk**
   - What we know: matrix-sdk returns an error when UIAA is required (401 response)
   - What's unclear: Exact error type and how to extract the session from the UIAA response in matrix-sdk 0.16
   - Recommendation: Check matrix-sdk's `HttpError::UiaaError` variant during implementation. May need to match on error type.

3. **matrix-sdk behavior under Shadow's simulated time**
   - What we know: tokio::time::sleep works under Shadow (proven in Phase 1). matrix-sdk uses tokio internally.
   - What's unclear: Whether matrix-sdk's internal timeouts and retry logic interact well with simulated time
   - Recommendation: Use generous timeouts (10+ seconds) and monitor for hangs. The single-threaded runtime is key.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Shadow | All scenarios | Yes | installed at ~/.local/bin/shadow | -- |
| Rust nightly | Build | Yes | 1.94.0 | -- |
| CMake | Shadow (if rebuild needed) | Yes | 3.28.3 | -- |
| SQLite3 dev libs | matrix-sdk sqlite feature | Unknown | -- | Use in-memory store only; add sqlite as optional |

**Missing dependencies with no fallback:**
- None identified

**Missing dependencies with fallback:**
- SQLite3 dev libraries: may be needed for matrix-sdk sqlite feature. Fallback: disable sqlite feature, use in-memory crypto store only for initial implementation.

## TuwunelConfig Extensions

The existing `TuwunelConfig` struct needs additional fields for E2EE scenarios:

```rust
pub struct TuwunelGlobal {
    // ... existing fields ...
    pub allow_encryption: bool,  // default: true (already tuwunel default)
    // No additional E2EE config needed -- tuwunel enables encryption by default
}
```

Tuwunel's `allow_encryption` defaults to `true`, so no special server configuration is needed for E2EE tests. The server already supports key upload, key claim, and encrypted message relay out of the box.

## Sources

### Primary (HIGH confidence)
- matrix-sdk 0.16.0 crates.io listing -- version and publish date verified
- [matrix-sdk docs.rs](https://docs.rs/matrix-sdk/0.16.0/matrix_sdk/) -- Client, encryption, SyncSettings API
- [matrix-sdk SAS verification](https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk/encryption/verification/struct.SasVerification.html) -- SAS API methods
- [matrix-sdk emoji verification example](https://matrix-org.github.io/matrix-rust-sdk/src/example_emoji_verification/main.rs.html) -- automated SAS flow
- [matrix-sdk encryption module](https://docs.rs/matrix-sdk/0.16.0/matrix_sdk/encryption/index.html) -- E2EE architecture
- Phase 1 artifacts (existing code in tests/shadow/) -- foundation patterns
- tuwunel-example.toml and src/core/config/mod.rs -- server config options

### Secondary (MEDIUM confidence)
- [matrix-rust-sdk Cargo.toml](https://github.com/matrix-org/matrix-rust-sdk/blob/main/Cargo.toml) -- ruma dependency (git rev, not crates.io)
- [registration v3 Request](https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk_base/ruma/api/client/account/register/v3/struct.Request.html) -- UIAA registration fields
- [RegistrationToken UIAA](https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk/ruma/api/client/uiaa/struct.RegistrationToken.html) -- token auth data

### Tertiary (LOW confidence)
- Stop_time estimates -- based on theoretical round-trip counts, not empirical measurement
- Workspace patch interaction -- needs empirical build verification

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - matrix-sdk 0.16.0 is the correct tool, version verified on crates.io
- Architecture: HIGH - extends proven Phase 1 patterns, SDK API surface documented
- Pitfalls: HIGH - identified from Phase 1 experience, SDK documentation, and workspace analysis
- Discretion areas: MEDIUM - recommendations are well-reasoned but some need empirical validation

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (matrix-sdk is stable; 30-day window appropriate)
