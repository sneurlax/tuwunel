# Milestones

## v1.0 E2E Testing & Embedding (Shipped: 2026-03-27)

**Phases completed:** 4 phases, 11 plans, 19 tasks

**Key accomplishments:**

- Shadow build profile with io_uring compile guard, and typed Shadow YAML + tuwunel TOML config generation in tests/shadow/ crate
- matrix-test-client binary with retry-polling smoke subcommand and Shadow runner module with structured result capture and failure diagnostics
- Integration test wiring ShadowConfig, TuwunelConfig, and run_shadow into a full smoke scenario with two Shadow hosts and deterministic seed validation
- Ruma+reqwest MatrixClient with UIAA registration, multi-host Shadow topology builder, and cs-api/e2ee/sas subcommand stubs
- Two-client Matrix CS API test scenario with alice (register, login, create room, send message) and bob (register, login, join by alias with retry, sync, verify message receipt) via Shadow simulation
- Server-side E2EE endpoint testing via raw CS API with fake keys and encrypted message marker pattern under Shadow simulation
- SAS verification protocol message routing test via raw CS API to-device endpoints under Shadow simulation
- Inline GML topology support with three named fixtures and E2EE-under-impairment integration test using 200ms RTT + 2% packet loss
- 100-client Shadow load test with programmatic N-host config builder, creator/joiner role dispatch, and binary pass/fail validation
- OnceLock multi-instance safety fix and tuwunel-embed crate scaffold with figment-based config builder, port 0 pre-bind, and auto tempdir
- Graceful stop(), UIAA register_user(), and multi-instance integration tests completing the tuwunel-embed API

---

## v1.0 E2E Testing & Embedding MVP (Shipped: 2026-03-27)

**Phases completed:** 4 phases, 11 plans, 19 tasks

**Key accomplishments:**

- Shadow build profile with io_uring compile guard, and typed Shadow YAML + tuwunel TOML config generation in tests/shadow/ crate
- matrix-test-client binary with retry-polling smoke subcommand and Shadow runner module with structured result capture and failure diagnostics
- Integration test wiring ShadowConfig, TuwunelConfig, and run_shadow into a full smoke scenario with two Shadow hosts and deterministic seed validation
- Ruma+reqwest MatrixClient with UIAA registration, multi-host Shadow topology builder, and cs-api/e2ee/sas subcommand stubs
- Two-client Matrix CS API test scenario with alice (register, login, create room, send message) and bob (register, login, join by alias with retry, sync, verify message receipt) via Shadow simulation
- Server-side E2EE endpoint testing via raw CS API with fake keys and encrypted message marker pattern under Shadow simulation
- SAS verification protocol message routing test via raw CS API to-device endpoints under Shadow simulation
- Inline GML topology support with three named fixtures and E2EE-under-impairment integration test using 200ms RTT + 2% packet loss
- 100-client Shadow load test with programmatic N-host config builder, creator/joiner role dispatch, and binary pass/fail validation
- OnceLock multi-instance safety fix and tuwunel-embed crate scaffold with figment-based config builder, port 0 pre-bind, and auto tempdir
- Graceful stop(), UIAA register_user(), and multi-instance integration tests completing the tuwunel-embed API

---
