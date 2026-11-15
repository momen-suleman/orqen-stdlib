# orqen-stdlib

The Orqen standard library: small, deterministic, typed operations that
Orqen workflows are built from, and the first published micro-workflows
built from them.

```text
std.text.*        trim, lowercase, uppercase, split, join, replace, contains, contains_any, length,
                  normalize_unicode, normalize_line_endings, collapse_whitespace
std.number.*      sum, add, subtract, multiply, divide, round
std.logic.*       equal, not_equal, greater_than, less_than, and, or, not, choose
std.collection.*  get, max, sort, contains, count, append, slice
std.object.*      get, keys, values, from_lists, set
std.type.*        to_number, to_text, validate
std.error.raise
```

Every operation is pure (no I/O, no clock, no randomness, no permissions)
and has a contract: typed inputs with defaults, typed outputs and error
codes. `call` validates inputs against the contract before running and
checks outputs after, so a contract cannot be broken silently.

```rust
use serde_json::json;
let out = orqen_stdlib::call("std.number.divide", json!({"a": 25, "b": 200}).as_object().unwrap())?;
assert_eq!(out["result"], 0.125);
```

## Numeric model

Integers are exact `i64` (overflow is an error, never wraps); decimals are
finite binary64; `divide` by zero fails with `division_by_zero`; `round`
rounds the shortest decimal form half away from zero (`1.005` → `1.01`).
Text is never turned into a number implicitly: use `std.type.to_number`.

## Workflows

`workflows/` is an Orqen Library: importing this folder installs and
publishes

| Id | What |
|---|---|
| `wf.std.text.normalize_input` 1.0.0 | NFC, LF line endings, trim |
| `wf.std.text.normalize_search_query` 1.0.0 | `normalize_input` (pinned 1.0.0), lowercase, collapse whitespace |
| `wf.std.number.calculate_percentage` 1.0.0 | checks `total ≠ 0`, divides, × 100, rounds; `invalid_total` error otherwise |

```bash
orqen library import path/to/orqen-stdlib
orqen publications run wf.std.number.calculate_percentage '{"value": 25, "total": 200}'
```

## In Orqen

Orqen compiles this crate in (`crates/orqen-core/Cargo.toml`) and exposes
each operation as a Canvas node through `std_capabilities.rs`. Operations
appear in the palette under **Standard Library** automatically. See
Orqen's `docs/STDLIB.md` and `docs/PUBLISHING.md`.

## Develop

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
```

Add an operation: write the function and its `Operation` entry in the
matching module (`src/text.rs`, …), keep it pure, test it there, bump the
version. Ids are `std.<area>.<verb>` and never change meaning.

## Trust

This crate is first-party code compiled into the Orqen engine (trusted,
in-process). It is not a mechanism for loading third-party code: those run
out of process as external HTTP capabilities.
