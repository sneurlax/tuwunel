# Technology Stack

**Analysis Date:** 2026-03-25

## Languages

**Primary:**
- Rust 1.94.0 (nightly channel) - All server code across all crates

**Secondary:**
- Nix - Reproducible build system, flake-based packaging (`flake.nix`, `default.nix`, `nix/`)
- TOML - Configuration format (`tuwunel-example.toml`, all `Cargo.toml` files)
- Dockerfile/HCL - Container builds (`docker/`)

## Runtime

**Environment:**
- Linux (x86_64-unknown-linux-gnu, x86_64-unknown-linux-musl, aarch64-unknown-linux-gnu, aarch64-unknown-linux-musl)
- Also unofficially: x86_64-apple-darwin, aarch64-apple-darwin (commented out in `rust-toolchain.toml`)

**Package Manager:**
- Cargo (Rust workspace with resolver = "2")
- Lockfile: `Cargo.lock` - present and committed

## Frameworks

**Core HTTP:**
- axum 0.8 - HTTP framework with HTTP/1 + HTTP/2, JSON, form, cookies, tracing
- axum-extra 0.12 - Cookie handling, typed headers
- axum-server 0.8 - TLS-capable server
- axum-server-dual-protocol (git fork) - HTTP+HTTPS dual protocol support
- hyper 1.8 - Underlying HTTP transport (server mode, HTTP/1+2)
- tower 0.5 - Middleware stack
- tower-http 0.6 - CORS, compression, timeouts, tracing, catch-panic

**Async Runtime:**
- tokio 1.50 - Multi-threaded async runtime with full feature set (fs, net, sync, signal, time, io-util)
- futures 0.3 - Async combinators

**Serialization:**
- serde 1.0 + serde_json 1.0 - JSON serialization
- serde_yaml 0.9 - YAML parsing
- minicbor 2.2 + minicbor-serde 0.6 - CBOR binary format (database storage)
- toml 1.0 - TOML configuration parsing

**Matrix Protocol:**
- ruma (git fork at `github.com/matrix-construct/ruma`) - Comprehensive Matrix protocol types and APIs
  - Enabled features: client-api, federation-api, appservice-api, push-gateway-api, markdown, numerous unstable MSCs

**Configuration:**
- figment 0.10 - Multi-source config loading (TOML files + env vars with `TUWUNEL_` prefix)
- clap 4.5 - CLI argument parsing

**Testing:**
- criterion 0.7 - Benchmarking framework (async/tokio support)
- insta 1.43 - Snapshot testing (JSON format)

**Build/Dev:**
- rustfmt (via toolchain) - Code formatting, config: `rustfmt.toml`
- clippy (via toolchain) - Linting, config: `clippy.toml` and `[workspace.lints.clippy]` in `Cargo.toml`
- cargo-deb / cargo-generate-rpm - Debian/RPM package generation
- Docker + BuildKit bake - Container builds (`docker/bake.hcl`)

## Key Dependencies

**Critical:**
- `rust-rocksdb` (git fork: `github.com/matrix-construct/rust-rocksdb`) - Embedded key-value database engine; the only persistent storage backend
- `ruma` (git fork) - Matrix protocol types, cryptography, event handling; all Matrix protocol support depends on this
- `tokio` 1.50 - Async runtime; entire server concurrency model built on this

**Cryptography:**
- `ring` 0.17 - Cryptographic primitives (hashing, signing)
- `rustls` 0.23 with `aws_lc_rs` backend and post-quantum preference - TLS implementation
- `argon2` 0.5 - Password hashing
- `hmac` 0.12 / `sha1` / `sha2` - HMAC and hash algorithms
- `jsonwebtoken` 10.3 - JWT handling with AWS LC, Ed25519, HMAC, SHA2

**Networking:**
- `reqwest` 0.13 - HTTP client (with hickory-dns, HTTP/2, rustls, socks proxy support)
- `hickory-resolver` 0.25 - Async DNS resolver (patched fork for resolv.conf options)
- `hyper-util` (git fork) - HTTP utilities, federation resolver hooks

**Memory:**
- `jevmalloc` (git: `github.com/matrix-construct/jevmalloc`) - Custom jemalloc allocator wrapper (optional feature)

**Auth/Identity:**
- `ldap3` (git fork: `github.com/matrix-construct/ldap3`) - LDAP client for optional LDAP login
- `jsonwebtoken` 10.3 - JWT login support

**Observability:**
- `sentry` 0.46 - Crash reporting and performance monitoring (optional `sentry_telemetry` feature)
- `opentelemetry` 0.31 + `opentelemetry_sdk` + `tracing-opentelemetry` 0.32 - OpenTelemetry integration (optional `perf_measurements` feature)
- `tracing` 0.1.43 (pinned) + `tracing-subscriber` 0.3 - Structured logging
- `tracing-flame` 0.2 - Flame graph profiling

**Infrastructure:**
- `sd-notify` 0.4 - systemd service notification (optional `systemd` feature)
- `core_affinity` (git fork) - CPU affinity masks for thread pinning
- `libloading` 0.8 - Dynamic library loading (hot-reload support in dev builds)
- `nix` 0 - Unix syscall bindings (resource limits, socket ops, user info)

**Media:**
- `image` 0.25 - Image processing (JPEG, PNG, GIF, WebP) for thumbnails and URL previews
- `blurhash` 0.2 - Blurhash generation for image placeholders

## Configuration

**Environment:**
- Primary config: TOML file at path specified by `TUWUNEL_CONFIG` env var (or `CONDUIT_CONFIG` / `CONDUWUIT_CONFIG` for compatibility)
- Override any config key via `TUWUNEL__<SECTION>__<KEY>` env vars (double-underscore separator)
- Config file defaults ship as `tuwunel-example.toml`
- Required minimum: `server_name` and `database_path`

**Build:**
- `Cargo.toml` workspace with shared dependency versions and lint rules
- `.cargo/config.toml` - Sets `RUMA_UNSTABLE_EXHAUSTIVE_TYPES=true`
- `rust-toolchain.toml` - Pins Rust 1.94.0 with rustfmt, clippy, rust-src, rust-analyzer components
- `rustfmt.toml` - Formatting rules
- `clippy.toml` - Clippy configuration

**Feature Flags (in `src/main/Cargo.toml`):**
Default enabled features: `brotli_compression`, `element_hacks`, `gzip_compression`, `io_uring`, `jemalloc`, `jemalloc_conf`, `media_thumbnail`, `release_max_log_level`, `systemd`, `url_preview`, `zstd_compression`

Optional features: `ldap`, `sentry_telemetry`, `perf_measurements`, `direct_tls`, `console`, `blurhashing`, `bzip2_compression`, `lz4_compression`, `tuwunel_mods`

## Platform Requirements

**Development:**
- Rust 1.94.0 (nightly) via rustup with toolchain file
- Linux recommended; Nix flake available for reproducible environment
- RocksDB native library (bundled via `rust-rocksdb` feature flags with bzip2, lz4, zstd)

**Production:**
- Linux x86_64 or aarch64 (musl or glibc)
- Deployment targets: systemd service (Debian package, RPM package, Arch), Docker/OCI container
- Reverse proxy required (nginx, Caddy, etc.) for standard Matrix port handling
- No external database needed (RocksDB is embedded)

---

*Stack analysis: 2026-03-25*
