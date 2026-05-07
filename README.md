# ftai-rs

Rust parser and `serde` adapter for the [FolkTech FTAI v2.0](https://github.com/FolkTechAI/ftai-spec) format.

Sibling of the Python and Swift reference parsers in `ftai-spec`. Foundational FolkTech component consumed by `mitosis-core`, `myelin`, `claude-mesh`, and future Rust crates.

## Install

```toml
[dependencies]
ftai = "0.1"
```

## 30-second example

```rust
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Note {
    title: String,
    body: String,
}

let n = Note { title: "Hi".into(), body: "Hello.".into() };
let text = ftai::to_string(&n)?;
let back: Note = ftai::from_str(&text)?;
assert_eq!(n, back);
```

## Public API

| Function | Purpose |
|---|---|
| `ftai::parse(s)` | Parse raw `&str` to a `Document` AST |
| `ftai::parse_lenient(s)` | Parse with error recovery; returns `(Document, Vec<Error>)` |
| `ftai::to_string(value)` | Serialize a `Serialize` value to FTAI text |
| `ftai::from_str(s)` | Deserialize FTAI text into a `DeserializeOwned` value |
| `ftai::to_string_doc(doc)` | Serialize a `Document` AST directly |

## What's NOT in v1 (by design)

- Streaming / incremental parsing
- Schema validation (lives in `ftai-spec/schema/`)
- Multi-version support (v2.0 only)
- Async / WASM / `no_std`
- C-ABI / Swift FFI bindings

See `docs/specs/2026-05-07-ftai-rs-v1-design.md` for the full out-of-scope list and rationale.

## Contributing

Format checks: `cargo fmt --check && cargo clippy --all-targets -- -D warnings`.
Tests: `cargo test --all-targets`.
Python parity test (optional, requires `python3`): `cargo test --test parity_python -- --ignored`.

PRs welcome. The crate has `#![forbid(unsafe_code)]` and only two production dependencies (`serde`, `thiserror`) — please keep both invariants.

## License

Apache-2.0 — matches `FolkTechAI/ftai-spec`.
