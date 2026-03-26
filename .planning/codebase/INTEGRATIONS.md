# External Integrations

**Analysis Date:** 2026-03-25

## APIs & External Services

**Matrix Federation:**
- Matrix homeserver federation protocol (S2S API)
  - SDK/Client: `ruma` (git fork) federation-api feature
  - Auth: Ed25519 server signing keys (stored in RocksDB, managed by `src/service/server_keys/`)
  - Outbound requests: `reqwest` HTTP client via `src/service/sending/`
  - Inbound: handled by `src/api/` federation routes

**Matrix Appservices:**
- Matrix Application Services (bridging IRC, Telegram, etc.)
  - SDK/Client: `ruma` appservice-api-c feature
  - Auth: `as_token` / `hs_token` per appservice config
  - Config: `[global.appservice.<ID>]` sections in TOML config
  - Outbound notifications: `reqwest` HTTP client, timeout `appservice_timeout` (default 35s)
  - Inbound: appservice registration at `/_matrix/app/v1/`

**Matrix Push Gateway (Notifications):**
- Outbound push notifications to mobile/web clients via push gateways (e.g. ntfy, UnifiedPush, FCM/APNS via Sygnal)
  - SDK/Client: `ruma` push-gateway-api-c feature
  - Implementation: `src/service/pusher/`
  - Auth: no auth; target URL configured by client registration
  - Pool: `pusher_idle_timeout` (default 15s)

**TURN/VoIP Server:**
- External coturn (or compatible) TURN server for WebRTC voice/video
  - Configuration: `turn_uris`, `turn_secret` / `turn_secret_file`, `turn_username`, `turn_password`, `turn_ttl`
  - Auth: HMAC-SHA1 shared secret or static credentials
  - API endpoint served: `/_matrix/client/v3/voip/turnServer`

**URL Preview (oEmbed/Open Graph):**
- Fetches external URLs to generate previews for Matrix messages
  - SDK/Client: `reqwest` + `webpage` crate
  - Feature: `url_preview` (enabled by default)
  - Config: `url_preview_domain_*_allowlist/denylist`, `url_preview_max_spider_size`
  - No external service dependency; scrapes URLs directly

**Matrix.org Trusted Key Servers:**
- Queries trusted notary servers for federation server signing keys
  - Default: `trusted_servers = ["matrix.org"]`
  - Protocol: Matrix key server API
  - Config: `trusted_servers`, `query_trusted_key_servers_first`, `only_query_trusted_key_servers`

## Data Storage

**Databases:**
- RocksDB (embedded key-value store)
  - Connection: Local filesystem path via `database_path` config (default `/var/lib/tuwunel`)
  - Client: `rust-rocksdb` (git fork: `github.com/matrix-construct/rust-rocksdb`)
  - Compression: zstd (default), bzip2, lz4 optional
  - Direct I/O: enabled by default (`rocksdb_direct_io = true`)
  - Backup engine: online backups via `database_backup_path` config
  - No relational database; no SQL

**File Storage:**
- Local filesystem only
  - Media files: stored under `database_path` directory
  - No S3 / object storage integration detected

**Caching:**
- In-process LRU caches (multiple, in `src/service/`)
  - `auth_chain_cache_capacity`, `pdu_cache_capacity`, `shorteventid_cache_capacity`, etc.
  - Scaled by CPU core count; adjustable with `cache_capacity_modifier`
- In-process DNS cache (`hickory-resolver` with configurable TTL)
  - `dns_cache_entries` (default 32768), `dns_min_ttl` (default 10800s)
- No Redis / Memcached

## Authentication & Identity

**Native Matrix Auth:**
- Username + password (argon2 hashed) - `login_with_password` (default enabled)
- Login tokens for session transfer - `login_via_existing_session`, `login_via_token`
- Registration tokens - `registration_token` / `registration_token_file`
- Access token TTL - `access_token_ttl` (default 7 days)
- Implementation: `src/service/globals/`, `src/api/client/`

**JWT Login:**
- Config section: `[global.jwt]`
- Key formats: HMAC (plaintext), B64HMAC, ECDSA PEM, EdDSA Ed25519 PEM
- Algorithms: HS256 (default), configurable
- Library: `jsonwebtoken` 10.3
- Implementation: `src/service/`

**LDAP Login (optional feature `ldap`):**
- Config section: `[global.ldap]`
- URI: `ldap://` or `ldaps://`
- Bind: supports reader bind DN + password file, or direct bind via `{username}` variable
- Search filter, uid/mail/name attribute mapping, admin user detection via `admin_filter`
- Library: `ldap3` (git fork: `github.com/matrix-construct/ldap3`) with rustls TLS
- Implementation: `src/service/`, `src/api/`

**OAuth / SSO / OIDC (identity providers):**
- Config section: `[[global.identity_provider]]` (multiple providers supported)
- Discovery: OpenID Connect `.well-known/openid-configuration` (default auto-discovery)
- Supported brands with built-in defaults: Apple, Facebook, GitHub, GitLab, Google, Keycloak, MAS
- Flow: authorization code grant with callback at `/_matrix/client/unstable/login/sso/callback/<client_id>`
- Session: grant session `check_cookie`, `grant_session_duration` (default 300s)
- User matching: trusted vs untrusted providers, `unique_id_fallbacks` for conflict resolution
- Configurable URL overrides: `authorization_url`, `token_url`, `revocation_url`, `introspection_url`, `userinfo_url`
- Implementation: `src/service/oauth/`, `src/api/`

**OpenID Tokens (Matrix integrations):**
- Short-lived tokens for Matrix account integrations (e.g. Element Integrations / Scalar)
- TTL: `openid_token_ttl` (default 3600s)
- Distinct from OIDC/OAuth login flow

## Monitoring & Observability

**Error Tracking:**
- Sentry.io (optional `sentry_telemetry` feature)
  - SDK: `sentry` 0.46, `sentry-tracing`, `sentry-tower`
  - Default endpoint: `o4509498990067712.ingest.us.sentry.io`
  - Custom endpoint: `sentry_endpoint` config
  - Reports: panics (`sentry_send_panic`), errors (`sentry_send_error`), performance traces
  - Config: `sentry`, `sentry_endpoint`, `sentry_traces_sample_rate` (default 15%), `sentry_attach_stacktrace`, `sentry_filter`
  - Disabled by default (`sentry = false`)

**Distributed Tracing:**
- OpenTelemetry (optional `perf_measurements` feature)
  - SDK: `opentelemetry` 0.31, `opentelemetry_sdk`, `tracing-opentelemetry` 0.32
  - Jaeger integration disabled (pending opentelemetry 0.30 update, commented out in `Cargo.toml`)
- Flame graph profiling: `tracing-flame` 0.2 (optional, config: `tracing_flame`, `tracing_flame_output_path`)

**Logs:**
- `tracing` 0.1.43 + `tracing-subscriber` 0.3
- Output: stdout (default) or stderr (`log_to_stderr`)
- Format: full (default), compact (`log_compact`)
- Level: configured via `log` config key (tracing EnvFilter directives) and `TUWUNEL_LOG` env var
- ANSI colors: `log_colors` (default true)
- systemd journal integration: automatic when `systemd` feature enabled
- Thread IDs: `log_thread_ids` (default false)

**Runtime Metrics:**
- `tokio-metrics` 0.4 - tokio task/runtime metrics
- `tokio-console` (optional, disabled pending axum update) - async task debugging

## CI/CD & Deployment

**Hosting:**
- Self-hosted (the server IS the deployment target for end users)
- Developer CI: GitHub Actions (`.github/workflows/`) and GitLab CI (`.gitlab/`)

**CI Pipeline:**
- GitHub Actions workflows: `main.yml`, `test.yml`, `lint.yml`, `bake.yml`, `package.yml`, `publish.yml`, `autocopr.yml`
- Docker BuildKit bake: `docker/bake.hcl` for multi-platform container builds
- Complement test suite: Matrix spec compliance testing (`docker/Dockerfile.complement`, `docker/complement.sh`)
- Nix flake: `flake.nix` with crane build, Attic/Cachix binary cache support

**Package Distribution:**
- Debian package (`cargo-deb` config in `src/main/Cargo.toml`)
- RPM package (`cargo-generate-rpm` config in `src/main/Cargo.toml`)
- Docker/OCI container images
- Nix package (`default.nix`, `nix/`)
- AUR (Arch Linux) via `autocopr.yml`

## Webhooks & Callbacks

**Incoming:**
- Matrix Client-Server API: `/_matrix/client/`
- Matrix Server-Server (Federation) API: `/_matrix/federation/`
- Matrix Appservice API: `/_matrix/app/v1/`
- Matrix Media API: `/_matrix/media/`
- OAuth SSO callback: `/_matrix/client/unstable/login/sso/callback/<client_id>`
- Well-known delegation: `/.well-known/matrix/server`, `/.well-known/matrix/client`, `/.well-known/matrix/support`

**Outgoing:**
- Federation requests to remote Matrix homeservers (`src/service/sending/`)
- Appservice event pushes to registered appservice URLs
- Push notification delivery to client-registered push gateway URLs
- URL preview HTTP fetches to arbitrary external URLs
- TURN credential requests (served to clients; no outbound)
- LDAP queries to configured LDAP server (when `ldap` feature enabled)
- OAuth/OIDC discovery and token exchange to configured identity providers
- Trusted key server queries to `matrix.org` (or configured servers)

---

*Integration audit: 2026-03-25*
