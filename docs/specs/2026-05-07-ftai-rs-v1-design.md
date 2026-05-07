# Spec — ftai-rs v1 (Rust FTAI v2.0 parser)

**Spec ID:** FTAI-RS-001
**Date:** 2026-05-07
**Status:** Draft — awaiting project-owner approval before plan + implementation

---

## 1. Intent

A Rust crate that reads and writes FTAI v2.0 documents and exposes a `serde` adapter so any `#[derive(Serialize, Deserialize)]` type can round-trip via FTAI text.

**Why now:** Mitosis v2 Phase 2 needs FTAI for storage rows, events, audit records, and config loading. Path 0 (this crate) is the unblocking dependency. Once landed, mitosis-core's `[dev-dependencies] serde_json` is replaced with `ftai-rs` and the FTAI canonical ("FTAI everywhere — no JSON translation layer") becomes provably true in production.

**Sibling-crate posture:** `FolkTechAI/ftai-spec` is the canonical format spec + Python and Swift reference parsers. `ftai-rs` is the Rust sibling. License (Apache-2.0) matches the parent.

**Consumers (known v1):** mitosis-core, myelin (transitively), claude-mesh (transitively, for FTAI-structured cross-session knowledge), future Rust FolkTech crates.

## 2. Constraints

| # | Constraint | Why |
|---|---|---|
| C1 | Format compliance: full FTAI v2.0 spec per `ftai-spec/spec.md` and `ftai-spec/grammar/ftai.ebnf` | Aligns with Python + Swift siblings; no surprise feature gaps |
| C2 | Rust edition 2021, stable toolchain ≥ 1.94 | Matches workspace baseline (Mitosis-Clustering, myelin) |
| C3 | Production deps: `serde`, `thiserror` only — **no parser-combinator crate** | The EBNF is 15 lines; nom/chumsky/pest add cargo-tree weight without paying back |
| C4 | License: Apache-2.0 (matches `ftai-spec`) | Project-owner decision 2026-05-07 |
| C5 | Synchronous API; no async | FFI-friendly; matches Mitosis v2's design principle |
| C6 | FFI-friendly public surface: simple types, `Result`-returning fns, no panics on the public API | Future Swift FFI / WASM targets without redesign |
| C7 | Zero `unsafe` code | Defensive posture for a foundational crate; assert via `#![forbid(unsafe_code)]` at crate root |
| C8 | All public items carry doc-comments | Library consumability + rustdoc generation |
| C9 | Zero clippy warnings under `-D warnings` on Rust 1.94+ | Matches FolkTech workspace standard |
| C10 | Production code is JSON-free | FTAI canonical alignment; `serde_json` may appear only in `[dev-dependencies]` for round-trip testing scaffolding |

## 3. Acceptance Criteria

A1. **Round-trip fidelity on the official corpus.** Every `.ftai` file in `FolkTechAI/ftai-spec/tests/` parses to a `Document` AST and serializes back to text that re-parses to the structurally-equal AST. (Whitespace within unquoted values is the only non-significant variation tolerated.)

A2. **Python-parser parity on a fixture corpus.** For a curated corpus of representative `.ftai` documents, `ftai_rs::parse(input)` produces a `Document` whose JSON-serialized form is byte-equal to the Python reference parser's JSON output. Equivalence relation: keys + values match; key order is order-of-appearance.

A3. **EBNF conformance suite.** `tests/grammar_conformance.rs` enumerates every production in `ftai-spec/grammar/ftai.ebnf` and includes at least one accepting input and at least one rejecting input per production.

A4. **`serde` derive coverage.** Types annotated with `#[derive(Serialize, Deserialize)]` round-trip via `ftai::to_string` → `ftai::from_str` for: primitives (bool, integers, floats, String), `Option<T>`, `Vec<T>`, `HashMap<String, T>`, structs, tagged enums (mapping to FTAI's `@tag` semantics), nested structs, and types containing all of the above.

A5. **Fault-tolerant mode.** `parse_lenient(input) -> (Document, Vec<Error>)` recovers from these classes of malformedness per the FTAI spec's "fault-tolerant" design principle:
- Unknown tags: ignored, parser continues
- Missing `@end` markers: section closed at next `@tag` or EOF, error logged
- Stray whitespace inside structured blocks: tolerated
- Truncated final block: structurally valid prefix preserved, error appended

A6. **Security CAT 1 (Input Injection) red tests.** Each of the following input classes MUST be rejected without panicking, with a precise error category. Minimum 5 red tests in `tests/red_cat1_input.rs`, one per bullet:
- Null bytes in any position
- Tag names exceeding the documented max length (configurable; default 256 bytes)
- Nested blocks deeper than the documented limit (configurable; default 64)
- Control characters inside values
- Malformed UTF-8 byte sequences

A7. **Security CAT 7 (LLM Output handling) parser hygiene.** Parser MUST NOT silently absorb structural malformedness in `@ai` blocks (truncated `@end`, unbalanced delimiters, embedded `@end` inside quoted strings being misread as a section close). Such cases produce an error in either strict or lenient mode — never a silent recovery that drops content. Note: prompt-injection prevention itself is a *consumer* concern (the consumer that feeds parsed `@ai` content to a tool dispatcher is responsible for CAT 7). This crate's responsibility is faithfulness — parsed output reflects the bytes that were there.

A11. **Determinism.** `to_string(value)` is a pure function: same input value, same output bytes, every time. No timestamps, no UUIDs, no environment-derived content in the output unless explicitly part of the value.

A8. **Coverage.** ≥80% line coverage (measured via `cargo llvm-cov` or `cargo tarpaulin`).

A9. **CI.** Tests pass on macOS-arm64 and linux-x86_64 via GitHub Actions on every push and PR.

A10. **Documentation.** A `README.md` with: 30-second usage example, link to ftai-spec, contributing guide, license. A `CHANGELOG.md` with v0.1.0 entry. Rustdoc renders without warnings.

## 4. Out of Scope (v1)

- **Streaming / incremental parser.** Entire document held in memory. The format is small; batch reads dominate. (Promote in v1.1+ when a real consumer needs it.)
- **Schema validation.** Schemas live in `ftai-spec/schema/`. This crate parses + serializes; validation is a separate consumer or future sibling crate.
- **JSON export interop.** Consumers can serde-derive both FTAI and JSON for the same type if they want; this crate doesn't ship a JSON adapter.
- **Multi-version format support.** v2.0 only. Documents declaring older `@ftai vN` are rejected at parse time with a clear error.
- **Async / tokio integration.** Sync only.
- **WASM / `no_std`.** Standard library required; WASM target may be added later if a browser consumer arrives.
- **C-ABI / Swift FFI bindings.** Swift consumers use the existing Swift parser. Bindings to ftai-rs are a separate decision (own spec, own tech-stack-authorization gate).
- **Pretty-printing options.** Single canonical output format. Configurable formatting can be added when multiple consumers express conflicting preferences.
- **Inline `[name:value]` flag normalization beyond parsing.** v1 surfaces inline flags as data on the AST; semantic interpretation is the consumer's concern.

## 5. Open Questions

None blocking spec finalization. Implementation-time questions (parser-combinator vs hand-rolled trade-offs at the lexer level, error-reporting formats, exact AST shape) are deferred to the implementation plan and resolved during TDD execution.

---

## Reference index

- **Format spec:** `FolkTechAI/ftai-spec` repo, `spec.md` (v2.0)
- **Grammar (EBNF):** `FolkTechAI/ftai-spec` repo, `grammar/ftai.ebnf`
- **Python reference parser:** `FolkTechAI/ftai-spec` repo, `parsers/python/parseftai_linter.py`
- **Swift reference parser:** `FolkTechAI/ftai-spec` repo, `parsers/swift/FTAIParser.swift` + `FTAIValidator.swift`
- **Sample documents:** `FolkTechAI/ftai-spec` repo, `parsers/python/sample_valid.ftai` and `tests/`
- **Sibling work (Mitosis v2):** `FolkTechAI/Mitosis-Clustering` repo, `crates/mitosis-core/` — first internal consumer of ftai-rs once both ship

---

## Project-owner sign-off

**Status:** Awaiting approval. After approval, the next step is `writing-plans` skill → implementation plan → subagent dispatch → PR.
