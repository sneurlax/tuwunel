# Codebase Concerns

**Analysis Date:** 2026-03-25

## Tech Debt

**ABA Race Condition in Device Counter Increment:**
- Issue: `increment()` does a non-atomic read-modify-write: reads old value, computes new, writes back, with no CAS or lock. This is a classic ABA race under concurrent device updates.
- Files: `src/service/users/device.rs:452`
- Impact: Device list version counter can diverge under high concurrency; clients may miss key updates.
- Fix approach: Replace with a RocksDB `merge_operator` that atomically increments, or use a Tokio mutex around the read-modify-write pair.

**One-Time Keys Not Removed on Device Removal:**
- Issue: When a device is removed, its one-time keys are left in the database.
- Files: `src/service/users/device.rs:102`
- Impact: Key material leaks in the database; not exploitable remotely but wastes storage and violates spec.
- Fix approach: Iterate and delete entries under the `userid_onetimekeys` prefix for the removed device.

**3PID Not Unhooked on Account Deactivation:**
- Issue: Account deactivation sets the password to `""` and removes devices but does not unlink third-party identifiers (email/phone) from the identity server.
- Files: `src/service/users/mod.rs:149`
- Impact: Deactivated accounts may still receive Matrix invites via email. Spec §10.1 requires notifying the identity server.
- Fix approach: Call the identity server unbind API for each 3PID associated with the account on deactivation.

**No Appservice Namespace Collision Checking:**
- Issue: When registering an appservice, exclusive namespace conflicts with existing appservices are not validated.
- Files: `src/service/appservice/mod.rs:128`
- Impact: Two appservices could claim the same user/room namespace, causing ambiguous routing.
- Fix approach: After loading existing registrations, check new namespaces against all `exclusive: true` regexes.

**SSO UIAA Flow is Dead Code:**
- Issue: `has_sso` is hardcoded to `false` even though OAuth session detection is implemented in `_has_sso`. The SSO re-authentication flow via UIAA is therefore never exposed.
- Files: `src/api/router/auth/uiaa.rs:37-48`
- Impact: Clients using SSO cannot complete UIAA-gated operations (e.g., device deletion, password change) as per spec.
- Fix approach: Replace `has_sso = false` with `has_sso = _has_sso` once the UIAA SSO spec path is understood.

**Pusher `send_pdu_push` Called with `.expect()` Instead of Future:**
- Issue: `send_pdu_push()` is called inside a `ready_for_each` closure with `.expect("TODO: replace with future")`. This panics if the push queue is full rather than propagating backpressure.
- Files: `src/service/pusher/append.rs:142`
- Impact: A full push queue crashes the push dispatch path.
- Fix approach: Refactor to use an `async` stream combinator (`for_each_concurrent` or `try_for_each`) so errors propagate correctly.

**Push Rules `append` Mode Unhandled:**
- Issue: The `PUT /_matrix/client/r0/pushrules/` endpoint documents that it should handle `append` but does not.
- Files: `src/api/client/push.rs:583`
- Impact: Clients expecting `append` semantics when adding push rules will silently get incorrect behavior.

**Redaction Transaction ID Idempotency Missing:**
- Issue: `PUT /_matrix/client/v3/rooms/{roomId}/redact/{eventId}/{txnId}` does not handle the transaction ID for idempotency.
- Files: `src/api/client/redact.rs:13`
- Impact: Retried redaction requests from clients can produce duplicate redaction events.

**Third-Party Protocol Endpoint Stub:**
- Issue: `GET /_matrix/client/r0/thirdparty/protocols` always returns an empty object with no implementation.
- Files: `src/api/client/thirdparty.rs`
- Impact: Bridges relying on this endpoint for protocol negotiation will fail or silently misbehave.

**Email Push Notifications Not Implemented:**
- Issue: Two separate places in the pusher explicitly note email push handling is TODO.
- Files: `src/service/pusher/send.rs:88`, `src/service/pusher/send.rs:189`
- Impact: Push gateways that request email delivery silently drop notifications.

**To-Device Messages Not Sent to Appservices:**
- Issue: The appservice event push path sends an empty `to_device` list.
- Files: `src/service/sending/sender.rs:767`
- Impact: Appservices do not receive to-device messages, breaking bridges that rely on them (e.g., E2EE bridging).

**Module Management Incomplete:**
- Issue: Dynamically loaded modules (the `tuwunel_mods` feature) store instances in a plain `Vec` on `Server`. The TODO notes that proper lifecycle management via a `mods::loaded` vector is pending.
- Files: `src/main/server.rs:26`
- Impact: No formal unload ordering or reload coordination; hot-reload is risky without it.

**`BadRequest` Error Variant Pending Removal:**
- Issue: `Error::BadRequest` is marked `//TODO: remove` but still used throughout the codebase.
- Files: `src/core/error/mod.rs:109`
- Impact: Inconsistent error construction; some callers use this variant while new code uses the `err!` macro, making the error path harder to audit.

## Known Bugs

**`de_array_integer` Deserialization Broken:**
- Symptoms: Deserialization of `ArrayVec<u64, N>` from byte arrays fails.
- Files: `src/database/tests.rs:487`, `src/database/tests.rs:606`
- Trigger: `#[ignore = "does not work yet. TODO! Fixme!"]` — test is disabled rather than fixed.
- Workaround: None known; avoid serializing bare integer arrays in column family keys.

**Media Tests Module Disabled:**
- Symptoms: The entire media service test module is compiled out with `#[cfg(disable)]`.
- Files: `src/service/media/tests.rs:4`
- Trigger: All test helper methods are `todo!()` stubs.
- Workaround: No unit-level coverage of media upload/download/delete logic.

**`unimplemented!` on Unrecognized `RawId` Length:**
- Symptoms: Any database key with an unrecognized byte length panics at runtime.
- Files: `src/core/matrix/pdu/raw_id.rs:146`
- Trigger: Corrupt or future-format database entries with unexpected key lengths.
- Workaround: Stable database format prevents this in practice; upgrade paths could trigger it.

**`unimplemented!` on Unsupported HTTP Methods in Router:**
- Symptoms: Any HTTP method not in the router's match arm causes a panic instead of returning `405 Method Not Allowed`.
- Files: `src/api/router/handler.rs:85`
- Trigger: HTTP client sending an unusual method to a registered route.
- Workaround: Unlikely in practice but should return an error response.

## Security Considerations

**`allow_invalid_tls_certificates` Option:**
- Risk: A config flag completely disables TLS certificate validation for all outgoing HTTP requests (federation, push, appservice).
- Files: `src/core/config/mod.rs:2218`, `src/service/client/mod.rs:171`
- Current mitigation: Config docs warn against it; config check warns if the example emergency password is in use.
- Recommendations: Add a startup warning to logs that prominently identifies this as an active security risk; consider restricting to development builds only.

**JWT Signature Validation Can Be Disabled:**
- Risk: `insecure_disable_signature_validation()` is called when JWT config lacks a secret/key, effectively accepting any JWT. This appears intentional for testing but could misconfigure production.
- Files: `src/api/client/session/jwt.rs:119`
- Current mitigation: Feature-gated behind `jwt.enable` in config.
- Recommendations: Require a non-empty key when `jwt.enable = true`; refuse to start if the key is absent rather than silently disabling validation.

**Cross-Signing Key Signatures Not Verified:**
- Risk: `add_cross_signing_keys` stores master, self-signing, and user-signing keys without verifying signatures.
- Files: `src/service/users/keys.rs:229`
- Current mitigation: None; any client can submit arbitrary cross-signing keys.
- Recommendations: Validate that the master key signs the self-signing and user-signing keys per spec before persisting.

**`server_name` Cannot Be Changed Without Database Wipe:**
- Risk: If `server_name` is mis-configured and then corrected, all existing data (user accounts, room memberships, event IDs) becomes orphaned and the database must be wiped.
- Files: `src/core/config/mod.rs:75`
- Current mitigation: Documentation warning in config field.
- Recommendations: Add a startup check that reads the stored server name from the database and aborts with a clear error if it differs from config, preventing silent corruption.

**`unsafe impl Send/Sync` for Raw Pointer State:**
- Risk: `State` contains a `*const Services` raw pointer and manually asserts `Send + Sync`. If `Services` is dropped before all in-flight requests complete, this becomes use-after-free.
- Files: `src/api/router/state.rs:49-54`
- Current mitigation: Detailed safety comment explains the invariant; `Guard` holds the extra `Arc` reference. However, the safety relies on correct drop ordering at shutdown, which is not enforced by the type system.
- Recommendations: Attempt to encode the lifetime constraint via a `'static` reference or `Pin` rather than raw pointers, or add a canary assertion in tests that verifies shutdown ordering.

**Unsafe `mem::transmute` for Config Lifetime Extension:**
- Risk: `Manager::load()` uses `mem::transmute` to extend a thread-local `Arc<Config>` reference to `'static`. If a thread holds two simultaneous config references and a reload occurs, the first reference can dangle.
- Files: `src/core/config/manager.rs:101`, `src/core/config/manager.rs:127`
- Current mitigation: Safety comment notes the constraint; `HISTORY` buffer of 8 old configs makes the window "astronomical."
- Recommendations: Track this with a Miri or loom test to confirm absence of UB; document calling conventions for all config access sites.

**Unvalidated UIAA Session Error Handling:**
- Risk: UIAA session management notes "why is uiaainfo.session optional" — session handling may silently accept requests with missing session tokens.
- Files: `src/service/uiaa/mod.rs:59`

## Performance Bottlenecks

**Key Watch in Sync Broadcasts to All Devices:**
- Problem: The sync watch sends device key change events for *any* key change by the user, regardless of whether the watching user shares a room with the changed user.
- Files: `src/service/sync/watch.rs:70`
- Cause: Lack of per-room filtering on the key update watcher.
- Improvement path: Index key-change notifications by room membership before broadcasting to sync subscribers.

**Large Admin Command Stack Frames:**
- Problem: Admin command dispatch functions have large stack frames, suppressed with `#[expect(clippy::large_stack_frames)]`.
- Files: `src/macros/admin.rs:37`
- Cause: Generated dispatcher functions allocate large enum variants on the stack.
- Improvement path: Box the largest variants; use `Box::pin` for async dispatch rather than inline futures.

**Blocking DB Calls on Non-Pool Threads:**
- Problem: `get_blocking()` and `exists_blocking()` are called outside the database thread pool in `device.rs` (the ABA increment), `keypair.rs`, and `globals/data.rs`.
- Files: `src/service/users/device.rs:453`, `src/service/server_keys/keypair.rs:27`, `src/service/globals/data.rs:92`
- Cause: These are startup or low-frequency paths but still block the Tokio worker thread.
- Improvement path: Migrate to the async pool path or wrap with `tokio::task::spawn_blocking`.

**State Compressor Operates on Full State Sets:**
- Problem: `state_compressor` loads entire compressed state hashes into memory for diff/resolve operations.
- Files: `src/service/rooms/state_compressor/mod.rs`
- Cause: Design limitation inherited from Conduit lineage.
- Improvement path: Incremental hash structures (e.g., persistent hash-array-mapped tries) would reduce memory and I/O for large rooms.

**Mutual Rooms Endpoint Returns Everything Without Pagination:**
- Problem: `GET /_matrix/client/unstable/uk.half-shot.msc2666/user/mutual_rooms` loads all shared rooms into a single response.
- Files: `src/api/client/unstable.rs:27`
- Cause: Pagination not yet implemented.
- Improvement path: Add `next_batch_token` support using a room ID cursor.

## Fragile Areas

**Config `Manager` Thread-Local Cache:**
- Files: `src/core/config/manager.rs`
- Why fragile: Correctness depends on callers never holding two `&Config` references simultaneously on the same thread across an await point where a reload could occur. This convention is implicit and not enforced.
- Safe modification: Always copy config values rather than holding references across `await`. Never pattern-match on two config fields in the same expression without capturing into a local first.
- Test coverage: No dedicated tests for reload under concurrent access.

**`State` Raw Pointer Shutdown Ordering:**
- Files: `src/api/router/state.rs`
- Why fragile: If axum's request future is polled after `Guard` is dropped (e.g., during graceful shutdown), the raw pointer dereference in `deref()` is undefined behavior.
- Safe modification: Do not modify the shutdown sequence in `src/router/run.rs` or `src/router/serve.rs` without verifying that the request drain completes before `Guard` drops.
- Test coverage: Smoke shutdown tests exist (`src/main/tests/smoke_shutdown.rs`) but do not explicitly test in-flight request behavior.

**Database Serializer Recursion Invariant:**
- Files: `src/database/ser.rs:38`
- Why fragile: The custom serializer asserts that serialization completes at recursion level zero. Nested types that re-enter the serializer (e.g., via custom `Serialize` impls) will panic.
- Safe modification: Only serialize flat, non-recursive types with the database serializer. Do not implement custom `Serialize` impls that call back into the serializer.
- Test coverage: Unit tests in `src/database/tests.rs` cover basic cases; array integer round-trip is broken (see above).

**Rejected Events Handling Incomplete:**
- Files: `src/core/matrix/pdu.rs:77`
- Why fragile: The `rejected` flag only exists in test builds (`#[cfg(test)]`). Production code has no way to mark or query whether an event was rejected during auth, which is required by Matrix room version 11+.
- Safe modification: Do not add room v11 as a supported version without implementing the full rejected event pipeline.
- Test coverage: `rejected` field is test-only; no integration coverage of rejection scenarios.

**Admin Command Processor Not Unwind-Safe:**
- Files: `src/service/admin/mod.rs:104`, `src/admin/processor.rs`
- Why fragile: The admin processor wraps commands in `AssertUnwindSafe` and uses `catch_unwind` to recover from panics. However, the processor itself is noted as `//TODO: not unwind safe`, meaning a panic could leave internal state (e.g., the admin room message queue) partially updated.
- Safe modification: Treat the admin processor as a best-effort command surface; do not use it for state-critical operations.
- Test coverage: `admin_execute_echo` smoke test exists; panic recovery is not tested.

## Scaling Limits

**Single RocksDB Instance:**
- Current capacity: All data for all rooms, users, and media metadata lives in one RocksDB instance.
- Limit: Scales vertically; no horizontal sharding. Large homeservers with millions of events may hit compaction pressure.
- Scaling path: Column-family-level partitioning exists; true horizontal scale would require a different storage backend.

**Sync v3 Single Stream Constraint:**
- Current capacity: One active sync stream per user session.
- Limit: Tools like Pantalaimon (E2EE proxy) open multiple sync connections; only one is served correctly.
- Scaling path: `src/api/client/sync/v3.rs:109` — requires per-session stream multiplexing or a shared sliding-window approach.

## Dependencies at Risk

**Pinned to Rust 1.94.0 Stable:**
- Risk: `rust-toolchain.toml` pins to `1.94.0`. The codebase works around a compiler bug in `1.96.0-nightly` (see `#![allow(unused_features)]` comments dated 2026-03-07), suggesting tracking of nightly regressions is active. Falling behind stable releases means missing security fixes.
- Impact: If the pinned version has an unpatched CVE, there is no quick upgrade path.
- Migration plan: Regularly advance the pinned version; the `unused_features` workaround should be removed once the nightly bug is resolved in stable.

**`stmt_expr_attributes` Stable Dependency:**
- Risk: `src/core/utils/math.rs:52` notes a rewrite is pending stabilization of `stmt_expr_attributes`. The current implementation is a workaround.
- Impact: Low; the workaround is functional but produces non-idiomatic code.
- Migration plan: Revisit when `stmt_expr_attributes` stabilizes.

## Missing Critical Features

**History Visibility Not Respected in Message/Search/Context Endpoints:**
- Problem: `/messages`, `/search`, and `/context` endpoints restrict access only to currently-joined users instead of enforcing the room's `m.room.history_visibility` setting (e.g., `world_readable`, `shared`, `invited`).
- Blocks: Spec-compliant read-only access for guest users and users who were previously members.
- Files: `src/api/client/message.rs:64`, `src/api/client/search.rs:36`, `src/api/client/context.rs:30`

**Search Response Missing Context Fields:**
- Problem: The search response hardcodes `profile_info`, `events_after`, `events_before`, `start`, `end`, and `groups` to empty/null.
- Blocks: Search result context navigation in clients.
- Files: `src/api/client/search.rs:150-177`

**Keys for Left/Deactivated Users Not Returned:**
- Problem: `GET /_matrix/client/v3/keys/query` returns an empty `left` list for users who have left rooms, which means clients cannot decrypt historical messages from those users.
- Files: `src/api/client/keys.rs:387`

**`invite_3pid` in Room Creation Not Processed:**
- Problem: Third-party invites supplied in `POST /_matrix/client/v3/createRoom` via `invite_3pid` are accepted in the request but not acted on.
- Files: `src/api/client/room/create.rs:376`

## Test Coverage Gaps

**Media Service:**
- What's not tested: Upload, download, thumbnail generation, and delete flows.
- Files: `src/service/media/tests.rs` (entire module disabled with `#[cfg(disable)]`), `src/service/media/mod.rs`, `src/service/media/thumbnail.rs`
- Risk: Regressions in media handling go undetected until integration testing or production reports.
- Priority: High

**Database Serializer Array Integer Round-Trip:**
- What's not tested: Deserialization of `ArrayVec<u64, N>` and `[u64; N]` from byte slices.
- Files: `src/database/tests.rs:487`, `src/database/tests.rs:606`
- Risk: Any key schema that embeds multiple integers silently deserializes incorrectly.
- Priority: High

**Config Reload Under Concurrency:**
- What's not tested: Thread-local config cache correctness during a reload while requests are in flight.
- Files: `src/core/config/manager.rs`
- Risk: The transmute-based lifetime extension could produce dangling references in reload scenarios.
- Priority: Medium

**Admin Processor Panic Recovery:**
- What's not tested: State consistency of the admin room after a panicking command is caught by `catch_unwind`.
- Files: `src/admin/processor.rs`, `src/service/admin/mod.rs`
- Risk: Partial state mutations from a panicking command could leave the admin room in an inconsistent state.
- Priority: Medium

**Federation Send Timeout Handling:**
- What's not tested: Behavior when a federation HTTP response hangs after headers are received (body read timeout absent).
- Files: `src/service/federation/execute.rs:180`, `src/service/appservice/request.rs:98`, `src/service/pusher/request.rs:77`
- Risk: A slow or malicious remote server can hold a connection open indefinitely, exhausting the sender thread pool.
- Priority: High

---

*Concerns audit: 2026-03-25*
