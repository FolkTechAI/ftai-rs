# Changelog

## v0.1.0 — 2026-05-07 (planned)

### Added
- FTAI v2.0 parser (hand-rolled lexer + recursive-descent parser).
- `serde::Serializer` / `Deserializer` adapter for typed round-trips of
  primitives, `Option<T>`, `Vec<T>`, `HashMap<String, T>`, structs, nested
  structs, and tagged enums.
- `parse_lenient` mode for fault-tolerant parsing.
- CAT 1 (input injection) hardening: null bytes, oversize tag names,
  control characters, malformed UTF-8 — all rejected with
  `Error::InputInjection`.
- CAT 1 (limit exceedance): nesting-depth limit (default 64) enforced in
  the parser with `Error::NestingTooDeep`.
- CAT 7 (LLM output handling): parser hygiene tests for truncated `@end`,
  embedded `@end` inside quoted strings, and unbalanced quotes.
- EBNF conformance suite: 1 accept + 1 reject test per grammar production.
- Round-trip on the upstream `ftai-spec` corpus.
- Python reference-parser parity test (run with `--ignored`, requires
  `python3`).

### Constraints (per spec FTAI-RS-001)
- Apache-2.0 license.
- `#![forbid(unsafe_code)]`.
- Production deps: `serde`, `thiserror` only.
- Sync API only.
- Zero clippy warnings under `-D warnings` on Rust 1.94+.
