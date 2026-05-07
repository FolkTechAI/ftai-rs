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

/// Test-only access to internals. Not part of the public API stability surface.
#[doc(hidden)]
pub mod __testing {
    pub use crate::lexer::{Token, TokenKind, tokenize};
}

pub use crate::ast::{Block, Document};
pub use crate::error::{Error, Result};

/// Parse a `.ftai` source string into a structural [`Document`] AST.
///
/// Returns `Err` on the first structural failure. For fault-tolerant
/// parsing, use [`parse_lenient`].
///
/// # Errors
/// Returns `Err` on the first lex or parse failure.
pub fn parse(input: &str) -> Result<Document> {
    let tokens = lexer::tokenize(input)?;
    parser::parse_tokens(&tokens)
}

/// Parse leniently: recover from unknown tags, missing `@end` markers,
/// and stray whitespace per the FTAI spec's "fault-tolerant" principle.
/// Returns the best-effort AST and the list of errors encountered.
#[must_use]
pub fn parse_lenient(input: &str) -> (Document, Vec<Error>) {
    match crate::lexer::tokenize(input) {
        Ok(tokens) => crate::parser::parse_tokens_lenient(&tokens),
        Err(e) => (Document::default(), vec![e]),
    }
}

/// Serialize a value implementing [`serde::Serialize`] to FTAI text.
///
/// # Errors
/// Returns `Err` if the value cannot be represented as FTAI.
pub fn to_string<T: serde::Serialize + ?Sized>(value: &T) -> Result<String> {
    crate::ser::to_string(value)
}

/// Deserialize a `.ftai` source string into a value implementing
/// [`serde::de::DeserializeOwned`].
///
/// # Errors
/// Returns `Err` if the input cannot be parsed or shaped into `T`.
pub fn from_str<T: serde::de::DeserializeOwned>(input: &str) -> Result<T> {
    crate::de::from_str(input)
}
