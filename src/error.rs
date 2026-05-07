//! Error type for ftai-rs. All public fallible APIs return `Result<T>` aliasing
//! `std::result::Result<T, Error>`.
//!
//! Errors carry line/column information when applicable so consumers can
//! produce useful diagnostics. The [`ErrorCategory`] taxonomy matches the
//! security categories from `FolkTech` CLAUDE.md (CAT 1 input injection in
//! particular) so consumers can branch on category for security-relevant
//! handling.

use thiserror::Error;

/// Result alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;

/// Top-level error type for parsing and serializing FTAI documents.
#[derive(Debug, Error)]
#[non_exhaustive]
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
    /// Maps to `FolkTech` CAT 1.
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
/// Mirrors the `FolkTech` 9-category security taxonomy (CLAUDE.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
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
    /// Coarse category for branching by consumers.
    #[must_use]
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
