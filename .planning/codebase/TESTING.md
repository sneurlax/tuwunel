# Testing Patterns

**Analysis Date:** 2026-03-25

## Test Framework

**Runner:**
- Rust's built-in `cargo test` harness
- No separate test runner (no nextest config found)
- Nightly toolchain pinned at `1.94.0` (via `rust-toolchain.toml`)

**Async test runtime:**
- `#[tokio::test]` for async tests (from `tokio` workspace dependency)

**Snapshot testing:**
- `insta` — used for `assert_debug_snapshot!` and `assert_json_snapshot!` with `with_settings!` context

**Benchmarking:**
- `criterion` — used for all benchmarks (`harness = false` bench targets)

**Test data / fixtures:**
- `maplit` — for `hashmap!` macro in tests
- `similar` — for diff output in state resolution integration tests
- `serde_json::json!` — for constructing inline JSON PDU fixtures

**Run Commands:**
```bash
cargo test                          # Run all tests
cargo test -- --nocapture           # Show stdout (println!) during tests
cargo bench                         # Run benchmarks
RUST_LOG=debug cargo test           # Enable logging in tests
cargo insta review                  # Review snapshot changes interactively
cargo insta accept                  # Accept all pending snapshot updates
```

## Test File Organization

**Two patterns are used side-by-side:**

**Pattern A — Separate test file, declared with `mod tests;`:**
```
src/core/utils/mod.rs          # declares: #[cfg(test)] mod tests;
src/core/utils/tests.rs        # contains: #[test] fn ...
src/core/utils/string.rs       # declares: mod tests;
src/core/utils/string/tests.rs # contains: #[test] fn ...
```

**Pattern B — Inline `mod tests` block at end of file:**
```rust
// At bottom of src/core/utils/content_disposition.rs:
#[cfg(test)]
mod tests {
    #[test]
    fn string_sanitisation() { ... }
}
```

**Pattern C — Dedicated test directory at crate level:**
```
src/main/tests/
    smoke.rs
    smoke_async.rs
    smoke_shutdown.rs
    admin_execute_echo.rs
    snapshots/                 # insta snapshot files

src/service/tests/
    state_res/
        main.rs
        resolve.rs
        resolve/snapshot_tests.rs
        resolve/snapshots/
        fixtures/              # JSON PDU fixture files
            bootstrap-private-chat.json
            MSC4297-problem-A/
            MSC4297-problem-B/
```

Test files in dedicated directories use `#![cfg(test)]` at the file level rather than a `#[cfg(test)]` module wrapper.

**Naming convention for test functions:** descriptive snake_case names describing the scenario tested, not the function under test. Examples: `increment_none`, `increment_wrap`, `mutex_map_cleanup`, `valid_room_create`, `missing_state_key`.

## Test Structure

**Standard sync test:**
```rust
#[test]
fn increment_none() {
    let bytes: [u8; 8] = utils::increment(None);
    let res = u64::from_be_bytes(bytes);
    assert_eq!(res, 1);
}
```

**Panic / expected failure test:**
```rust
#[test]
#[should_panic(expected = "overflow")]
fn checked_add_overflow() {
    use crate::checked;
    let a = u64::MAX;
    let res = checked!(a + 1).expect("overflow");
    assert_eq!(res, 0);
}
```

**Async test:**
```rust
#[tokio::test]
async fn mutex_map_cleanup() {
    use crate::utils::MutexMap;
    let map = MutexMap::<String, ()>::new();
    let lock = map.lock("foo").await;
    assert!(!map.is_empty(), "map must not be empty");
    drop(lock);
    assert!(map.is_empty(), "map must be empty");
}
```

**Integration smoke test with snapshot:**
```rust
#[test]
fn smoke() -> Result {
    with_settings!({
        description => "Smoke Test",
        snapshot_suffix => "smoke_test",
    }, {
        let args = Args::default_test(&["smoke", "fresh", "cleanup"]);
        let runtime = runtime::new(Some(&args))?;
        let server = Server::new(Some(&args), Some(runtime.handle()))?;
        let result = tuwunel::exec(&server, runtime);
        assert_debug_snapshot!(result);
        result
    })
}
```

**Conditional panic based on build mode:**
```rust
#[test]
#[cfg_attr(
    debug_assertions,
    should_panic(expected = "serializing string at the top-level")
)]
fn ser_str() { ... }
```

## Mocking

**No mocking framework detected.** Tests use real implementations with test-specific helpers and in-memory stores.

**Test fixture helpers** in `src/service/rooms/state_res/test_utils.rs`:
- `TestStore` — in-memory event store implementing the same trait as the production store
- `TestStateMap` — wraps a `HashMap` and exposes `fetch_state_fn()` closures that simulate DB lookups
- Helper functions: `alice()`, `bob()`, `charlie()`, `ella()`, `zara()` — return `OwnedUserId`
- Helper functions: `room_id()`, `event_id(n)` — return typed IDs
- `to_pdu_event(...)`, `to_init_pdu_event(...)` — build `PduEvent` instances for test scenarios
- `init_subscriber()` — sets up a `tracing_subscriber` for log output in tests; returns a guard; call as `let _guard = init_subscriber();`

**What to mock:**
- DB/store access is replaced by in-memory test implementations (`TestStore`, `TestStateMap`)
- External network calls are not tested at the unit level (no HTTP mocking)

**What NOT to mock:**
- State resolution logic itself (tested against real `resolve()` function)
- `PduEvent` construction (tested with real types via helper builders)

## Fixtures and Factories

**JSON PDU fixtures** (for state resolution integration tests):
- Location: `src/service/tests/state_res/fixtures/`
- Format: JSON files containing arrays of PDU objects
- Named descriptively: `bootstrap-private-chat.json`, `origin-server-ts-tiebreak.json`
- MSC-specific fixtures are namespaced in subdirectories: `fixtures/MSC4297-problem-A/`

**In-code test event builders** (`src/service/rooms/state_res/test_utils.rs`):
```rust
// Build a PDU event with explicit auth/prev_event chains:
let event = to_pdu_event(
    "HELLO",                          // event "name" (used as ID)
    charlie(),                        // sender
    TimelineEventType::RoomMember,    // event type
    None,                             // state key (None = missing)
    member_content_join(),            // content
    &["CREATE", "IMA", "IPOWER"],     // auth_events (by name)
    &["IPOWER"],                      // prev_events (by name)
);
```

**Static initial event sets:**
- `INITIAL_EVENTS()` — returns a standard set of bootstrap events (create, join, power levels)
- `INITIAL_HYDRA_EVENTS()` — variant for Hydra room version tests

## Coverage

**Requirements:** None enforced. No minimum coverage thresholds configured.

**No coverage tooling** configured in CI (`/.github/workflows/` not explored). Coverage can be generated manually with:
```bash
cargo llvm-cov                      # if llvm-cov is installed
```

## Test Types

**Unit Tests:**
- Scope: individual pure functions and data transformations
- Location: `#[cfg(test)] mod tests { ... }` inline blocks or sibling `tests.rs` files
- Examples: `src/core/utils/tests.rs`, `src/core/utils/string/tests.rs`, `src/database/tests.rs`, `src/core/matrix/pdu/tests.rs`
- Do not require a running server or database

**State Resolution Logic Tests:**
- Scope: auth rules and state resolution algorithms — close to integration tests but without a real server
- Location: `src/service/rooms/state_res/event_auth/tests/`, `src/service/rooms/state_res/resolve/tests.rs`
- Use `TestStore` and `TestStateMap` in-memory implementations
- Examples: `valid_room_create`, `missing_state_key`, `check_room_member`

**Integration Smoke Tests:**
- Scope: full server startup/shutdown lifecycle
- Location: `src/main/tests/`
- Use `Args::default_test(&["smoke", "fresh", "cleanup"])` to configure in-memory/temp-dir server
- Results captured with `insta::assert_debug_snapshot!`
- Examples: `smoke`, `smoke_shutdown`, `smoke_async`, `admin_execute_echo`

**Snapshot Tests:**
- Scope: state resolution outputs — verify resolved state maps against committed `.snap` files
- Location: `src/service/tests/state_res/resolve/snapshot_tests.rs`
- Use `snapshot_test!` and `snapshot_test_contrived_states!` macros to reduce boilerplate
- Snapshots stored in `snapshots/` sibling directory
- Use `insta::assert_json_snapshot!` with `omit_expression => true`

**Benchmarks:**
- Scope: performance of serialization, state resolution, and server startup
- Location: `src/database/benches/ser.rs`, `src/service/benches/state_res.rs`, `src/main/benches/main.rs`
- Use `criterion` with `criterion_group!` / `criterion_main!`
- All bench files use `#![cfg(test)]` to keep them out of normal builds

## Common Patterns

**Async testing with concurrency barriers:**
```rust
#[tokio::test]
async fn mutex_map_contend() {
    use std::sync::Arc;
    use tokio::sync::Barrier;
    use crate::utils::MutexMap;

    let map = Arc::new(MutexMap::<String, ()>::new());
    let seq = Arc::new([Barrier::new(2), Barrier::new(2)]);

    let join_a = tokio::spawn(async move { /* ... */ });
    let join_b = tokio::spawn(async move { /* ... */ });

    seq[0].wait().await;
    // assert invariants mid-contention
    seq[1].wait().await;

    tokio::try_join!(join_b, join_a).expect("joined");
}
```

**Error testing (expected failure):**
```rust
#[test]
#[should_panic(expected = "unverified")]
fn password_hash_and_verify_fail() {
    let digest = hash::password("temp123").expect("digest");
    hash::verify_password("temp321", &digest).expect("unverified");
}
```

**Snapshot test with description and suffix:**
```rust
insta::with_settings!({
    description => "Resolved state",
    omit_expression => true,
    snapshot_suffix => "resolved_state",
}, {
    insta::assert_json_snapshot!(&resolved_state);
});
```

**Lint suppression in tests:**
```rust
#[expect(clippy::iter_on_single_items, clippy::many_single_char_names)]
fn set_intersection_all() { ... }
```
`#[expect]` is preferred over `#[allow]` even in test code so the compiler warns if the suppression becomes stale.

**Tracing subscriber for test log output:**
```rust
// In test:
let _guard = init_subscriber();
// Guard must be kept alive for the test duration; assign to `_guard` not `_`
```

---

*Testing analysis: 2026-03-25*
