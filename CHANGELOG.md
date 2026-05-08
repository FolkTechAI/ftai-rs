# Changelog

## v0.1.1 — 2026-05-07

### Fixed

- **Bug 1 (round-trip):** integer/float/bool fields nested inside an
  internally-tagged enum's struct variant were being passed to the visitor
  as `String` (e.g. `"42"`) instead of the requested type (e.g. `u32`).
  `ValueDeserializer::deserialize_any` now attempts `i64 → u64 → f64 →
  bool → string` parsing on `Unquoted` values before falling back to
  `visit_string`. Found by mitosis-core's Phase 2a `MitosisCluster`
  round-trip.
- **Bug 2 (round-trip):** externally-tagged enums (the default — no
  `#[serde(tag = "...")]`) failed deserialization at top level because
  the serializer emits lowercased section tags (per FTAI's
  case-insensitive tag rule) but serde's variant matcher is case-sensitive.
  `SectionDeserializer::deserialize_enum` now folds case against the
  static `variants` list to recover the canonical case before passing to
  the visitor. Found by mitosis-core's Phase 2b `Actor` /
  `AuditAction` enums.

### Known limitation (deferred)

- Externally-tagged enums **nested inside a struct** still lose their
  variant name on round-trip — the field-name-as-section-tag architecture
  cannot encode both the field name AND the variant name in the same
  section. Workaround: use **internally-tagged** enums
  (`#[serde(tag = "kind")]`) for nested use. mitosis-core's `Actor`,
  `AuditAction`, and `VerificationOutcome` all do this. The test
  `externally_tagged_enum_nested_in_struct_roundtrips` is `#[ignore]`-marked
  with explanatory text. A future serializer rework can add a wrapping
  layer (`@field { @variant { fields } }`) to lift this limitation
  without breaking existing consumers.

### Tests

- 4 new red tests in `tests/serde_roundtrip.rs` covering both bugs and
  the documented limitation. All non-ignored tests pass; the documented
  limitation test is `#[ignore]`-marked.
- Aggregate: 59 passing, 0 failing, 3 ignored across the full crate.

---

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
