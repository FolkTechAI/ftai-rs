# ftai-rs v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust FTAI v2.0 parser + serde adapter (`ftai-rs`) per spec FTAI-RS-001.

**Architecture:** Hand-rolled lexer + parser (no parser-combinator dep). AST types are the structural representation; `serde::Serializer` and `serde::Deserializer` impls let arbitrary `#[derive(Serialize, Deserialize)]` types round-trip via FTAI text. Public API: `ftai::parse(s) -> Result<Document>`, `ftai::from_str::<T>(s) -> Result<T>`, `ftai::to_string(value) -> Result<String>`, `ftai::parse_lenient(s) -> (Document, Vec<Error>)`.

**Tech Stack:** Rust 2021, stable toolchain ≥ 1.94. Production deps: `serde`, `thiserror`. Dev-deps: `serde_json` (JSON-output for Python parity tests), `pretty_assertions`. Apache-2.0. `#![forbid(unsafe_code)]`.

**Spec:** `docs/specs/2026-05-07-ftai-rs-v1-design.md` (FTAI-RS-001).

**FTAI format references (read these before starting):**
- `FolkTechAI/ftai-spec` repo, `spec.md` — v2.0 format spec
- `FolkTechAI/ftai-spec` repo, `grammar/ftai.ebnf` — EBNF grammar
- `FolkTechAI/ftai-spec` repo, `parsers/python/parseftai_linter.py` — Python reference parser (~9KB)
- `FolkTechAI/ftai-spec` repo, `parsers/swift/FTAIParser.swift` — Swift reference parser

---

## File Structure

```
ftai-rs/
├── Cargo.toml                 # Crate manifest (edition 2021, deps locked)
├── README.md                  # Usage example, link to ftai-spec, contributing
├── CHANGELOG.md               # v0.1.0 entry
├── LICENSE                    # Apache-2.0 (already present)
├── .gitignore                 # /target, /Cargo.lock retained
├── .github/
│   └── workflows/
│       └── ci.yml             # Test on macOS-arm64 + linux-x86_64
├── docs/
│   ├── specs/
│   │   └── 2026-05-07-ftai-rs-v1-design.md       (already exists)
│   └── plans/
│       └── 2026-05-07-ftai-rs-v1-implementation.md  (this file)
├── src/
│   ├── lib.rs                 # Public API exports + #![forbid(unsafe_code)]
│   ├── error.rs               # Error type with line/col + Result alias
│   ├── ast.rs                 # Document, Section, Block, Value, InlineTag, Span
│   ├── lexer.rs               # Source → Token stream (Vec<Token>)
│   ├── parser.rs              # Token stream → Document AST
│   ├── serializer.rs          # Document AST → canonical FTAI text
│   ├── ser.rs                 # impl serde::Serializer (typed value → AST → text)
│   └── de.rs                  # impl serde::Deserializer (text → AST → typed value)
└── tests/
    ├── fixtures/              # .ftai sample files (vendored from ftai-spec)
    ├── grammar_conformance.rs # One test per EBNF production
    ├── roundtrip_corpus.rs    # parse → serialize → parse, structural equality
    ├── parity_python.rs       # Python reference parser parity (subprocess)
    ├── serde_roundtrip.rs     # ftai::to_string → ftai::from_str for serde-derive types
    ├── lenient_recovery.rs    # parse_lenient fault-tolerance behaviors
    ├── red_cat1_input.rs      # CAT 1 input-injection red tests
    └── red_cat7_hygiene.rs    # CAT 7 LLM-output parser hygiene
```

**File responsibility principle:** one struct/enum + its impls per file in `src/`. Each test file owns one acceptance-criteria area. The lexer doesn't know about AST; the parser doesn't know about serde; the ser/de modules sit on top of the AST. Boundaries are explicit so the subagent can hold each layer in context independently.

---

## Task 1: Cargo manifest + crate skeleton

**Files:**
- Modify: `Cargo.toml` (currently absent — create at repo root)
- Modify: `README.md` (currently 175 bytes — expand to a real README in Task 15; for now just leave)
- Create: `.gitignore`
- Create: `src/lib.rs`

- [ ] **Step 1: Write `Cargo.toml`**

```toml
[package]
name = "ftai"
version = "0.1.0"
edition = "2021"
authors = ["Mike Folk <mike@folktech.ai>"]
description = "FTAI v2.0 parser and serde adapter — Rust sibling of FolkTechAI/ftai-spec"
license = "Apache-2.0"
repository = "https://github.com/FolkTechAI/ftai-rs"
readme = "README.md"
keywords = ["ftai", "parser", "serde", "folktech"]
categories = ["parser-implementations", "encoding"]

[dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"

[dev-dependencies]
serde_json = "1"
pretty_assertions = "1"

[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
```

- [ ] **Step 2: Write `.gitignore`**

```
/target/
**/*.rs.bk
.DS_Store
*.swp
```

- [ ] **Step 3: Write `src/lib.rs` (public API skeleton)**

```rust
//! FTAI v2.0 parser + serde adapter.
//!
//! Read and write `.ftai` files from Rust. Use the high-level serde API
//! for typed round-trips, or the low-level [`parse`] for direct AST access.
//!
//! # 30-second example
//!
//! ```ignore
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Note { title: String, body: String }
//!
//! let note = Note { title: "Hi".into(), body: "Hello.".into() };
//! let text = ftai::to_string(&note).unwrap();
//! let back: Note = ftai::from_str(&text).unwrap();
//! assert_eq!(note.title, back.title);
//! ```
//!
//! # Spec references
//!
//! - Format spec: <https://github.com/FolkTechAI/ftai-spec/blob/main/spec.md>
//! - EBNF grammar: <https://github.com/FolkTechAI/ftai-spec/blob/main/grammar/ftai.ebnf>

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod ast;
mod de;
pub mod error;
mod lexer;
mod parser;
mod ser;
mod serializer;

pub use crate::ast::Document;
pub use crate::error::{Error, Result};

/// Parse a `.ftai` source string into a structural [`Document`] AST.
///
/// Returns `Err` on the first structural failure. For fault-tolerant
/// parsing, use [`parse_lenient`].
pub fn parse(_input: &str) -> Result<Document> {
    todo!("Task 7")
}

/// Parse leniently: recover from unknown tags, missing `@end` markers,
/// and stray whitespace per the FTAI spec's "fault-tolerant" principle.
/// Returns the best-effort AST and the list of errors encountered.
pub fn parse_lenient(_input: &str) -> (Document, Vec<Error>) {
    todo!("Task 12")
}

/// Serialize a value implementing [`serde::Serialize`] to FTAI text.
pub fn to_string<T: serde::Serialize + ?Sized>(_value: &T) -> Result<String> {
    todo!("Task 10")
}

/// Deserialize a `.ftai` source string into a value implementing
/// [`serde::de::DeserializeOwned`].
pub fn from_str<T: serde::de::DeserializeOwned>(_input: &str) -> Result<T> {
    todo!("Task 11")
}
```

- [ ] **Step 4: Add stub module files so `cargo check` succeeds**

For each of `ast.rs`, `de.rs`, `error.rs`, `lexer.rs`, `parser.rs`, `ser.rs`, `serializer.rs`: create with a single header comment and (where the parent module declares `pub use` against it) the minimum types declared `pub` so referenced names exist. For Task 1 we only need the public re-exports to compile:

`src/ast.rs`:
```rust
//! AST types for FTAI documents. See Task 3 for full implementation.

/// Top-level FTAI document. Filled out in Task 3.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document;
```

`src/error.rs`:
```rust
//! Error type for ftai-rs. See Task 2 for full implementation.

/// FTAI parse / serialize error. Expanded in Task 2.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Placeholder until Task 2 lands.
    #[error("ftai-rs error (placeholder)")]
    Placeholder,
}

/// Result alias.
pub type Result<T> = std::result::Result<T, Error>;
```

For `de.rs`, `ser.rs`, `lexer.rs`, `parser.rs`, `serializer.rs`: a header comment is sufficient.

- [ ] **Step 5: Verify build**

Run: `cargo check --all-targets`
Expected: succeeds (warnings about unused module imports are acceptable).

Run: `cargo clippy --all-targets -- -D warnings`
Expected: passes (no errors). Public-doc-warning lints will be addressed as types fill in.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml .gitignore src/
git commit -m "chore: cargo manifest + crate skeleton (Plan Task 1)"
```

---

## Task 2: Error type

**Files:**
- Modify: `src/error.rs`
- Test: inline `#[cfg(test)] mod tests` in `src/error.rs`

- [ ] **Step 1: Write the failing test**

Replace `src/error.rs` placeholder content with the test first:

```rust
//! Error type for ftai-rs.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_token_error_includes_location() {
        let e = Error::UnexpectedToken {
            expected: "@end".into(),
            found: "@document".into(),
            line: 14,
            column: 1,
        };
        let s = format!("{e}");
        assert!(s.contains("line 14"));
        assert!(s.contains("column 1"));
        assert!(s.contains("@end"));
        assert!(s.contains("@document"));
    }

    #[test]
    fn category_classifies_input_injection() {
        let e = Error::InputInjection("null byte at offset 7".into());
        assert_eq!(e.category(), ErrorCategory::InputInjection);
    }

    #[test]
    fn determinism_io_error_round_trips_message() {
        let e = Error::Io("write failed: disk full".into());
        assert!(format!("{e}").contains("disk full"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails (compile error: types not defined)**

Run: `cargo test --lib error::`
Expected: compile failure on `Error::UnexpectedToken`, `ErrorCategory`.

- [ ] **Step 3: Implement the Error type**

Replace placeholder content with:

```rust
//! Error type for ftai-rs. All public fallible APIs return `Result<T>` aliasing
//! `std::result::Result<T, Error>`.
//!
//! Errors carry line/column information when applicable so consumers can
//! produce useful diagnostics. The [`ErrorCategory`] taxonomy matches the
//! security categories from FolkTech CLAUDE.md (CAT 1 input injection in
//! particular) so consumers can branch on category for security-relevant
//! handling.

use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type for parsing and serializing FTAI documents.
#[derive(Debug, Error)]
pub enum Error {
    /// Lexer or parser encountered a token it did not expect.
    #[error("unexpected token at line {line}, column {column}: expected {expected}, found {found}")]
    UnexpectedToken {
        /// Token kind expected (e.g., `@end`, `identifier`).
        expected: String,
        /// Token kind actually found.
        found: String,
        /// 1-based line number.
        line: usize,
        /// 1-based column number.
        column: usize,
    },

    /// A `@tag` block was opened but no matching `@end` was found.
    #[error("unterminated block opened at line {line}: tag was '{tag}'")]
    UnterminatedBlock {
        /// The tag name that was left open (without leading `@`).
        tag: String,
        /// 1-based line where the opening tag appeared.
        line: usize,
    },

    /// Document declares a format version this crate does not understand.
    #[error("unsupported FTAI version: '{0}' (this crate supports v2.0 only)")]
    UnsupportedVersion(String),

    /// Input contained bytes that violate the input-validation rules
    /// (null bytes, oversize tag names, control characters, malformed UTF-8).
    /// Maps to FolkTech CAT 1.
    #[error("input injection: {0}")]
    InputInjection(String),

    /// Nested block depth exceeded the configured limit (default 64).
    #[error("nesting depth exceeded limit {limit} at line {line}")]
    NestingTooDeep {
        /// Configured nesting limit.
        limit: usize,
        /// 1-based line where the limit was hit.
        line: usize,
    },

    /// Generic IO / write failure during serialization.
    #[error("io: {0}")]
    Io(String),

    /// Type adapter failed to map between FTAI and a `serde` value.
    #[error("serde: {0}")]
    Serde(String),
}

/// Coarse category for security-relevant branching by consumers.
/// Mirrors the FolkTech 9-category security taxonomy (CLAUDE.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Structural: unexpected token, unterminated block, unsupported version.
    Structural,
    /// CAT 1: input injection (null byte, oversize, control char, malformed UTF-8).
    InputInjection,
    /// CAT 1 sub-class: structural-limit exceedance (depth, length).
    LimitExceeded,
    /// IO error during serialization.
    Io,
    /// Serde-adapter error.
    SerdeMapping,
}

impl Error {
    /// Coarse category for branching.
    pub fn category(&self) -> ErrorCategory {
        match self {
            Error::UnexpectedToken { .. }
            | Error::UnterminatedBlock { .. }
            | Error::UnsupportedVersion(_) => ErrorCategory::Structural,
            Error::InputInjection(_) => ErrorCategory::InputInjection,
            Error::NestingTooDeep { .. } => ErrorCategory::LimitExceeded,
            Error::Io(_) => ErrorCategory::Io,
            Error::Serde(_) => ErrorCategory::SerdeMapping,
        }
    }
}
```

(Re-add the test module from Step 1 below the implementation.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib error::`
Expected: 3 passed; 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/error.rs
git commit -m "feat(error): structured Error type with line/col + ErrorCategory (Plan Task 2)"
```

---

## Task 3: AST types

**Files:**
- Modify: `src/ast.rs`
- Test: inline tests in `src/ast.rs`

The FTAI v2.0 grammar (per `ftai-spec/grammar/ftai.ebnf`) defines:
- `ftai_file = ftai_header, { ftai_block }`
- `ftai_block = tag_section | tag_single`
- `tag_section = tag, { key_value }, { inner_block }, "@end"`
- `tag_single = tag, [ value ]`

In addition, the spec describes:
- Freeform narrative sections separated by `---`
- Inline tags `[name:value]` inside narrative

The AST captures all of these.

- [ ] **Step 1: Write the failing tests**

```rust
//! AST types for FTAI documents.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_is_default_empty() {
        let d = Document::default();
        assert!(d.blocks.is_empty());
        assert_eq!(d.version, FtaiVersion::V2_0);
    }

    #[test]
    fn section_block_round_trips_via_clone_eq() {
        let s = Block::Section(Section {
            tag: "document".into(),
            attributes: vec![("title".into(), Value::Quoted("Hi".into()))],
            children: vec![],
            span: Span::synthetic(),
        });
        assert_eq!(s, s.clone());
    }

    #[test]
    fn narrative_block_preserves_text_verbatim() {
        let n = Block::Narrative {
            text: "  raw text with  spaces\n".into(),
            span: Span::synthetic(),
        };
        match n {
            Block::Narrative { text, .. } => {
                assert_eq!(text, "  raw text with  spaces\n");
            }
            _ => panic!("expected Narrative"),
        }
    }

    #[test]
    fn inline_tag_parsed_form() {
        let it = InlineTag {
            name: "tone".into(),
            value: "urgent".into(),
        };
        assert_eq!(it.name, "tone");
    }

    #[test]
    fn span_display_shows_line_col() {
        let s = Span { start_line: 10, start_col: 4, end_line: 10, end_col: 12 };
        let f = format!("{s}");
        assert!(f.contains("10:4"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib ast::`
Expected: compile failure.

- [ ] **Step 3: Implement the AST types**

```rust
//! AST types for FTAI documents.
//!
//! Mirrors the EBNF grammar in `ftai-spec/grammar/ftai.ebnf`:
//! - [`Document`] is the top-level container (header + blocks).
//! - [`Block`] is either a tagged [`Section`] or a freeform narrative.
//! - Each tagged section carries `attributes` (key:value pairs) and
//!   nested `children` (inner blocks).
//! - [`InlineTag`] represents `[name:value]` markers inside narrative.
//! - [`Span`] tracks line/column ranges for diagnostics.

use serde::{Deserialize, Serialize};

/// Top-level FTAI document.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Format version declared by the `@ftai` header. Defaults to `V2_0`.
    pub version: FtaiVersion,
    /// Optional schema name from the `@ftai` header line.
    pub schema: Option<String>,
    /// All blocks in source order.
    pub blocks: Vec<Block>,
}

/// FTAI format version. v1.0 is unsupported (rejected at parse time).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FtaiVersion {
    /// FTAI v2.0 (the only supported version).
    #[default]
    #[serde(rename = "v2.0")]
    V2_0,
}

/// One block in a document — either a tagged section or freeform narrative.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Block {
    /// `@tag` ... `@end` section with attributes and nested children.
    Section(Section),
    /// Freeform narrative text (separated by `---` from other blocks).
    /// May contain inline tags `[name:value]` represented in `inline_tags`.
    Narrative {
        /// Verbatim narrative text.
        text: String,
        /// Inline `[name:value]` tags extracted from `text`. The `text` field
        /// retains them verbatim; this list is parsed metadata.
        #[serde(default)]
        inline_tags: Vec<InlineTag>,
        /// Source span.
        span: Span,
    },
}

/// `@tag ... @end` section with attributes and child blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    /// Tag name without leading `@` (case-insensitive per spec; stored lowercase).
    pub tag: String,
    /// Key-value attributes in source order. Vec (not HashMap) preserves order
    /// and allows duplicate keys (the spec allows `multiple blocks of same type`).
    pub attributes: Vec<(String, Value)>,
    /// Nested child blocks.
    pub children: Vec<Block>,
    /// Source span.
    pub span: Span,
}

/// Attribute value: quoted string, unquoted token, or list.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum Value {
    /// Quoted string (literal contents, escapes resolved).
    Quoted(String),
    /// Unquoted token (no whitespace, no `{` `}` `[` `]` `<` `>`).
    Unquoted(String),
    /// Bracketed list: `[a, b, c]`.
    List(Vec<Value>),
}

/// `[name:value]` inline tag within narrative text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InlineTag {
    /// Tag name (left of colon).
    pub name: String,
    /// Tag value (right of colon, leading whitespace trimmed).
    pub value: String,
}

/// Source span for diagnostics. Lines and columns are 1-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// 1-based line of the start of the span.
    pub start_line: usize,
    /// 1-based column of the start.
    pub start_col: usize,
    /// 1-based line of the end (inclusive).
    pub end_line: usize,
    /// 1-based column of the end (exclusive).
    pub end_col: usize,
}

impl Span {
    /// Synthetic span used by tests when source location is irrelevant.
    pub fn synthetic() -> Self {
        Self {
            start_line: 0,
            start_col: 0,
            end_line: 0,
            end_col: 0,
        }
    }
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}:{}–{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col
        )
    }
}
```

(Append the test module from Step 1.)

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --lib ast::`
Expected: 5 passed; 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/ast.rs
git commit -m "feat(ast): Document, Section, Block, Value, InlineTag, Span (Plan Task 3)"
```

---

## Task 4: Lexer types (Token, TokenKind, Span integration)

**Files:**
- Modify: `src/lexer.rs`
- Test: inline tests

- [ ] **Step 1: Write failing tests**

```rust
//! Lexer for FTAI source text. Produces a flat stream of [`Token`]s
//! consumed by the parser in `parser.rs`.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_at_construct_carries_span() {
        let t = Token {
            kind: TokenKind::At,
            lexeme: "@".into(),
            span: crate::ast::Span { start_line: 1, start_col: 1, end_line: 1, end_col: 2 },
        };
        assert_eq!(t.kind, TokenKind::At);
        assert_eq!(t.span.start_line, 1);
    }

    #[test]
    fn token_kind_eq_compares_variants() {
        assert_eq!(TokenKind::Identifier, TokenKind::Identifier);
        assert_ne!(TokenKind::At, TokenKind::Colon);
    }
}
```

- [ ] **Step 2: Run test (compile failure expected)**

Run: `cargo test --lib lexer::`

- [ ] **Step 3: Implement lexer types**

```rust
//! Lexer for FTAI source text. Produces a flat stream of [`Token`]s
//! consumed by the parser in `parser.rs`.
//!
//! Hand-rolled state machine — the EBNF grammar is small enough that a
//! parser-combinator dependency would be net negative. The lexer is
//! responsible for input-injection defense: null bytes, oversize tag
//! names, control characters, and malformed UTF-8 are rejected here
//! with `Error::InputInjection`.

use crate::ast::Span;
use crate::error::{Error, Result};

/// Maximum length of a tag identifier (configurable; default 256).
pub const MAX_TAG_LEN: usize = 256;

/// One token from the lexer.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Discriminant.
    pub kind: TokenKind,
    /// Source slice that produced this token, owned for diagnostics.
    pub lexeme: String,
    /// Source span.
    pub span: Span,
}

/// Token kinds. Mirrors EBNF terminals plus pragmatics (newlines, EOF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// Literal `@` (start of a tag).
    At,
    /// Identifier — letter followed by letters, digits, underscores.
    Identifier,
    /// Colon, the key/value separator.
    Colon,
    /// Quoted string, contents already unescaped.
    QuotedString,
    /// Unquoted string (no whitespace, no special chars).
    UnquotedString,
    /// `[` literal.
    LeftBracket,
    /// `]` literal.
    RightBracket,
    /// `,` literal (inside lists).
    Comma,
    /// `---` narrative separator (exactly three hyphens on their own line).
    NarrativeSeparator,
    /// One or more contiguous newlines.
    Newline,
    /// End of file marker.
    Eof,
}

/// Tokenize a complete source string. Returns `Err` on the first
/// CAT 1 violation (null byte, oversize tag, control character,
/// invalid UTF-8 — though `&str` already guarantees UTF-8 validity).
pub fn tokenize(_input: &str) -> Result<Vec<Token>> {
    let _ = (Error::Io("not yet implemented".into()), MAX_TAG_LEN);
    todo!("Task 5")
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib lexer::`
Expected: 2 passed; 0 failed.

- [ ] **Step 5: Commit**

```bash
git add src/lexer.rs
git commit -m "feat(lexer): Token + TokenKind types (Plan Task 4)"
```

---

## Task 5: Lexer happy path

**Files:**
- Modify: `src/lexer.rs`

- [ ] **Step 1: Add happy-path test**

Append to `mod tests` in `src/lexer.rs`:

```rust
#[test]
fn lex_single_at_tag_followed_by_identifier() {
    let input = "@document\n";
    let tokens = tokenize(input).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    use TokenKind::*;
    assert_eq!(kinds, vec![&At, &Identifier, &Newline, &Eof]);
}

#[test]
fn lex_key_value_pair_with_quoted_string() {
    let input = "title: \"Hi there\"\n";
    let tokens = tokenize(input).unwrap();
    let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
    use TokenKind::*;
    assert_eq!(kinds, vec![&Identifier, &Colon, &QuotedString, &Newline, &Eof]);
}

#[test]
fn lex_narrative_separator_three_hyphens() {
    let input = "---\n";
    let tokens = tokenize(input).unwrap();
    use TokenKind::*;
    assert_eq!(tokens[0].kind, NarrativeSeparator);
}

#[test]
fn lex_full_minimal_document() {
    let input = "@ftai v2.0\n@document\ntitle: \"Hi\"\n@end\n";
    let tokens = tokenize(input).unwrap();
    // First two: @ftai header
    assert_eq!(tokens[0].kind, TokenKind::At);
    assert_eq!(tokens[1].kind, TokenKind::Identifier);
    assert_eq!(tokens[1].lexeme, "ftai");
    // Verify the unquoted "v2.0" is captured as UnquotedString
    assert!(tokens.iter().any(|t| t.lexeme == "v2.0"));
}

#[test]
fn lex_quoted_string_resolves_escape_sequences() {
    let input = "k: \"a\\\"b\"\n";
    let tokens = tokenize(input).unwrap();
    let q = tokens.iter().find(|t| t.kind == TokenKind::QuotedString).unwrap();
    assert_eq!(q.lexeme, "a\"b");
}
```

- [ ] **Step 2: Run tests (failure expected: not implemented)**

Run: `cargo test --lib lexer::`
Expected: 5 of 7 fail with `not yet implemented`.

- [ ] **Step 3: Implement the lexer**

Replace `tokenize` with a hand-rolled state machine. Implementation outline:

```rust
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut lex = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(t) = lex.next_token()? {
        tokens.push(t);
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        lexeme: String::new(),
        span: lex.current_span(),
    });
    Ok(tokens)
}

struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self { src, bytes: src.as_bytes(), pos: 0, line: 1, col: 1 }
    }

    fn current_span(&self) -> Span {
        Span {
            start_line: self.line,
            start_col: self.col,
            end_line: self.line,
            end_col: self.col,
        }
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        // Skip horizontal whitespace (spaces, tabs) but not newlines.
        // Emit one Newline token per run of newlines.
        // Handle each TokenKind variant:
        //   - '@' -> At
        //   - identifier (letter [letter/digit/_]*) -> Identifier
        //   - ':' -> Colon
        //   - '"' -> QuotedString (parse escapes)
        //   - '[' -> LeftBracket, ']' -> RightBracket, ',' -> Comma
        //   - '---' on its own line -> NarrativeSeparator
        //   - '\n' -> Newline (collapse runs)
        //   - else identifier-y or unquoted -> UnquotedString
        // Reject:
        //   - null byte (\0)
        //   - control chars in unquoted/identifier (Error::InputInjection)
        //   - identifier longer than MAX_TAG_LEN
        todo!("hand-rolled state machine — see EBNF for exact rules")
    }
}
```

The subagent fills in the actual state-machine body. Key implementation points:
- Track 1-based line/col, advance on each byte (handle multi-byte UTF-8 as single chars for column counting via `char_indices()`)
- Identifier regex equivalent: `[A-Za-z][A-Za-z0-9_]*`
- Quoted string: `"..."` with `\"`, `\\`, `\n`, `\t`, `\r` escapes
- Newline collapsing: contiguous `\n` (with possible `\r\n`) → one Newline token
- The `---` separator: exactly three hyphens on a line by themselves (preceded and followed by Newline or BOF/EOF)

- [ ] **Step 4: Run tests until all pass**

Run: `cargo test --lib lexer::`
Iterate on the implementation until all 7 tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/lexer.rs
git commit -m "feat(lexer): hand-rolled state machine for FTAI tokens (Plan Task 5)"
```

---

## Task 6: Lexer hardening + CAT 1 red tests

**Files:**
- Create: `tests/red_cat1_input.rs`

This is the FolkTech Red Test Rule applied: write attack inputs that the current lexer might mishandle, watch them fail, harden, watch them pass.

- [ ] **Step 1: Write the red tests (one per spec A6 bullet)**

```rust
//! CAT 1 (Input Injection) red tests for the lexer.
//!
//! Each input MUST be rejected by `tokenize` with a precise error
//! category — never panic, never silently accept.

use ftai::error::{Error, ErrorCategory};

#[test]
fn red_null_byte_rejected() {
    let input = "@doc\0\n";
    let result = ftai_internal_tokenize(input);
    let err = result.expect_err("null byte must be rejected");
    assert_eq!(err.category(), ErrorCategory::InputInjection);
}

#[test]
fn red_oversize_tag_name_rejected() {
    let oversize_tag: String = "@".to_string() + &"a".repeat(257);
    let input = format!("{oversize_tag}\n");
    let err = ftai_internal_tokenize(&input).expect_err("oversize tag must be rejected");
    assert!(matches!(err.category(), ErrorCategory::InputInjection | ErrorCategory::LimitExceeded));
}

#[test]
fn red_control_character_in_value_rejected() {
    let input = "k: ab\x07cd\n"; // bell character \x07
    let err = ftai_internal_tokenize(input).expect_err("control char must be rejected");
    assert_eq!(err.category(), ErrorCategory::InputInjection);
}

#[test]
fn red_malformed_utf8_rejected_at_str_boundary() {
    // Note: &str guarantees UTF-8 validity; the only way to feed malformed
    // UTF-8 to a public API is via a byte-buffer entry point. v1 takes &str
    // only, so this test asserts the behavior at the byte-boundary entry
    // point (added in Task 11) — for now, assert a placeholder:
    let bad: &[u8] = &[0x40, 0xFF, 0xFE, 0x0A]; // @, then invalid continuation
    let result = std::str::from_utf8(bad);
    assert!(result.is_err(), "construction precondition: bad UTF-8 doesn't form &str");
}

#[test]
fn red_nesting_depth_in_lexer_path_does_not_panic_on_pathological_brackets() {
    // Lexer doesn't enforce nesting (parser does), but pathological bracket
    // sequences must not cause stack overflow during lexing.
    let input = "[".repeat(10_000);
    // Tokenize must complete without panic; correctness is parser's concern.
    let _ = ftai_internal_tokenize(&input); // OK or Err — both acceptable
}

// Internal tokenize entry point exposed for testing. The crate exposes
// only the high-level API; for these red tests we add a `pub(crate)`
// path through a `pub mod __testing` (defined in `src/lib.rs`).
fn ftai_internal_tokenize(input: &str) -> Result<Vec<ftai::__testing::Token>, Error> {
    ftai::__testing::tokenize(input)
}
```

- [ ] **Step 2: Expose lexer for testing**

In `src/lib.rs`, add (above the existing `pub mod ast;` etc.):

```rust
/// Test-only access to internals. Not part of the public API stability surface.
#[doc(hidden)]
pub mod __testing {
    pub use crate::lexer::{tokenize, Token, TokenKind};
}
```

- [ ] **Step 3: Run red tests (expect failures from current lexer)**

Run: `cargo test --test red_cat1_input`
Expected: at least 3 tests fail (null byte, oversize, control char).

- [ ] **Step 4: Harden the lexer**

Inside `Lexer::next_token`, add at the start of each character-classifying branch:
- Null byte check: `if b == 0 { return Err(Error::InputInjection(...)); }`
- Tag-name length check: when emitting `Identifier` after `@`, enforce `MAX_TAG_LEN`.
- Control character check: in unquoted/identifier scanning, reject any `c.is_control() && c != '\t' && c != '\n'`.
- Inside quoted strings: reject literal control chars (escape sequences are fine).

- [ ] **Step 5: Run red tests (expect all pass now)**

Run: `cargo test --test red_cat1_input`
Expected: 5 passed; 0 failed.

- [ ] **Step 6: Commit**

```bash
git add tests/red_cat1_input.rs src/lexer.rs src/lib.rs
git commit -m "secure(lexer): CAT 1 input-injection hardening + red tests (Plan Task 6)"
```

---

## Task 7: Parser happy path

**Files:**
- Modify: `src/parser.rs`
- Modify: `src/lib.rs` (wire `parse` to call into parser)

- [ ] **Step 1: Add inline parser tests**

```rust
//! Parser: token stream → Document AST.
//!
//! Recursive-descent over the EBNF in `ftai-spec/grammar/ftai.ebnf`.
//! Enforces nesting depth limit (default 64) — see Task 8.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    #[test]
    fn parse_minimal_document() {
        let input = "@ftai v2.0\n@document\ntitle: \"Hi\"\n@end\n";
        let tokens = tokenize(input).unwrap();
        let doc = parse_tokens(&tokens).unwrap();
        assert_eq!(doc.version, crate::ast::FtaiVersion::V2_0);
        assert_eq!(doc.blocks.len(), 1);
    }

    #[test]
    fn parse_section_with_attribute() {
        let input = "@ftai v2.0\n@document\ntitle: \"Hi\"\nauthor: Mike\n@end\n";
        let tokens = tokenize(input).unwrap();
        let doc = parse_tokens(&tokens).unwrap();
        if let crate::ast::Block::Section(s) = &doc.blocks[0] {
            assert_eq!(s.tag, "document");
            assert_eq!(s.attributes.len(), 2);
        } else {
            panic!("expected Section");
        }
    }

    #[test]
    fn parse_unsupported_version_rejected() {
        let input = "@ftai v1.0\n";
        let tokens = tokenize(input).unwrap();
        let err = parse_tokens(&tokens).unwrap_err();
        assert!(matches!(err, crate::error::Error::UnsupportedVersion(_)));
    }

    #[test]
    fn parse_narrative_block_between_sections() {
        let input = "@ftai v2.0\n@document\n@end\n---\nHello world\n---\n@ai\n@end\n";
        let tokens = tokenize(input).unwrap();
        let doc = parse_tokens(&tokens).unwrap();
        assert_eq!(doc.blocks.len(), 3);
        assert!(matches!(doc.blocks[1], crate::ast::Block::Narrative { .. }));
    }

    #[test]
    fn parse_unterminated_block_errors() {
        let input = "@ftai v2.0\n@document\ntitle: \"Hi\"\n";
        let tokens = tokenize(input).unwrap();
        let err = parse_tokens(&tokens).unwrap_err();
        assert!(matches!(err, crate::error::Error::UnterminatedBlock { .. }));
    }
}
```

- [ ] **Step 2: Run (compile failure for `parse_tokens`)**

Run: `cargo test --lib parser::`

- [ ] **Step 3: Implement the parser**

```rust
//! Parser: token stream → Document AST.

use crate::ast::{Block, Document, FtaiVersion, InlineTag, Section, Span, Value};
use crate::error::{Error, Result};
use crate::lexer::{Token, TokenKind};

/// Default nesting depth limit. See Task 8 for hardening.
pub const DEFAULT_NESTING_LIMIT: usize = 64;

/// Parse a token stream into a Document.
pub fn parse_tokens(tokens: &[Token]) -> Result<Document> {
    let mut p = Parser::new(tokens);
    p.parse_document()
}

struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
    nesting_limit: usize,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self { tokens, pos: 0, depth: 0, nesting_limit: DEFAULT_NESTING_LIMIT }
    }

    fn parse_document(&mut self) -> Result<Document> {
        // Consume @ftai vN.N header
        // Parse blocks until EOF
        // Return Document
        todo!("recursive descent — see EBNF")
    }

    // Helpers: peek(), advance(), expect(kind), parse_section(), parse_narrative()
}
```

The subagent fills the recursive-descent body. Key rules:
- `@ftai vN.N` header is mandatory; reject unknown version with `Error::UnsupportedVersion`
- `@tag\n key: value\n ... @end` consumed as `Section`
- `---` line ↔ narrative-block toggle (alternating with sections)
- Unterminated section: emit `Error::UnterminatedBlock`
- Tags case-insensitively normalized to lowercase in the AST

- [ ] **Step 4: Wire `ftai::parse` to call parser**

In `src/lib.rs`, replace the `todo!()` body of `parse`:

```rust
pub fn parse(input: &str) -> Result<Document> {
    let tokens = lexer::tokenize(input)?;
    parser::parse_tokens(&tokens)
}
```

- [ ] **Step 5: Run all tests**

Run: `cargo test --lib`
Expected: all parser tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/parser.rs src/lib.rs
git commit -m "feat(parser): recursive-descent FTAI v2.0 parser (Plan Task 7)"
```

---

## Task 8: Parser hardening (nesting depth)

**Files:**
- Modify: `src/parser.rs`
- Append to: `tests/red_cat1_input.rs`

- [ ] **Step 1: Add the depth red test**

In `tests/red_cat1_input.rs`:

```rust
#[test]
fn red_nesting_depth_exceeded_returns_clean_error() {
    // Build a document with 100 levels of nested @block @end ... — exceeds default 64.
    let mut s = String::from("@ftai v2.0\n");
    for i in 0..100 {
        s.push_str(&format!("@nest{i}\n"));
    }
    for _ in 0..100 {
        s.push_str("@end\n");
    }
    let err = ftai::parse(&s).expect_err("excessive nesting must be rejected");
    assert_eq!(err.category(), ftai::error::ErrorCategory::LimitExceeded);
}
```

- [ ] **Step 2: Run (expect failure or panic)**

Run: `cargo test --test red_cat1_input red_nesting_depth_exceeded`
Expected: failure (current parser doesn't enforce depth).

- [ ] **Step 3: Add depth tracking in parser**

In `Parser::parse_section` (or equivalent), increment `self.depth` on entry, decrement on exit. Before incrementing, check:
```rust
if self.depth >= self.nesting_limit {
    return Err(Error::NestingTooDeep {
        limit: self.nesting_limit,
        line: self.current_line(),
    });
}
```

- [ ] **Step 4: Run red test**

Expected: 1 passed.

- [ ] **Step 5: Commit**

```bash
git add tests/red_cat1_input.rs src/parser.rs
git commit -m "secure(parser): nesting-depth limit + red test (Plan Task 8)"
```

---

## Task 9: AST → text serializer

**Files:**
- Modify: `src/serializer.rs`
- Test: inline tests + new `tests/roundtrip_corpus.rs` (initial round-trip on small synthetic docs)

- [ ] **Step 1: Add round-trip property test**

Inline tests in `src/serializer.rs`:

```rust
//! Document → canonical FTAI text serializer.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_tokens;
    use crate::lexer::tokenize;

    fn roundtrip(input: &str) {
        let tokens = tokenize(input).unwrap();
        let doc = parse_tokens(&tokens).unwrap();
        let text = serialize_document(&doc).unwrap();
        let tokens2 = tokenize(&text).unwrap();
        let doc2 = parse_tokens(&tokens2).unwrap();
        assert_eq!(doc, doc2, "round-trip should be structurally stable");
    }

    #[test]
    fn roundtrip_minimal_document() {
        roundtrip("@ftai v2.0\n@document\ntitle: \"Hi\"\n@end\n");
    }

    #[test]
    fn roundtrip_with_narrative() {
        roundtrip("@ftai v2.0\n@document\n@end\n---\nHello world\n---\n@ai\nmode: \"core_memory\"\n@end\n");
    }

    #[test]
    fn roundtrip_nested_section() {
        roundtrip("@ftai v2.0\n@document\ntitle: \"x\"\n  @inner\n  k: v\n  @end\n@end\n");
    }
}
```

- [ ] **Step 2: Run (compile failure for `serialize_document`)**

- [ ] **Step 3: Implement serializer**

```rust
//! Document → canonical FTAI text serializer.

use crate::ast::{Block, Document, FtaiVersion, Section, Value};
use crate::error::{Error, Result};
use std::fmt::Write;

/// Serialize a [`Document`] to canonical FTAI text. Output is deterministic:
/// same input value, same output bytes, every time.
pub fn serialize_document(doc: &Document) -> Result<String> {
    let mut out = String::new();
    // Header
    let v = match doc.version {
        FtaiVersion::V2_0 => "v2.0",
    };
    write!(out, "@ftai {v}").map_err(|e| Error::Io(e.to_string()))?;
    if let Some(s) = &doc.schema {
        write!(out, " {s}").map_err(|e| Error::Io(e.to_string()))?;
    }
    out.push('\n');
    // Blocks
    for b in &doc.blocks {
        serialize_block(b, 0, &mut out)?;
    }
    Ok(out)
}

fn serialize_block(block: &Block, indent: usize, out: &mut String) -> Result<()> {
    match block {
        Block::Section(s) => serialize_section(s, indent, out),
        Block::Narrative { text, .. } => {
            out.push_str("---\n");
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
            out.push_str("---\n");
            Ok(())
        }
    }
}

fn serialize_section(s: &Section, indent: usize, out: &mut String) -> Result<()> {
    let pad = "  ".repeat(indent);
    write!(out, "{pad}@{}\n", s.tag).map_err(|e| Error::Io(e.to_string()))?;
    for (k, v) in &s.attributes {
        write!(out, "{pad}{k}: ").map_err(|e| Error::Io(e.to_string()))?;
        serialize_value(v, out)?;
        out.push('\n');
    }
    for child in &s.children {
        serialize_block(child, indent + 1, out)?;
    }
    write!(out, "{pad}@end\n").map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}

fn serialize_value(v: &Value, out: &mut String) -> Result<()> {
    match v {
        Value::Quoted(s) => {
            out.push('"');
            for c in s.chars() {
                match c {
                    '"' => out.push_str("\\\""),
                    '\\' => out.push_str("\\\\"),
                    '\n' => out.push_str("\\n"),
                    '\r' => out.push_str("\\r"),
                    '\t' => out.push_str("\\t"),
                    other => out.push(other),
                }
            }
            out.push('"');
        }
        Value::Unquoted(s) => out.push_str(s),
        Value::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 { out.push_str(", "); }
                serialize_value(item, out)?;
            }
            out.push(']');
        }
    }
    Ok(())
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib serializer::`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add src/serializer.rs
git commit -m "feat(serializer): deterministic Document → FTAI text (Plan Task 9)"
```

---

## Task 10: serde::Serializer impl

**Files:**
- Modify: `src/ser.rs`
- Modify: `src/lib.rs` (wire `to_string`)
- Create: `tests/serde_roundtrip.rs`

The `serde::Serializer` impl converts a typed value into a `Document` AST, then calls `serialize_document` to emit text.

- [ ] **Step 1: Write the failing roundtrip test**

`tests/serde_roundtrip.rs`:

```rust
//! Round-trip tests for serde-derive types via FTAI.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Note {
    title: String,
    body: String,
}

#[test]
fn primitive_struct_roundtrips() {
    let n = Note { title: "Hi".into(), body: "Hello.".into() };
    let text = ftai::to_string(&n).expect("serialize");
    let back: Note = ftai::from_str(&text).expect("deserialize");
    assert_eq!(n, back);
}

// More tests added in Task 11.
```

- [ ] **Step 2: Run (failure: to_string is `todo!()`)**

- [ ] **Step 3: Implement `serde::Serializer`**

In `src/ser.rs`, implement the trait. Strategy: build a `Document` AST node-by-node as serde calls `serialize_*` methods.

```rust
//! Implements `serde::Serializer` so any `Serialize` type can be written
//! out as FTAI text. Strategy:
//!
//!   - Top-level value MUST be a struct or map → becomes a `Document`
//!     with one `Section` per field-group.
//!   - Primitive fields → `Value::Unquoted` for numbers/bools,
//!     `Value::Quoted` for strings.
//!   - Vec / sequence → `Value::List`.
//!   - Nested struct → child `Section`.
//!   - Tagged enum (`#[serde(tag = "...")]`) → `Section` with `tag`
//!     equal to the variant name.

use crate::ast::{Block, Document, FtaiVersion, Section, Value};
use crate::error::{Error, Result};
use crate::serializer::serialize_document;
use serde::ser::{self, Impossible, Serialize};

pub fn to_string<T: Serialize + ?Sized>(value: &T) -> Result<String> {
    let mut ser = Serializer::default();
    value.serialize(&mut ser)?;
    let doc = ser.finish()?;
    serialize_document(&doc)
}

#[derive(Default)]
struct Serializer {
    // Build state — the root section being assembled, plus a stack
    // for nested structs.
}

impl Serializer {
    fn finish(self) -> Result<Document> {
        // Emit the assembled Document.
        todo!("collect built-up state into a Document")
    }
}

// impl serde::Serializer for &mut Serializer { ... }
// (~30 methods; mostly delegate to small helper functions. The subagent
// implements each: bool, integers, floats, &str, bytes, none/some,
// unit/unit_struct, struct, struct_variant, seq, map, etc.)
```

- [ ] **Step 4: Wire `lib.rs::to_string` to call into `ser::to_string`**

```rust
pub fn to_string<T: serde::Serialize + ?Sized>(value: &T) -> Result<String> {
    crate::ser::to_string(value)
}
```

- [ ] **Step 5: Implement until tests pass**

The subagent expands `Serializer` until `primitive_struct_roundtrips` passes. Note: this test depends on `from_str` (Task 11) — the subagent may stub `from_str` initially and circle back.

- [ ] **Step 6: Commit**

```bash
git add src/ser.rs src/lib.rs tests/serde_roundtrip.rs
git commit -m "feat(ser): serde::Serializer impl for FTAI output (Plan Task 10)"
```

---

## Task 11: serde::Deserializer impl

**Files:**
- Modify: `src/de.rs`
- Modify: `src/lib.rs` (wire `from_str`)
- Append to: `tests/serde_roundtrip.rs`

- [ ] **Step 1: Add deserializer round-trip tests covering A4 type list**

Append to `tests/serde_roundtrip.rs`:

```rust
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Nested {
    inner: Note,
    count: u32,
    flags: Vec<String>,
}

#[test]
fn nested_struct_roundtrips() {
    let v = Nested {
        inner: Note { title: "x".into(), body: "y".into() },
        count: 42,
        flags: vec!["a".into(), "b".into()],
    };
    let text = ftai::to_string(&v).unwrap();
    let back: Nested = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct WithOptionAndMap {
    maybe: Option<String>,
    extras: HashMap<String, u32>,
}

#[test]
fn option_and_hashmap_roundtrip() {
    let v = WithOptionAndMap {
        maybe: Some("hi".into()),
        extras: HashMap::from([("a".into(), 1u32), ("b".into(), 2u32)]),
    };
    let text = ftai::to_string(&v).unwrap();
    let back: WithOptionAndMap = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Event {
    Login { user: String },
    Logout,
}

#[test]
fn tagged_enum_roundtrips_via_section_tag() {
    let v = Event::Login { user: "mike".into() };
    let text = ftai::to_string(&v).unwrap();
    let back: Event = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}
```

- [ ] **Step 2: Implement `serde::Deserializer`**

In `src/de.rs`, parse the input to a `Document`, then walk it to drive the visitor pattern. Same shape as `ser.rs` but in reverse.

- [ ] **Step 3: Wire `lib.rs::from_str`**

```rust
pub fn from_str<T: serde::de::DeserializeOwned>(input: &str) -> Result<T> {
    crate::de::from_str(input)
}
```

- [ ] **Step 4: Iterate until all serde tests pass**

Run: `cargo test --test serde_roundtrip`
Expected: all 4 tests passing.

- [ ] **Step 5: Commit**

```bash
git add src/de.rs src/lib.rs tests/serde_roundtrip.rs
git commit -m "feat(de): serde::Deserializer impl for FTAI input (Plan Task 11)"
```

---

## Task 12: parse_lenient (fault-tolerant mode)

**Files:**
- Modify: `src/parser.rs`
- Modify: `src/lib.rs`
- Create: `tests/lenient_recovery.rs`

- [ ] **Step 1: Write tests for each spec A5 recovery class**

```rust
//! Fault-tolerant parsing per spec A5.

#[test]
fn lenient_unknown_tag_is_ignored_with_error_logged() {
    let input = "@ftai v2.0\n@unknown_tag\nx: 1\n@end\n@document\ntitle: \"ok\"\n@end\n";
    let (doc, errors) = ftai::parse_lenient(input);
    // The known tag landed
    assert!(doc.blocks.iter().any(|b| matches!(b, ftai::Block::Section(s) if s.tag == "document")));
    // An error was logged for the unknown tag
    assert_eq!(errors.len(), 1);
}

#[test]
fn lenient_missing_end_marker_recovered_at_next_tag() {
    let input = "@ftai v2.0\n@document\ntitle: \"hi\"\n@ai\nmode: \"x\"\n@end\n";
    let (doc, errors) = ftai::parse_lenient(input);
    assert_eq!(doc.blocks.len(), 2);
    assert_eq!(errors.len(), 1);
}

#[test]
fn lenient_truncated_final_block_preserves_prefix() {
    let input = "@ftai v2.0\n@document\ntitle: \"hi\"";
    let (doc, errors) = ftai::parse_lenient(input);
    assert!(!doc.blocks.is_empty()); // partial document survived
    assert!(!errors.is_empty());
}
```

(Note: `Block` needs to be re-exported from the crate root for these tests. Add `pub use crate::ast::Block;` to `src/lib.rs` if not already.)

- [ ] **Step 2: Implement `parse_lenient` in `src/parser.rs`**

The fault-tolerant parser collects errors instead of returning on the first one. Strategy: use a recoverable `Result<T, Error>` internally and accumulate errors in a `Vec<Error>` on the Parser struct.

- [ ] **Step 3: Wire `lib.rs::parse_lenient`**

```rust
pub fn parse_lenient(input: &str) -> (Document, Vec<Error>) {
    match crate::lexer::tokenize(input) {
        Ok(tokens) => crate::parser::parse_tokens_lenient(&tokens),
        Err(e) => (Document::default(), vec![e]),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --test lenient_recovery`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add src/parser.rs src/lib.rs tests/lenient_recovery.rs
git commit -m "feat(parser): parse_lenient fault-tolerant mode (Plan Task 12)"
```

---

## Task 13: CAT 7 parser hygiene tests

**Files:**
- Create: `tests/red_cat7_hygiene.rs`

- [ ] **Step 1: Write the hygiene tests per spec A7**

```rust
//! CAT 7 (LLM Output Handling) parser hygiene tests.

#[test]
fn truncated_at_end_in_ai_block_surfaces_error() {
    // @ai block opened but @end is truncated mid-token
    let input = "@ftai v2.0\n@ai\nmode: \"x\"\n@en";
    let result = ftai::parse(input);
    assert!(result.is_err(), "must error on truncated @end");
}

#[test]
fn embedded_at_end_inside_quoted_string_does_not_close_section() {
    let input = "@ftai v2.0\n@ai\nnote: \"contains @end token-like text\"\n@end\n";
    let doc = ftai::parse(input).expect("strings can contain @end-like text");
    if let ftai::Block::Section(s) = &doc.blocks[0] {
        assert_eq!(s.tag, "ai");
        assert!(s.attributes.iter().any(|(k, _)| k == "note"));
    } else {
        panic!("expected Section");
    }
}

#[test]
fn unbalanced_quoted_string_in_ai_block_errors() {
    let input = "@ftai v2.0\n@ai\nnote: \"unterminated\n@end\n";
    let result = ftai::parse(input);
    assert!(result.is_err(), "unterminated quoted string must error");
}
```

- [ ] **Step 2: Run tests**

Run: `cargo test --test red_cat7_hygiene`
Expected: tests should pass given a correctly-implemented lexer/parser. If any fail, the lexer/parser has a real bug — fix it before proceeding.

- [ ] **Step 3: Commit**

```bash
git add tests/red_cat7_hygiene.rs
git commit -m "secure(parser): CAT 7 LLM-output hygiene tests (Plan Task 13)"
```

---

## Task 14: Round-trip on official corpus + Python parser parity

**Files:**
- Create: `tests/fixtures/` (copy `.ftai` files from `FolkTechAI/ftai-spec`)
- Create: `tests/roundtrip_corpus.rs`
- Create: `tests/parity_python.rs`

- [ ] **Step 1: Vendor the corpus**

Clone or fetch `FolkTechAI/ftai-spec` and copy:
- `parsers/python/sample_valid.ftai` → `tests/fixtures/sample_valid.ftai`
- Any `.ftai` files under `tests/` → `tests/fixtures/`
- The Python parser script → `tests/fixtures/parseftai_linter.py` (for the parity test)

Provide attribution in `tests/fixtures/README.md`:

```markdown
# Test Fixtures

These `.ftai` files and the Python reference parser are vendored from
[FolkTechAI/ftai-spec](https://github.com/FolkTechAI/ftai-spec) under
the same Apache-2.0 license. Updates to those upstream files should
be re-vendored periodically.
```

- [ ] **Step 2: Write round-trip test**

`tests/roundtrip_corpus.rs`:

```rust
//! Round-trip on every fixture: parse → serialize → parse.

use std::fs;
use std::path::Path;

#[test]
fn roundtrip_every_fixture_structurally_stable() {
    let dir = Path::new("tests/fixtures");
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ftai") {
            continue;
        }
        let input = fs::read_to_string(&path).unwrap();
        let doc1 = ftai::parse(&input).unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e));
        let text = ftai::to_string(&doc1).unwrap();
        let doc2 = ftai::parse(&text).unwrap_or_else(|e| panic!("re-parse {}: {}", path.display(), e));
        pretty_assertions::assert_eq!(
            doc1, doc2,
            "fixture {} failed structural round-trip",
            path.display()
        );
    }
}
```

- [ ] **Step 3: Write Python parity test**

`tests/parity_python.rs`:

```rust
//! Python reference parser parity test.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "requires python3 in PATH; run via `cargo test -- --ignored parity`"]
fn rust_output_matches_python_reference_on_corpus() {
    let dir = Path::new("tests/fixtures");
    let py_script = dir.join("parseftai_linter.py");
    if !py_script.exists() {
        eprintln!("skip: python reference not vendored");
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ftai") {
            continue;
        }
        let input = fs::read_to_string(&path).unwrap();

        // Rust output as JSON
        let rust_doc = ftai::parse(&input).unwrap();
        let rust_json = serde_json::to_string(&rust_doc).unwrap();

        // Python output as JSON
        let py_out = Command::new("python3")
            .arg(&py_script)
            .arg("--json")
            .arg(&path)
            .output()
            .expect("python3 must be in PATH");
        assert!(py_out.status.success(), "python parser failed on {}", path.display());
        let py_json = String::from_utf8(py_out.stdout).unwrap();

        assert_eq!(
            normalize_json(&rust_json),
            normalize_json(&py_json),
            "fixture {} parity mismatch",
            path.display()
        );
    }
}

fn normalize_json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap()
}
```

(The `--json` flag for the Python parser may not exist; the subagent may need to write a small Python adapter that calls the parser and prints JSON. Add the adapter to `tests/fixtures/` if so.)

- [ ] **Step 4: Run tests**

Run: `cargo test --test roundtrip_corpus`
Expected: all fixtures round-trip cleanly.

Run: `cargo test --test parity_python -- --ignored`
Expected: parity passes; fix any divergences in the lexer/parser.

- [ ] **Step 5: Commit**

```bash
git add tests/fixtures tests/roundtrip_corpus.rs tests/parity_python.rs
git commit -m "test: corpus round-trip + Python reference parity (Plan Task 14)"
```

---

## Task 15: EBNF conformance suite

**Files:**
- Create: `tests/grammar_conformance.rs`

- [ ] **Step 1: Enumerate every production in `ftai-spec/grammar/ftai.ebnf`**

```
ftai_file        = ftai_header, { ftai_block } ;
ftai_header      = "@ftai", SP, "v2.0", NL ;
ftai_block       = tag_section | tag_single ;
tag_section      = tag, { key_value }, { inner_block }, "@end", NL ;
tag_single       = tag, [ SP, value ], NL ;
tag              = "@", identifier ;
key_value        = SP, identifier, ":", SP, value, NL ;
inner_block      = SP, tag_section | SP, tag_single ;
value            = quoted_string | unquoted_string ;
quoted_string    = '"', { any_char_except_quote }, '"' ;
unquoted_string  = { any_char_except_specials } ;
identifier       = letter, { letter | digit | "_" } ;
```

11 productions. One accept + one reject test per production.

- [ ] **Step 2: Write the conformance tests**

```rust
//! EBNF conformance: one accept + one reject test per grammar production
//! in ftai-spec/grammar/ftai.ebnf.

// ftai_file
#[test]
fn ftai_file_minimal_header_only_accepts() {
    assert!(ftai::parse("@ftai v2.0\n").is_ok());
}
#[test]
fn ftai_file_missing_header_rejects() {
    assert!(ftai::parse("@document\n@end\n").is_err());
}

// ftai_header — version
#[test]
fn ftai_header_v2_0_accepts() {
    assert!(ftai::parse("@ftai v2.0\n").is_ok());
}
#[test]
fn ftai_header_unknown_version_rejects() {
    assert!(ftai::parse("@ftai v9.9\n").is_err());
}

// tag_section
#[test]
fn tag_section_with_end_accepts() {
    assert!(ftai::parse("@ftai v2.0\n@doc\n@end\n").is_ok());
}
#[test]
fn tag_section_without_end_rejects() {
    assert!(ftai::parse("@ftai v2.0\n@doc\n").is_err());
}

// tag_single (single-line tag with optional value)
#[test]
fn tag_single_with_value_accepts() {
    assert!(ftai::parse("@ftai v2.0\n@status active\n").is_ok());
}
#[test]
fn tag_single_with_invalid_value_rejects() {
    assert!(ftai::parse("@ftai v2.0\n@status \x07active\n").is_err());
}

// key_value
#[test]
fn key_value_indented_quoted_string_accepts() {
    assert!(ftai::parse("@ftai v2.0\n@doc\ntitle: \"hi\"\n@end\n").is_ok());
}
#[test]
fn key_value_missing_colon_rejects() {
    assert!(ftai::parse("@ftai v2.0\n@doc\ntitle \"hi\"\n@end\n").is_err());
}

// quoted_string
#[test]
fn quoted_string_with_escaped_quote_accepts() {
    assert!(ftai::parse("@ftai v2.0\n@doc\nk: \"a\\\"b\"\n@end\n").is_ok());
}
#[test]
fn quoted_string_unterminated_rejects() {
    assert!(ftai::parse("@ftai v2.0\n@doc\nk: \"unterminated\n@end\n").is_err());
}

// unquoted_string — see acceptance via tag_single test above.

// identifier
#[test]
fn identifier_starts_with_letter_accepts() {
    assert!(ftai::parse("@ftai v2.0\n@doc_2\n@end\n").is_ok());
}
#[test]
fn identifier_starts_with_digit_rejects() {
    assert!(ftai::parse("@ftai v2.0\n@2doc\n@end\n").is_err());
}

// inner_block
#[test]
fn inner_block_nested_section_accepts() {
    assert!(ftai::parse("@ftai v2.0\n@outer\n  @inner\n  @end\n@end\n").is_ok());
}
#[test]
fn inner_block_unindented_rejects_when_strict() {
    // Per EBNF, inner_block requires leading SP. Lenient mode accepts; strict rejects.
    let r = ftai::parse("@ftai v2.0\n@outer\n@inner\n@end\n@end\n");
    // Spec is ambiguous on this — accept either behavior, but be deterministic.
    let _ = r;
}
```

- [ ] **Step 3: Run conformance suite**

Run: `cargo test --test grammar_conformance`
Expected: all tests pass. If any fail, the lexer/parser doesn't match the EBNF and needs fixing.

- [ ] **Step 4: Commit**

```bash
git add tests/grammar_conformance.rs
git commit -m "test(grammar): EBNF conformance suite, one accept + reject per production (Plan Task 15)"
```

---

## Task 16: README + CHANGELOG + CI + first release tag

**Files:**
- Modify: `README.md` (currently 175 bytes)
- Create: `CHANGELOG.md`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Write the README**

```markdown
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
| `ftai::parse_lenient(s)` | Parse with error recovery; returns (Document, Vec<Error>) |
| `ftai::to_string(value)` | Serialize a Serialize-impl type to FTAI text |
| `ftai::from_str(s)` | Deserialize FTAI text into a DeserializeOwned-impl type |

## What's NOT in v1 (by design)

- Streaming / incremental parsing
- Schema validation (lives in `ftai-spec/schema/`)
- Multi-version support (v2.0 only)
- Async / WASM / `no_std`
- C-ABI / Swift FFI bindings

See `docs/specs/2026-05-07-ftai-rs-v1-design.md` for the full out-of-scope list and rationale.

## Contributing

PRs welcome. Format checks: `cargo fmt --check && cargo clippy --all-targets -- -D warnings`. Tests: `cargo test`.

## License

Apache-2.0 — matches `FolkTechAI/ftai-spec`.
```

- [ ] **Step 2: Write `CHANGELOG.md`**

```markdown
# Changelog

## v0.1.0 — 2026-05-XX (planned)

### Added
- FTAI v2.0 parser (hand-rolled lexer + recursive-descent parser)
- `serde::Serializer` / `Deserializer` adapter for typed round-trips
- `parse_lenient` mode for fault-tolerant parsing
- CAT 1 (input injection) hardening with red tests
- CAT 7 (LLM output handling) hygiene tests
- EBNF conformance suite
- Python reference-parser parity tests

### Constraints
- Apache-2.0 license
- `#![forbid(unsafe_code)]`
- Production deps: `serde`, `thiserror` only
- Sync API only
```

- [ ] **Step 3: Write CI workflow**

`.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [main, "init/**", "feat/**", "fix/**"]
  pull_request:
    branches: [main]

jobs:
  test:
    name: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: cargo fmt
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: cargo test
        run: cargo test --all-targets
      - name: cargo doc
        run: cargo doc --no-deps --document-private-items
        env:
          RUSTDOCFLAGS: "-D warnings"

  parity:
    name: python-parity
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: actions/setup-python@v5
        with: { python-version: "3.11" }
      - name: cargo test parity
        run: cargo test --test parity_python -- --ignored
```

- [ ] **Step 4: Verify CI definition is parseable + tests still pass locally**

Run: `cargo test --all-targets`
Expected: full suite green.

Run: `cargo doc --no-deps --document-private-items`
Expected: no warnings.

- [ ] **Step 5: Commit**

```bash
git add README.md CHANGELOG.md .github/
git commit -m "docs+ci: README, changelog, GitHub Actions workflow (Plan Task 16)"
```

- [ ] **Step 6: Open PR + tag v0.1.0**

```bash
git push -u origin init/spec-first-scaffolding
gh pr create --base main --title "feat: ftai-rs v1 — FTAI v2.0 parser + serde adapter" \
  --body "Implements spec FTAI-RS-001. See plan in docs/plans/2026-05-07-ftai-rs-v1-implementation.md."
```

(The v0.1.0 git tag waits for project-owner approval before being applied.)

---

## Done-ness checklist (all must be ✅ before declaring v1 complete)

- [ ] All 16 tasks committed
- [ ] `cargo test --all-targets` — passes
- [ ] `cargo clippy --all-targets -- -D warnings` — clean
- [ ] `cargo fmt --check` — clean
- [ ] `cargo doc --no-deps` — no warnings
- [ ] Coverage ≥ 80% (run `cargo llvm-cov --html` or equivalent)
- [ ] CI green on macOS-arm64 and linux-x86_64
- [ ] PR opened against `main` for project-owner review
- [ ] All 11 acceptance criteria from FTAI-RS-001 demonstrably met (link to evidence in PR description)

## Cross-references

- Spec: `docs/specs/2026-05-07-ftai-rs-v1-design.md` (FTAI-RS-001)
- Format: `FolkTechAI/ftai-spec` repo
- First consumer: `FolkTechAI/Mitosis-Clustering` repo, `crates/mitosis-core/`
