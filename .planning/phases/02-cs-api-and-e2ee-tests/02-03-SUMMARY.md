---
phase: 02-cs-api-and-e2ee-tests
plan: 03
subsystem: testing
tags: [e2ee, matrix, shadow, encryption, keys, reqwest, ruma]

# Dependency graph
requires:
  - phase: 02-01
    provides: MatrixClient wrapper, scenario module scaffold, three_host_config
provides:
  - E2EE messaging scenario (alice/bob roles) exercising server-side E2EE endpoints
  - E2EE integration test with Shadow runner (3-host, 120s stop_time)
affects: [02-04-sas-verify, phase-03-embed]

# Tech tracking
tech-stack:
  added: [urlencoding]
  patterns: [raw-cs-api-e2ee-testing, fake-device-key-generation, encrypted-event-marker-pattern]

key-files:
  created:
    - tests/shadow/src/scenarios/e2ee_msg.rs
    - tests/shadow/tests/e2ee.rs
  modified:
    - tests/shadow/src/bin/matrix_test_client.rs
    - tests/shadow/Cargo.toml
    - Cargo.lock

key-decisions:
  - "Used raw CS API endpoints for E2EE testing instead of matrix-sdk due to async-channel conflict"
  - "Fake device keys and signatures for testing server-side E2EE endpoint acceptance and storage"
  - "Encrypted message uses marker-in-ciphertext pattern for bob to verify receipt without real Olm/Megolm"
  - "120s Shadow stop_time for E2EE key exchange round trips (vs 30s for smoke test)"

patterns-established:
  - "Raw E2EE endpoint testing: upload_device_keys, claim_one_time_keys, send_encrypted_message via reqwest"
  - "Encrypted message marker pattern: embed plaintext marker in fake ciphertext for sync-based verification"
  - "Key query pattern: query_user_devices to discover device_ids before key claiming"

requirements-completed: [E2EE-01, E2EE-02, E2EE-03, E2EE-04]

# Metrics
duration: 5min
completed: 2026-03-26
---

# Phase 02 Plan 03: E2EE Messaging Scenario Summary

**Server-side E2EE endpoint testing via raw CS API with fake keys and encrypted message marker pattern under Shadow simulation**

## What Was Built

### E2EE Messaging Scenario (`e2ee_msg.rs`)

Two-role scenario testing tuwunel's server-side E2EE support:

**Alice flow:**
1. Register, login, initial sync
2. Upload device keys + 5 one-time keys (E2EE-01: `/keys/upload`)
3. Create encrypted room with `m.room.encryption` state event and alias `#e2ee-room:tuwunel-server`
4. Invite bob, poll `joined_members` until bob joins (30 retries, 2000ms interval)
5. Send `m.room.encrypted` event with ciphertext containing "encrypted secret from alice" marker (E2EE-03)

**Bob flow:**
1. Register, login, initial sync
2. Upload device keys + one-time keys (E2EE-01)
3. Join encrypted room by alias with retry (E2EE-02: 30 retries, 1000ms)
4. Query alice's devices and claim one-time keys (E2EE-02: `/keys/claim`)
5. Poll sync for `m.room.encrypted` event containing the marker (E2EE-03)

All timing uses `tokio::time::sleep` for Shadow-compatible deterministic simulation (E2EE-04).

### Integration Test (`e2ee.rs`)

Shadow runner test with 3-host config:
- `tuwunel-server`: starts at 1s
- `alice-host`: starts at 5s with `e2ee-messaging --role alice`
- `bob-host`: starts at 15s with `e2ee-messaging --role bob`
- 120s stop_time, seed 42

Asserts: scenario completion for both roles, device key upload, key claim completion, encrypted message receipt.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Replaced matrix-sdk with raw CS API endpoints**
- **Found during:** Task 1
- **Issue:** Plan references matrix-sdk::Client and SDK-managed E2EE, but matrix-sdk cannot compile due to async-channel version conflict
- **Fix:** Implemented all E2EE operations via raw HTTP calls to CS API endpoints using existing MatrixClient + reqwest
- **Files modified:** tests/shadow/src/scenarios/e2ee_msg.rs
- **Commit:** d6817681

**2. [Rule 3 - Blocking] Added urlencoding dependency for room ID percent-encoding**
- **Found during:** Task 1
- **Issue:** Room IDs and aliases contain characters requiring percent-encoding in URL paths
- **Fix:** Added urlencoding crate to shadow-test-harness Cargo.toml
- **Files modified:** tests/shadow/Cargo.toml, Cargo.lock
- **Commit:** d6817681

**3. [Rule 1 - Bug] Fixed ruma::serde::Base64 generic type annotation**
- **Found during:** Task 1 verification
- **Issue:** Compiler could not infer generic parameter for `Base64::new()`
- **Fix:** Specified `Base64::<ruma::serde::base64::Standard>` explicitly
- **Files modified:** tests/shadow/src/scenarios/e2ee_msg.rs
- **Commit:** d6817681

## Known Stubs

None. All E2EE endpoint operations are fully implemented with fake-but-structurally-valid key material. The scenario tests server endpoint acceptance and event distribution, not cryptographic correctness (which belongs in matrix-rust-client tests).

## Task Commits

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Implement E2EE messaging scenario | d6817681 | e2ee_msg.rs, matrix_test_client.rs, Cargo.toml |
| 2 | Wire E2EE integration test | f28e1532 | e2ee.rs, Cargo.lock |

## Self-Check: PASSED

All files exist. All commit hashes verified.
