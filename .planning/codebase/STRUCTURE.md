# Codebase Structure

**Analysis Date:** 2026-03-25

## Directory Layout

```
tuwunel/                        # Workspace root
├── Cargo.toml                  # Workspace manifest, all shared dependencies, lint config
├── Cargo.lock                  # Locked dependency versions
├── rust-toolchain.toml         # Pinned Rust toolchain version
├── rustfmt.toml                # Rustfmt formatting config
├── clippy.toml                 # Clippy configuration
├── tuwunel-example.toml        # Generated example configuration file
├── flake.nix / default.nix     # Nix build system
├── src/                        # All Rust source crates
│   ├── main/                   # tuwunel — binary entry point crate
│   ├── router/                 # tuwunel-router — HTTP server lifecycle and middleware
│   ├── api/                    # tuwunel-api — Matrix API handlers and route registration
│   ├── admin/                  # tuwunel-admin — Admin room command processor
│   ├── service/                # tuwunel-service — Domain business logic (all services)
│   ├── database/               # tuwunel-database — RocksDB persistence layer
│   ├── core/                   # tuwunel-core — Foundation types, utils, config, PDU types
│   └── macros/                 # tuwunel-macros — Procedural macros
├── tests/                      # Integration / complement test artifacts
│   └── complement/             # Complement homeserver test results
├── docs/                       # User-facing documentation
│   ├── configuration/          # Config reference docs
│   ├── deploying/              # Deployment guides
│   └── development/            # Contributor documentation
├── arch/                       # Architecture diagrams / notes
├── nix/                        # Nix packaging helpers
│   └── pkgs/                   # Nix packages (main, complement, oci-image, book)
├── docker/                     # Docker build configuration
├── debian/                     # Debian package metadata
├── rpm/                        # RPM package metadata
└── .github/ / .gitlab/         # CI/CD pipeline definitions
```

## Directory Purposes

**`src/main/`:**
- Purpose: Binary entry point, runtime creation, startup orchestration
- Contains: `main.rs` (binary), `lib.rs` (library for tests), `server.rs`, `runtime.rs`, `logging.rs`, `signals.rs`, `mods.rs` (hot-reload), `args.rs`
- Key files:
  - `src/main/main.rs` — `fn main()` entry point
  - `src/main/lib.rs` — `exec/run/async_start/async_run/async_stop` orchestration
  - `src/main/server.rs` — `Server` wrapper struct with `#[implement]` constructor
  - `src/main/tests/` — Integration smoke tests using `insta` snapshots

**`src/router/`:**
- Purpose: HTTP server infrastructure — socket binding, tower middleware stack, request lifecycle
- Contains: `mod.rs` (extern `start/run/stop` fns), `run.rs`, `serve.rs`, `layers.rs`, `router.rs`, `request.rs`, `handle.rs`
- Key files:
  - `src/router/run.rs` — `start` / `run` / `stop` async functions
  - `src/router/layers.rs` — Tower `ServiceBuilder` stack (tracing, auth headers, CORS, timeouts, panic catcher)
  - `src/router/serve.rs` — TCP/Unix listener binding, TLS dispatch
  - `src/router/router.rs` — Final axum `Router` assembly, attaches `tuwunel_api::router::build`

**`src/api/`:**
- Purpose: Matrix Client-Server and Server-Server API handlers plus route wiring
- Contains:
  - `src/api/client/` — ~50 handler files for Client-Server API (one per endpoint group)
  - `src/api/server/` — ~20 handler files for Server-Server federation API
  - `src/api/router/` — `args.rs` (`Ruma<T>` extractor), `auth.rs`, `handler.rs` (`RumaHandler` trait), `request.rs`, `response.rs`, `state.rs`
- Key files:
  - `src/api/router/handler.rs` — `RumaHandler` / `RouterExt::ruma_route` generics
  - `src/api/router/args.rs` — Auth and request deserialization into `Ruma<T>`
  - `src/api/client/sync/` — Sync v2 implementation
  - `src/api/client/sync/v5/` — Sliding Sync (MSC4186) implementation

**`src/admin/`:**
- Purpose: Admin room text command processor
- Contains: Subdirectories per command namespace (`appservice/`, `debug/`, `federation/`, `media/`, `query/`, `room/`, `server/`, `token/`, `user/`), plus `admin.rs`, `context.rs`, `processor.rs`, `utils.rs`
- Key files:
  - `src/admin/mod.rs` — `init` / `fini` hooks called by router
  - `src/admin/processor.rs` — `dispatch` and `complete` functions registered as callbacks

**`src/service/`:**
- Purpose: All domain logic — this is where Matrix homeserver behavior lives
- Contains: ~40 service modules plus infrastructure (`service.rs` trait, `services.rs` aggregate, `manager.rs`, `migrations.rs`, `once_services.rs`)
- Notable sub-directories:
  - `src/service/rooms/` — ~15 room-related sub-services (alias, auth_chain, event_handler, state, state_res, timeline, etc.)
  - `src/service/users/` — User management (devices, keys, profiles, LDAP)
  - `src/service/federation/` — Federation request execution
  - `src/service/sending/` — Outbound PDU/EDU queue worker
  - `src/service/media/` — Media storage and retrieval
  - `src/service/oauth/` — OAuth/OIDC session management
  - `src/service/tests/` — Service-level unit tests (state resolution fixtures, snapshots)
- Key files:
  - `src/service/services.rs` — `Services` struct definition and `build`/`start`/`stop` methods
  - `src/service/service.rs` — `Service` trait and `Args` struct
  - `src/service/manager.rs` — `Manager` that spawns/supervises service worker tasks

**`src/database/`:**
- Purpose: RocksDB abstraction — opening, reading, writing, streaming
- Contains: `mod.rs` (`Database`), `map.rs` (`Map` — column family), `engine.rs` (`Engine`), `pool.rs`, `stream.rs`, `ser.rs`, `de.rs`, `maps.rs` (all column family declarations)
- Key files:
  - `src/database/mod.rs` — `Database::open` and public API
  - `src/database/map.rs` — `Map` with get/put/del/stream/watch methods
  - `src/database/engine/` — RocksDB options, backup, compaction, logger
  - `src/database/ser.rs` — `Cbor`, `Json`, `Interfix`, `Separator` serializers for keys/values
  - `src/database/stream.rs` — Async `Stream` wrappers over RocksDB iterators

**`src/core/`:**
- Purpose: Shared foundation — no business logic, only infrastructure primitives
- Contains: `config/`, `error/`, `log/`, `matrix/`, `metrics/`, `utils/`, `server.rs`, `mod.rs`
- Key files:
  - `src/core/server.rs` — `Server` struct (config, runtime handle, shutdown signal, metrics)
  - `src/core/config/mod.rs` — `Config` struct (all config fields with `#[config_example_generator]`)
  - `src/core/error/mod.rs` — `Error` enum with all variant types
  - `src/core/matrix/pdu/` — `PduEvent`, `PduId`, `PduCount`, `RawPduId`, builder
  - `src/core/utils/` — Utility modules: stream extensions, future extensions, math, string, hash, time, mutex_map, sys

**`src/macros/`:**
- Purpose: Compile-time code generation via procedural macros
- Contains: `implement.rs`, `admin.rs`, `config.rs`, `debug.rs`, `git.rs`, `rustc.rs`
- Key files:
  - `src/macros/implement.rs` — `#[implement(Type)]` allows splitting methods across files
  - `src/macros/admin.rs` — `#[admin_command]` and `#[admin_command_dispatch]` for admin CLI
  - `src/macros/config.rs` — `#[config_example_generator]` generates `tuwunel-example.toml`

## Key File Locations

**Entry Points:**
- `src/main/main.rs` — Binary `fn main()`
- `src/main/lib.rs` — Async exec/run/start/stop coordination
- `src/router/mod.rs` — `extern "Rust" fn start/run/stop` (hot-reload ABI boundary)

**Configuration:**
- `src/core/config/mod.rs` — `Config` struct (authoritative source of all config keys)
- `src/core/config/check.rs` — Config validation logic
- `tuwunel-example.toml` — Generated example (do not edit; generated from `Config` doc comments)
- `Cargo.toml` — Workspace dependencies and lint configuration

**Core Logic:**
- `src/service/services.rs` — `Services` aggregate struct
- `src/service/service.rs` — `Service` trait definition
- `src/service/manager.rs` — Service worker lifecycle manager
- `src/service/rooms/timeline/mod.rs` — Room timeline (PDU append/backfill)
- `src/service/rooms/event_handler/mod.rs` — Incoming federation PDU handling
- `src/service/rooms/state_res/` — Matrix state resolution algorithm
- `src/service/sending/mod.rs` — Outbound federation queue

**HTTP Wiring:**
- `src/api/router/handler.rs` — `RumaHandler` / `ruma_route` extension
- `src/api/router/args.rs` — `Ruma<T>` FromRequestParts extractor
- `src/router/layers.rs` — Tower middleware stack
- `src/router/router.rs` — Final axum Router assembly

**Testing:**
- `src/main/tests/` — Integration smoke tests
- `src/service/tests/` — Service unit tests (state resolution)
- `src/database/benches/` — Database benchmarks
- `src/service/benches/` — Service benchmarks
- `tests/complement/` — Complement homeserver test results

## Naming Conventions

**Files:**
- `mod.rs` — Module root; declares submodules and re-exports public API
- `<domain>.rs` (single file) or `<domain>/mod.rs` (directory module) — One concept per file
- Handler files named after endpoint group: `message.rs`, `membership.rs`, `sync.rs`, etc.
- Service data access struct in a `data.rs` file within the service directory

**Directories:**
- Plural noun for a collection of related services: `src/service/rooms/`, `src/admin/user/`
- Snake_case throughout

**Types:**
- `Service` — Every domain service struct is literally named `Service` within its module
- `Data` — Inner struct holding `Arc<Map>` database handles within a service
- `Services` — The aggregate singleton
- `Args<'_>` — The build-time argument bundle passed to `Service::build`

**Functions:**
- `build` — Constructor for `Service` types (returns `Result<Arc<Self>>`)
- `worker` — Async loop function run as a tokio task by `Manager`
- `interrupt` — Signal service to stop its worker loop
- Handler functions use the endpoint name in snake_case: `get_message_events`, `send_message_event`

## Where to Add New Code

**New Client-Server API Endpoint:**
- Handler implementation: `src/api/client/<group>.rs`
- Route registration: Find the relevant `ruma_route!` call in `src/api/client/mod.rs` or create a new one
- Business logic: `src/service/<domain>/` (new file or extend existing service)

**New Federation (S2S) Endpoint:**
- Handler: `src/api/server/<endpoint>.rs`
- Route registration: `src/api/server/mod.rs`

**New Service:**
- Create `src/service/<name>/mod.rs` with a `Service` struct implementing `crate::Service`
- Add `pub mod <name>;` to `src/service/mod.rs`
- Add `pub <name>: Arc<<name>::Service>,` field to `Services` in `src/service/services.rs`
- Instantiate in `Services::build` and include in `services()` iterator

**New Admin Command:**
- Add command function annotated with `#[admin_command]` in `src/admin/<namespace>/`
- Add variant to the dispatch enum annotated with `#[admin_command_dispatch]`

**New Database Column Family:**
- Declare the column family name and type in `src/database/maps.rs`
- Open the `Map` in the relevant service `Data` struct via `db["column_name"].clone()`

**New Config Option:**
- Add field to `Config` struct in `src/core/config/mod.rs` with doc comment (doc comment generates example toml)
- Optionally add validation in `src/core/config/check.rs`

**Utilities:**
- Shared helpers: `src/core/utils/` (pick the relevant sub-module)
- Stream utilities: `src/core/utils/stream/`
- Future utilities: `src/core/utils/future/`

## Special Directories

**`.planning/`:**
- Purpose: GSD planning documents and phase tracking
- Generated: No
- Committed: Yes

**`src/main/tests/snapshots/`:**
- Purpose: `insta` snapshot test golden files
- Generated: Yes (by running `cargo insta review`)
- Committed: Yes

**`src/service/tests/state_res/fixtures/`:**
- Purpose: JSON fixtures for state resolution algorithm tests
- Generated: No
- Committed: Yes

**`tests/complement/`:**
- Purpose: Complement Matrix homeserver conformance test results
- Generated: Yes (by CI)
- Committed: Yes (results file)

**`nix/pkgs/`:**
- Purpose: Nix derivations for building the project and Docker images
- Generated: No
- Committed: Yes

---

*Structure analysis: 2026-03-25*
