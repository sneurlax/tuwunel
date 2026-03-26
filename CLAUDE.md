<!-- GSD:project-start source:PROJECT.md -->
## Project

**Tuwunel E2E Testing & Embedding**

A friendly fork of tuwunel (Rust Matrix homeserver) that adds deterministic E2E testing via Shadow network simulation and a library embedding API. Shadow runs real tuwunel binaries under simulated network conditions with deterministic time control, replacing flaky Docker-based testing. The embed crate exposes tuwunel as an in-process library for downstream consumers like matrix-rust-client.

**Core Value:** Deterministic, reproducible E2E tests that verify tuwunel's Matrix protocol behavior under realistic network conditions — before and after any code changes.

### Constraints

- **Upstream compatibility**: Fork changes must be minimal and rebaseable on upstream tuwunel releases
- **Shadow syscall coverage**: tuwunel uses standard networking (axum/hyper/tokio) which Shadow supports, but io_uring (a tuwunel feature flag) may need to be disabled for Shadow builds
- **RocksDB threading**: Each embedded server instance needs its own tempdir; multiple RocksDB instances in one process are supported but need separate paths
- **Shadow build**: Requires CMake + Cargo hybrid build (`./setup build && ./setup install`), installed to `~/.local/bin/shadow`
- **No path whitespace**: Shadow's LD_PRELOAD shim requires paths without whitespace
<!-- GSD:project-end -->

<!-- GSD:stack-start source:codebase/STACK.md -->
## Technology Stack

## Languages
- Rust 1.94.0 (nightly channel) - All server code across all crates
- Nix - Reproducible build system, flake-based packaging (`flake.nix`, `default.nix`, `nix/`)
- TOML - Configuration format (`tuwunel-example.toml`, all `Cargo.toml` files)
- Dockerfile/HCL - Container builds (`docker/`)
## Runtime
- Linux (x86_64-unknown-linux-gnu, x86_64-unknown-linux-musl, aarch64-unknown-linux-gnu, aarch64-unknown-linux-musl)
- Also unofficially: x86_64-apple-darwin, aarch64-apple-darwin (commented out in `rust-toolchain.toml`)
- Cargo (Rust workspace with resolver = "2")
- Lockfile: `Cargo.lock` - present and committed
## Frameworks
- axum 0.8 - HTTP framework with HTTP/1 + HTTP/2, JSON, form, cookies, tracing
- axum-extra 0.12 - Cookie handling, typed headers
- axum-server 0.8 - TLS-capable server
- axum-server-dual-protocol (git fork) - HTTP+HTTPS dual protocol support
- hyper 1.8 - Underlying HTTP transport (server mode, HTTP/1+2)
- tower 0.5 - Middleware stack
- tower-http 0.6 - CORS, compression, timeouts, tracing, catch-panic
- tokio 1.50 - Multi-threaded async runtime with full feature set (fs, net, sync, signal, time, io-util)
- futures 0.3 - Async combinators
- serde 1.0 + serde_json 1.0 - JSON serialization
- serde_yaml 0.9 - YAML parsing
- minicbor 2.2 + minicbor-serde 0.6 - CBOR binary format (database storage)
- toml 1.0 - TOML configuration parsing
- ruma (git fork at `github.com/matrix-construct/ruma`) - Comprehensive Matrix protocol types and APIs
- figment 0.10 - Multi-source config loading (TOML files + env vars with `TUWUNEL_` prefix)
- clap 4.5 - CLI argument parsing
- criterion 0.7 - Benchmarking framework (async/tokio support)
- insta 1.43 - Snapshot testing (JSON format)
- rustfmt (via toolchain) - Code formatting, config: `rustfmt.toml`
- clippy (via toolchain) - Linting, config: `clippy.toml` and `[workspace.lints.clippy]` in `Cargo.toml`
- cargo-deb / cargo-generate-rpm - Debian/RPM package generation
- Docker + BuildKit bake - Container builds (`docker/bake.hcl`)
## Key Dependencies
- `rust-rocksdb` (git fork: `github.com/matrix-construct/rust-rocksdb`) - Embedded key-value database engine; the only persistent storage backend
- `ruma` (git fork) - Matrix protocol types, cryptography, event handling; all Matrix protocol support depends on this
- `tokio` 1.50 - Async runtime; entire server concurrency model built on this
- `ring` 0.17 - Cryptographic primitives (hashing, signing)
- `rustls` 0.23 with `aws_lc_rs` backend and post-quantum preference - TLS implementation
- `argon2` 0.5 - Password hashing
- `hmac` 0.12 / `sha1` / `sha2` - HMAC and hash algorithms
- `jsonwebtoken` 10.3 - JWT handling with AWS LC, Ed25519, HMAC, SHA2
- `reqwest` 0.13 - HTTP client (with hickory-dns, HTTP/2, rustls, socks proxy support)
- `hickory-resolver` 0.25 - Async DNS resolver (patched fork for resolv.conf options)
- `hyper-util` (git fork) - HTTP utilities, federation resolver hooks
- `jevmalloc` (git: `github.com/matrix-construct/jevmalloc`) - Custom jemalloc allocator wrapper (optional feature)
- `ldap3` (git fork: `github.com/matrix-construct/ldap3`) - LDAP client for optional LDAP login
- `jsonwebtoken` 10.3 - JWT login support
- `sentry` 0.46 - Crash reporting and performance monitoring (optional `sentry_telemetry` feature)
- `opentelemetry` 0.31 + `opentelemetry_sdk` + `tracing-opentelemetry` 0.32 - OpenTelemetry integration (optional `perf_measurements` feature)
- `tracing` 0.1.43 (pinned) + `tracing-subscriber` 0.3 - Structured logging
- `tracing-flame` 0.2 - Flame graph profiling
- `sd-notify` 0.4 - systemd service notification (optional `systemd` feature)
- `core_affinity` (git fork) - CPU affinity masks for thread pinning
- `libloading` 0.8 - Dynamic library loading (hot-reload support in dev builds)
- `nix` 0 - Unix syscall bindings (resource limits, socket ops, user info)
- `image` 0.25 - Image processing (JPEG, PNG, GIF, WebP) for thumbnails and URL previews
- `blurhash` 0.2 - Blurhash generation for image placeholders
## Configuration
- Primary config: TOML file at path specified by `TUWUNEL_CONFIG` env var (or `CONDUIT_CONFIG` / `CONDUWUIT_CONFIG` for compatibility)
- Override any config key via `TUWUNEL__<SECTION>__<KEY>` env vars (double-underscore separator)
- Config file defaults ship as `tuwunel-example.toml`
- Required minimum: `server_name` and `database_path`
- `Cargo.toml` workspace with shared dependency versions and lint rules
- `.cargo/config.toml` - Sets `RUMA_UNSTABLE_EXHAUSTIVE_TYPES=true`
- `rust-toolchain.toml` - Pins Rust 1.94.0 with rustfmt, clippy, rust-src, rust-analyzer components
- `rustfmt.toml` - Formatting rules
- `clippy.toml` - Clippy configuration
## Platform Requirements
- Rust 1.94.0 (nightly) via rustup with toolchain file
- Linux recommended; Nix flake available for reproducible environment
- RocksDB native library (bundled via `rust-rocksdb` feature flags with bzip2, lz4, zstd)
- Linux x86_64 or aarch64 (musl or glibc)
- Deployment targets: systemd service (Debian package, RPM package, Arch), Docker/OCI container
- Reverse proxy required (nginx, Caddy, etc.) for standard Matrix port handling
- No external database needed (RocksDB is embedded)
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

## Naming Patterns
- Snake_case: `state_cache`, `event_auth`, `room_member` — all source files use snake_case
- Modules split across directories: `mod.rs` is the default entry point for multi-file modules
- Test files named either `tests.rs` (standalone file) or `mod tests { ... }` (inline block)
- Bench files live in `benches/` subdirectory alongside the source crate
- PascalCase for all types: `PduEvent`, `MutexMap`, `RoomUpgradeContext`, `TestStateMap`
- Data structs bundling DB maps are named `Data` (private, defined per service)
- Service structs are always named `Service` within their module
- snake_case for all functions and methods
- Boolean predicates prefixed with `is_` or `has_` or end in `_exists`
- Async variants have the same name as sync with `async fn` — no `_async` suffix in names
- Getter functions use descriptive names (`get_pdu_from_id`, `first_pdu_in_room`) not `get_` prefixes alone
- snake_case throughout; no camelCase in Rust code
- Single-letter names accepted in tight iterator/closure scopes (e.g., `|c|`, `|e|`, `|v|`)
- DB map fields named after the key pattern they hold: `roomid_joinedcount`, `pduid_pdu`, `eventid_pduid`
- SCREAMING_SNAKE_CASE: `SERVER_TIMESTAMP`, `INITIAL_EVENTS`, `BUFSIZE`
## Code Style
- Hard tabs (`hard_tabs = true`)
- Max line width: 98 characters (`max_width = 98`)
- Edition: 2024 (`edition = "2024"`, `style_edition = "2024"`)
- Imports grouped with `StdExternalCrate` (`group_imports = "StdExternalCrate"`)
- Import granularity at crate level (`imports_granularity = "Crate"`)
- Single-line `fn` allowed (`fn_single_line = true`)
- Match arms never use blocks: `match_arm_blocks = false`
- Match arms always have leading pipes: `match_arm_leading_pipes = "Always"`
- `use_try_shorthand = true` — use `?` not `try!()`
- `use_field_init_shorthand = true`
- Comments are word-wrapped (`wrap_comments = true`)
- Hex literals are uppercase (`hex_literal_case = "Upper"`)
- `unwrap_used = "warn"` — `unwrap()` is prohibited outside tests (`allow-unwrap-in-tests = true` in `clippy.toml`)
- `get_unwrap = "warn"` — `.get(i).unwrap()` forbidden
- `arithmetic_side_effects = "warn"` — use checked arithmetic
- `as_conversions = "warn"` — explicit `as` casts require `#[expect]`
- `undocumented_unsafe_blocks = "warn"` — all `unsafe` blocks need a safety comment
- `dbg_macro = "warn"` — no `dbg!()` in committed code
- `exit = "warn"` — no `std::process::exit`
- `str_to_string = "warn"` — prefer `.to_owned()` or `String::from()`
- `tests_outside_test_module = "warn"` — test fns must live inside `#[cfg(test)]` module or test file
- Max function lines: 780 (aspirational target ≤ 100, marked TODO)
- Max cognitive complexity: 100
- Large error threshold: 256 bytes
- Future size threshold: 24576 bytes
## Import Organization
## Error Handling
- `LogErr` — `.log_err()` / `.err_log(level)` — log the error and return `Self`
- `NotFound` — `.is_not_found()` — check if error is a NotFound variant
- `FlatOk` — flatten nested `Result<Result<T>>`
- `MapExpect` — like `.expect()` but maps to an error instead of panicking
## Logging
## Comments
## Function Design
## Module Design
#[implement(Pdu)]
- `pub` — part of the public API between crates or exposed via crate root re-exports
- `pub(crate)` — accessible within the crate
- `pub(super)` — used for test utilities shared between submodules
- Private by default for internal implementation details
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

## Pattern Overview
- Each layer is a separate Cargo crate with explicit dependency direction (no upward references)
- All services implement a common `Service` trait and are lifecycle-managed by a `Manager`
- The HTTP layer (axum + tower) is fully decoupled from business logic via `tuwunel-router` and `tuwunel-api`
- Optional developer-mode hot-reload is available via dynamic linking (`tuwunel_mods` cfg flag)
- RocksDB is the sole persistence backend, abstracted behind a typed `Map` / `Database` API
## Layers
- Purpose: Foundation types, utilities, config, logging, error types, Matrix PDU/event abstractions, and re-exports
- Location: `src/core/`
- Contains: `Server` state struct, `Config`, `Error` enum, Matrix PDU types, utility modules
- Depends on: External crates only (ruma, tokio, tracing, etc.)
- Used by: Every other internal crate
- Purpose: Procedural macros for code generation
- Location: `src/macros/`
- Contains: `#[implement]`, `#[admin_command]`, `#[admin_command_dispatch]`, `#[config_example_generator]`, `rustc_flags_capture!`
- Depends on: `syn`, `proc-macro2`, `quote`
- Used by: `tuwunel-core` and all higher-level crates
- Purpose: RocksDB persistence layer with typed serialization/deserialization
- Location: `src/database/`
- Contains: `Database`, `Engine` (RocksDB wrapper), `Map` (column family abstraction), serializers (CBOR/JSON/separator), async streams over DB iterators
- Depends on: `tuwunel-core`, `rust-rocksdb`
- Used by: `tuwunel-service`
- Purpose: All domain business logic — users, rooms, federation, media, etc.
- Location: `src/service/`
- Contains: ~40 individual `Service` structs, a `Services` aggregate, a `Manager` that runs each service as a tokio worker task
- Depends on: `tuwunel-core`, `tuwunel-database`
- Used by: `tuwunel-api`, `tuwunel-admin`, `tuwunel-router`
- Purpose: HTTP request/response types and route registration for Matrix Client-Server and Server-Server APIs
- Location: `src/api/`
- Contains: Handler functions for client routes (`src/api/client/`), server-to-server federation routes (`src/api/server/`), `Ruma<T>` extractor, route builder using `RouterExt::ruma_route`
- Depends on: `tuwunel-core`, `tuwunel-service`
- Used by: `tuwunel-router`
- Purpose: Admin room command processor
- Location: `src/admin/`
- Contains: Command dispatch (`processor`), command implementations for appservice/debug/federation/media/query/room/server/token/user namespaces
- Depends on: `tuwunel-core`, `tuwunel-service`
- Used by: `tuwunel-router` (installed/uninstalled as a callback on the admin service at run time)
- Purpose: HTTP server lifecycle, Tower middleware stack, TLS/plain/unix socket listeners
- Location: `src/router/`
- Contains: `run::start/run/stop`, `layers::build` (tower middleware), `serve::serve` (listener binding), `router::build` (axum router)
- Depends on: `tuwunel-core`, `tuwunel-service`, `tuwunel-api`, `tuwunel-admin`
- Used by: `tuwunel` (main binary)
- Purpose: Binary entry point — arg parsing, runtime construction, top-level orchestration
- Location: `src/main/`
- Contains: `main()`, `Server` wrapper struct, `runtime`, `logging`, `signals`, `mods` (hot-reload logic)
- Depends on: All other crates
- Used by: N/A (top of dependency graph)
## Data Flow
- No in-process global state; all mutable state is behind `Arc<Mutex<T>>` or `Arc<RwLock<T>>`
- Persistent state lives exclusively in RocksDB column families (via `Map`)
- In-memory caches (e.g., LRU) are owned fields of individual `Service` structs
- Shutdown is coordinated via `broadcast::Sender<&'static str>` on `tuwunel_core::Server`
## Key Abstractions
- Purpose: Common interface for every domain service (build, worker, interrupt, clear_cache, memory_usage)
- Examples: `src/service/service.rs` (trait definition), all `mod.rs` files under `src/service/*/`
- Pattern: `fn build(args: &Args<'_>) -> Result<Arc<impl Service>>` + optional `async fn worker()`
- Purpose: Single struct holding `Arc<T>` references to every service; passed to API handlers as axum `State`
- Examples: `src/service/services.rs`
- Pattern: Constructed once at startup via `Services::build`, stored behind `Arc<Services>`
- Purpose: Typed key-value store wrapping a RocksDB column family with async stream support
- Examples: `src/database/map.rs`
- Pattern: Each service declares a `Data` struct holding named `Arc<Map>` fields, opened by name from `Database`
- Purpose: axum `FromRequestParts` extractor that deserializes a typed ruma `IncomingRequest` and authenticates the caller
- Examples: `src/api/router/args.rs`
- Pattern: Handlers take `Ruma<SomeRumaRequestType>` as the final parameter and return `Result<SomeRumaResponseType>`
- Purpose: Allows method blocks to be defined in separate files while still being methods on a struct, enabling large types to be split across many files
- Examples: `src/macros/implement.rs`, used throughout `src/service/services.rs`, `src/main/server.rs`
- Pattern: `#[implement(StructName)] pub fn method_name(...)`
- Purpose: In-memory representation of a Matrix Protocol Data Unit; wraps ruma types with server-internal IDs
- Examples: `src/core/matrix/pdu/`, `src/core/matrix/event/`
- Pattern: `PduEvent` holds both the ruma event and internal `PduId`/`ShortEventId` identifiers
## Entry Points
- Location: `src/main/main.rs`
- Triggers: Process start
- Responsibilities: Arg parsing, tokio runtime creation, `Server` construction, calling `exec`
- Location: `src/router/mod.rs` (`start` extern fn) and `src/router/run.rs` (`start` async fn)
- Triggers: Called from `tuwunel::async_start` (or via dynamic module load)
- Responsibilities: Build `Services`, run migrations, start service workers, return `Arc<Services>`
- Location: `src/router/serve.rs`
- Triggers: Called from `run::run` when `config.listening` is true
- Responsibilities: Bind TCP/Unix sockets (optionally with TLS), attach middleware/router, serve connections
- Location: `src/admin/mod.rs` (`init` / `fini`)
- Triggers: Installed by `run::run`, uninstalled on shutdown
- Responsibilities: Register command completion and dispatch callbacks on the admin `Service`
## Error Handling
- `Err!(Request(NotFound("message")))` macro constructs typed `Error::Request` variants with ruma `ErrorKind`
- `Error::Request(kind, msg, status)` variants serialize to Matrix-spec JSON error responses via `IntoResponse` impl in `src/core/error/response.rs`
- Service worker panics are caught by the `Manager` with optional restart; HTTP handler panics caught by `CatchPanicLayer` in `src/router/layers.rs`
- The `#[tracing::instrument]` attribute is used pervasively; `err(Debug)` fields log errors inline
## Cross-Cutting Concerns
<!-- GSD:architecture-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd:quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd:debug` for investigation and bug fixing
- `/gsd:execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd:profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
