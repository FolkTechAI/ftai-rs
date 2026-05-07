//! Lexer for FTAI source text. Produces a flat stream of [`Token`]s
//! consumed by the parser in `parser.rs`.
//!
//! Hand-rolled state machine — the EBNF grammar is small enough that a
//! parser-combinator dependency would be net negative. The lexer is
//! responsible for input-injection defense: null bytes, oversize tag
//! names, control characters, and malformed UTF-8 are rejected here
//! with `Error::InputInjection`.

#![allow(dead_code)] // Removed when Task 5 wires `tokenize` in.

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

/// Tokenize a complete source string.
///
/// # Errors
/// Returns `Err(Error::InputInjection)` on the first CAT 1 violation
/// (null byte, oversize tag, control character).
pub fn tokenize(_input: &str) -> Result<Vec<Token>> {
    let _ = (Error::Io("not yet implemented".into()), MAX_TAG_LEN);
    todo!("Task 5")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_at_construct_carries_span() {
        let t = Token {
            kind: TokenKind::At,
            lexeme: "@".into(),
            span: crate::ast::Span {
                start_line: 1,
                start_col: 1,
                end_line: 1,
                end_col: 2,
            },
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
