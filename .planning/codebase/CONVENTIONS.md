# Coding Conventions

**Analysis Date:** 2026-03-25

## Naming Patterns

**Files:**
- Snake_case: `state_cache`, `event_auth`, `room_member` — all source files use snake_case
- Modules split across directories: `mod.rs` is the default entry point for multi-file modules
- Test files named either `tests.rs` (standalone file) or `mod tests { ... }` (inline block)
- Bench files live in `benches/` subdirectory alongside the source crate

**Structs / Types:**
- PascalCase for all types: `PduEvent`, `MutexMap`, `RoomUpgradeContext`, `TestStateMap`
- Data structs bundling DB maps are named `Data` (private, defined per service)
- Service structs are always named `Service` within their module

**Functions:**
- snake_case for all functions and methods
- Boolean predicates prefixed with `is_` or `has_` or end in `_exists`
- Async variants have the same name as sync with `async fn` — no `_async` suffix in names
- Getter functions use descriptive names (`get_pdu_from_id`, `first_pdu_in_room`) not `get_` prefixes alone

**Variables / Fields:**
- snake_case throughout; no camelCase in Rust code
- Single-letter names accepted in tight iterator/closure scopes (e.g., `|c|`, `|e|`, `|v|`)
- DB map fields named after the key pattern they hold: `roomid_joinedcount`, `pduid_pdu`, `eventid_pduid`

**Constants:**
- SCREAMING_SNAKE_CASE: `SERVER_TIMESTAMP`, `INITIAL_EVENTS`, `BUFSIZE`

## Code Style

**Formatter:** `rustfmt` via `rustfmt.toml`

Key settings:
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

**Linting:** Clippy (workspace-wide rules in `Cargo.toml` `[workspace.lints.clippy]`)

Notable enforced rules:
- `unwrap_used = "warn"` — `unwrap()` is prohibited outside tests (`allow-unwrap-in-tests = true` in `clippy.toml`)
- `get_unwrap = "warn"` — `.get(i).unwrap()` forbidden
- `arithmetic_side_effects = "warn"` — use checked arithmetic
- `as_conversions = "warn"` — explicit `as` casts require `#[expect]`
- `undocumented_unsafe_blocks = "warn"` — all `unsafe` blocks need a safety comment
- `dbg_macro = "warn"` — no `dbg!()` in committed code
- `exit = "warn"` — no `std::process::exit`
- `str_to_string = "warn"` — prefer `.to_owned()` or `String::from()`
- `tests_outside_test_module = "warn"` — test fns must live inside `#[cfg(test)]` module or test file

Lint suppression uses `#[expect(clippy::...)]` (not `#[allow]`) to document intentional exceptions with compiler verification that the suppression is still needed.

**Clippy thresholds** (`clippy.toml`):
- Max function lines: 780 (aspirational target ≤ 100, marked TODO)
- Max cognitive complexity: 100
- Large error threshold: 256 bytes
- Future size threshold: 24576 bytes

## Import Organization

**rustfmt enforces three groups** (`group_imports = "StdExternalCrate"`):

1. `std` / `core` / `alloc` — standard library
2. External crates (alphabetical)
3. `crate` / `super` / `self` — internal paths

**Example from `src/service/rooms/state_cache/mod.rs`:**
```rust
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use futures::{Stream, StreamExt, future::join5, pin_mut};
use ruma::{
    OwnedRoomId, RoomId, ServerName, UserId,
    events::{AnyStrippedStateEvent, AnySyncStateEvent, room::member::MembershipState},
    serde::Raw,
};
use tuwunel_core::{
    Result, implement,
    result::LogErr,
    // ...
};
use tuwunel_database::{Deserialized, Ignore, Interfix, Map};

use crate::appservice::RegistrationInfo;
```

**Re-exports:** `pub use` is used at the module level to flatten internal sub-module types into the public API (e.g., `pub use tuwunel_core::matrix::pdu::{PduId, RawPduId};`).

**Path Aliases:** No `use` path aliases (no `use foo as bar`). Instead, local type aliases via `type Foo = ...` or inline `use BTreeMap as Map` inside function bodies.

## Error Handling

**Strategy:** `Result<T>` is the dominant return type. The project uses a custom `Error` enum in `src/core/error/mod.rs` (the `tuwunel_core::Error`) with `From` implementations for standard and third-party error types.

**Primary macro: `err!` / `Err!`** (defined in `src/core/error/err.rs`)

```rust
// Return a typed request error:
return Err!(Request(NotFound("event not found: {event_id}")));

// Return a database error:
return Err!(Database("Invalid unsigned in pdu event: {e}"));

// Return with simultaneous logging:
return Err!(Request(Forbidden(error!("forbidden: {reason}"))));

// Lower-level — get the Error value without wrapping in Err():
let e = err!(Request(NotFound("...")));
```

**Result extension traits** (in `src/core/utils/result/`):
- `LogErr` — `.log_err()` / `.err_log(level)` — log the error and return `Self`
- `NotFound` — `.is_not_found()` — check if error is a NotFound variant
- `FlatOk` — flatten nested `Result<Result<T>>`
- `MapExpect` — like `.expect()` but maps to an error instead of panicking

**Never use `.unwrap()`** outside of test code. Use `?`, `Err!()`, or `.expect("reason")` only when the value is provably non-failing. Clippy enforces this via `unwrap_used = "warn"`.

**`?` operator** is used throughout for early returns. The `use_try_shorthand = true` rustfmt setting enforces `?` over the old `try!()` macro.

## Logging

**Framework:** `tracing` (re-exported via `tuwunel_core` macros)

**Use project macros, not extern `tracing::` or `log::` crates directly:**
```rust
use tuwunel_core::{error, warn, info, debug, trace};

error!("something went wrong: {e}");
warn!("unexpected state: {msg}");
info!("starting service");
debug!("processing event {event_id}");
trace!("low-level detail");
```

These macros (in `src/core/log/mod.rs`) are thin wrappers over `tracing::` that allow future redirection and are the canonical logging interface.

**Release builds:** Log levels `debug` and `trace` are compiled out under the `release_max_log_level` feature flag to maximize performance.

**Structured logging:** Use field syntax for structured events: `error!(event_id = %id, "message")`.

## Comments

**Module-level docs:** Use `//!` at the top of small focused modules to describe purpose:
```rust
//! Two-Phase Counter.
//! System utilities related to compute/processing
```

**Inline docs:** Use `///` for public items when the purpose isn't self-evident. Many internal items are undocumented (`missing_docs = "allow"` in workspace lints).

**TODO comments:** `// TODO` is common, often with `ALARA` (as low as reasonably achievable) to indicate aspirational reductions. Tracked in `clippy.toml` thresholds.

**Safety comments:** All `unsafe` blocks require a `// SAFETY:` comment above them (`undocumented_unsafe_blocks = "warn"`).

## Function Design

**Size:** Target ≤ 100 lines (current threshold is 780, being reduced — see `clippy.toml`).

**Parameters:** Prefer `&str` / `&T` over owned values for read-only inputs. Use `impl Trait` for generic parameters where appropriate.

**Return Values:** Always `Result<T>` for fallible operations. `-> Result` (without generic) means `Result<()>`. Infallible operations return the concrete type directly.

**Single-line functions** are allowed and common for simple delegating implementations:
```rust
pub async fn get_pdu(&self, event_id: &EventId) -> Result<PduEvent> { self.get(event_id).await }
```

## Module Design

**`#[implement(Type)]` proc-macro** is used to add methods to types defined elsewhere without `impl` blocks in the same file. This splits large impl blocks across multiple files:
```rust
// In src/core/matrix/pdu/unsigned.rs:
#[implement(Pdu)]
pub fn remove_transaction_id(&mut self) -> Result { ... }
```

**Visibility:**
- `pub` — part of the public API between crates or exposed via crate root re-exports
- `pub(crate)` — accessible within the crate
- `pub(super)` — used for test utilities shared between submodules
- Private by default for internal implementation details

**Service struct pattern:** Each service subdirectory exposes a `Service` struct. A private `Data` struct holds `Arc<Map>` database handles. Services hold an `Arc<crate::services::OnceServices>` to access other services.

**Barrel files / re-exports:** Crate roots (`mod.rs` or `lib.rs`) use `pub use` to flatten their public API. External dependencies are frequently re-exported from `tuwunel_core` (e.g., `pub use ::ruma`, `pub use ::arrayvec`).

---

*Convention analysis: 2026-03-25*
