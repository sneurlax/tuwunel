# Architecture

**Analysis Date:** 2026-03-25

## Pattern Overview

**Overall:** Layered Microkernel — a Cargo workspace of seven crates arranged in strict dependency tiers, with a service-oriented core and hot-reloadable dynamic module support.

**Key Characteristics:**
- Each layer is a separate Cargo crate with explicit dependency direction (no upward references)
- All services implement a common `Service` trait and are lifecycle-managed by a `Manager`
- The HTTP layer (axum + tower) is fully decoupled from business logic via `tuwunel-router` and `tuwunel-api`
- Optional developer-mode hot-reload is available via dynamic linking (`tuwunel_mods` cfg flag)
- RocksDB is the sole persistence backend, abstracted behind a typed `Map` / `Database` API

## Layers

**Core (`tuwunel-core`):**
- Purpose: Foundation types, utilities, config, logging, error types, Matrix PDU/event abstractions, and re-exports
- Location: `src/core/`
- Contains: `Server` state struct, `Config`, `Error` enum, Matrix PDU types, utility modules
- Depends on: External crates only (ruma, tokio, tracing, etc.)
- Used by: Every other internal crate

**Macros (`tuwunel-macros`):**
- Purpose: Procedural macros for code generation
- Location: `src/macros/`
- Contains: `#[implement]`, `#[admin_command]`, `#[admin_command_dispatch]`, `#[config_example_generator]`, `rustc_flags_capture!`
- Depends on: `syn`, `proc-macro2`, `quote`
- Used by: `tuwunel-core` and all higher-level crates

**Database (`tuwunel-database`):**
- Purpose: RocksDB persistence layer with typed serialization/deserialization
- Location: `src/database/`
- Contains: `Database`, `Engine` (RocksDB wrapper), `Map` (column family abstraction), serializers (CBOR/JSON/separator), async streams over DB iterators
- Depends on: `tuwunel-core`, `rust-rocksdb`
- Used by: `tuwunel-service`

**Service (`tuwunel-service`):**
- Purpose: All domain business logic — users, rooms, federation, media, etc.
- Location: `src/service/`
- Contains: ~40 individual `Service` structs, a `Services` aggregate, a `Manager` that runs each service as a tokio worker task
- Depends on: `tuwunel-core`, `tuwunel-database`
- Used by: `tuwunel-api`, `tuwunel-admin`, `tuwunel-router`

**API (`tuwunel-api`):**
- Purpose: HTTP request/response types and route registration for Matrix Client-Server and Server-Server APIs
- Location: `src/api/`
- Contains: Handler functions for client routes (`src/api/client/`), server-to-server federation routes (`src/api/server/`), `Ruma<T>` extractor, route builder using `RouterExt::ruma_route`
- Depends on: `tuwunel-core`, `tuwunel-service`
- Used by: `tuwunel-router`

**Admin (`tuwunel-admin`):**
- Purpose: Admin room command processor
- Location: `src/admin/`
- Contains: Command dispatch (`processor`), command implementations for appservice/debug/federation/media/query/room/server/token/user namespaces
- Depends on: `tuwunel-core`, `tuwunel-service`
- Used by: `tuwunel-router` (installed/uninstalled as a callback on the admin service at run time)

**Router (`tuwunel-router`):**
- Purpose: HTTP server lifecycle, Tower middleware stack, TLS/plain/unix socket listeners
- Location: `src/router/`
- Contains: `run::start/run/stop`, `layers::build` (tower middleware), `serve::serve` (listener binding), `router::build` (axum router)
- Depends on: `tuwunel-core`, `tuwunel-service`, `tuwunel-api`, `tuwunel-admin`
- Used by: `tuwunel` (main binary)

**Main (`tuwunel`):**
- Purpose: Binary entry point — arg parsing, runtime construction, top-level orchestration
- Location: `src/main/`
- Contains: `main()`, `Server` wrapper struct, `runtime`, `logging`, `signals`, `mods` (hot-reload logic)
- Depends on: All other crates
- Used by: N/A (top of dependency graph)

## Data Flow

**Inbound Client Request:**

1. TCP/Unix socket listener in `src/router/serve/` accepts the connection
2. axum dispatches through the tower middleware stack built in `src/router/layers.rs`
3. `src/router/request.rs` middleware spawns a tokio task per request, checks server running state
4. Request hits a handler registered via `RouterExt::ruma_route` in `src/api/router/handler.rs`
5. Handler function (e.g., `src/api/client/message.rs`) calls services via `Arc<Services>`
6. Service methods read/write via `Arc<Map>` (RocksDB column families) in `tuwunel-database`
7. Response serialized to JSON and returned as HTTP response

**Inbound Federation PDU:**

1. `src/api/server/` handlers receive raw PDU JSON from remote server
2. `service::rooms::event_handler` validates and resolves state via `handle_incoming_pdu`
3. State resolution performed by `service::rooms::state_res`
4. Accepted PDU appended to the room timeline via `service::rooms::timeline`
5. `service::sending` queues outgoing EDUs/PDUs for other servers

**Server Startup:**

1. `src/main/main.rs` parses args, creates `Runtime`, creates `Server`
2. `tuwunel::exec` calls `router::start` which calls `Services::build` then `Services::start`
3. `Services::build` opens `Database` and constructs every `Arc<dyn Service>` via `Service::build`
4. `Services::start` runs database migrations then starts the `Manager`
5. `Manager` spawns each service's `worker()` as a tokio task
6. `router::run` installs admin callback, starts socket listeners, enters the main select loop

**State Management:**
- No in-process global state; all mutable state is behind `Arc<Mutex<T>>` or `Arc<RwLock<T>>`
- Persistent state lives exclusively in RocksDB column families (via `Map`)
- In-memory caches (e.g., LRU) are owned fields of individual `Service` structs
- Shutdown is coordinated via `broadcast::Sender<&'static str>` on `tuwunel_core::Server`

## Key Abstractions

**`Service` Trait:**
- Purpose: Common interface for every domain service (build, worker, interrupt, clear_cache, memory_usage)
- Examples: `src/service/service.rs` (trait definition), all `mod.rs` files under `src/service/*/`
- Pattern: `fn build(args: &Args<'_>) -> Result<Arc<impl Service>>` + optional `async fn worker()`

**`Services` Aggregate:**
- Purpose: Single struct holding `Arc<T>` references to every service; passed to API handlers as axum `State`
- Examples: `src/service/services.rs`
- Pattern: Constructed once at startup via `Services::build`, stored behind `Arc<Services>`

**`Map` (RocksDB Column Family):**
- Purpose: Typed key-value store wrapping a RocksDB column family with async stream support
- Examples: `src/database/map.rs`
- Pattern: Each service declares a `Data` struct holding named `Arc<Map>` fields, opened by name from `Database`

**`Ruma<T>` Extractor:**
- Purpose: axum `FromRequestParts` extractor that deserializes a typed ruma `IncomingRequest` and authenticates the caller
- Examples: `src/api/router/args.rs`
- Pattern: Handlers take `Ruma<SomeRumaRequestType>` as the final parameter and return `Result<SomeRumaResponseType>`

**`#[implement]` Macro:**
- Purpose: Allows method blocks to be defined in separate files while still being methods on a struct, enabling large types to be split across many files
- Examples: `src/macros/implement.rs`, used throughout `src/service/services.rs`, `src/main/server.rs`
- Pattern: `#[implement(StructName)] pub fn method_name(...)`

**`PduEvent` / PDU Types:**
- Purpose: In-memory representation of a Matrix Protocol Data Unit; wraps ruma types with server-internal IDs
- Examples: `src/core/matrix/pdu/`, `src/core/matrix/event/`
- Pattern: `PduEvent` holds both the ruma event and internal `PduId`/`ShortEventId` identifiers

## Entry Points

**Binary Entry:**
- Location: `src/main/main.rs`
- Triggers: Process start
- Responsibilities: Arg parsing, tokio runtime creation, `Server` construction, calling `exec`

**Router Start:**
- Location: `src/router/mod.rs` (`start` extern fn) and `src/router/run.rs` (`start` async fn)
- Triggers: Called from `tuwunel::async_start` (or via dynamic module load)
- Responsibilities: Build `Services`, run migrations, start service workers, return `Arc<Services>`

**HTTP Listener:**
- Location: `src/router/serve.rs`
- Triggers: Called from `run::run` when `config.listening` is true
- Responsibilities: Bind TCP/Unix sockets (optionally with TLS), attach middleware/router, serve connections

**Admin Room:**
- Location: `src/admin/mod.rs` (`init` / `fini`)
- Triggers: Installed by `run::run`, uninstalled on shutdown
- Responsibilities: Register command completion and dispatch callbacks on the admin `Service`

## Error Handling

**Strategy:** Single `tuwunel_core::Error` enum with `From` impls for all third-party error types; propagated via `Result<T>` alias (`type Result<T> = std::result::Result<T, Error>`)

**Patterns:**
- `Err!(Request(NotFound("message")))` macro constructs typed `Error::Request` variants with ruma `ErrorKind`
- `Error::Request(kind, msg, status)` variants serialize to Matrix-spec JSON error responses via `IntoResponse` impl in `src/core/error/response.rs`
- Service worker panics are caught by the `Manager` with optional restart; HTTP handler panics caught by `CatchPanicLayer` in `src/router/layers.rs`
- The `#[tracing::instrument]` attribute is used pervasively; `err(Debug)` fields log errors inline

## Cross-Cutting Concerns

**Logging:** `tracing` crate, configured via `tuwunel_core::log::Logging`; custom macros (`debug_info!`, `debug_error!`, `debug_warn!`) that include module context; optional flame profiling via `tracing-flame`, optional OpenTelemetry export, optional Sentry integration

**Validation:** Ruma types enforce Matrix protocol type safety at the HTTP boundary. Additional config validation in `src/core/config/check.rs`. Request body size is enforced by `DefaultBodyLimit` in `src/router/layers.rs`.

**Authentication:** `Ruma<T>` extractor in `src/api/router/args.rs` verifies Bearer tokens, extracts user identity, and validates appservice credentials before any handler is called. Auth middleware is part of the `FromRequestParts` implementation, not a separate tower layer.

---

*Architecture analysis: 2026-03-25*
