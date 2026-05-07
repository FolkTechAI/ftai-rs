//! Lexer for FTAI source text. Produces a flat stream of [`Token`]s
//! consumed by the parser in `parser.rs`.
//!
//! Hand-rolled state machine — the EBNF grammar is small enough that a
//! parser-combinator dependency would be net negative. The lexer is
//! responsible for input-injection defense: null bytes, oversize tag
//! names, control characters, and malformed UTF-8 are rejected here
//! with `Error::InputInjection`.

// Lexer items become live once the parser (Task 7) consumes them.
#![allow(dead_code)]

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
pub fn tokenize(input: &str) -> Result<Vec<Token>> {
    let mut lex = Lexer::new(input);
    let mut tokens = Vec::new();
    while let Some(t) = lex.next_token()? {
        tokens.push(t);
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        lexeme: String::new(),
        span: lex.point_span(),
    });
    Ok(tokens)
}

struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
    /// True when the current scan position is at the start of a logical line
    /// (after a newline or at BOF). Used to recognise `---` separators.
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            line: 1,
            col: 1,
            at_line_start: true,
        }
    }

    fn point_span(&self) -> Span {
        Span {
            start_line: self.line,
            start_col: self.col,
            end_line: self.line,
            end_col: self.col,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    /// Advance one byte. Updates line/col. Caller must have verified
    /// the byte at `pos` is not the start of a multi-byte UTF-8 sequence
    /// before calling, *or* be in a context where treating bytes as columns
    /// is acceptable.
    fn bump(&mut self) {
        if let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
        }
    }

    /// Advance one full UTF-8 character, returning the consumed slice.
    fn bump_char(&mut self) -> Option<&'a str> {
        let rest = &self.src[self.pos..];
        let mut iter = rest.char_indices();
        let (_, c) = iter.next()?;
        let next_idx = iter.next().map_or(rest.len(), |(i, _)| i);
        let consumed = &rest[..next_idx];
        self.pos += next_idx;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(consumed)
    }

    fn next_token(&mut self) -> Result<Option<Token>> {
        // Skip horizontal whitespace (spaces and tabs only).
        while let Some(b) = self.peek() {
            if b == b' ' || b == b'\t' {
                self.bump();
            } else {
                break;
            }
        }
        let Some(b) = self.peek() else {
            return Ok(None);
        };

        // Reject null bytes immediately (CAT 1).
        if b == 0 {
            return Err(Error::InputInjection(format!(
                "null byte at line {}, column {}",
                self.line, self.col
            )));
        }

        // Newline → emit Newline (collapses runs).
        if b == b'\n' || b == b'\r' {
            return Ok(Some(self.read_newline()));
        }

        // Narrative separator: `---` on a line by itself, only at line start.
        if b == b'-'
            && self.at_line_start
            && self.peek_at(1) == Some(b'-')
            && self.peek_at(2) == Some(b'-')
            && matches!(self.peek_at(3), Some(b'\n' | b'\r') | None)
        {
            let start_line = self.line;
            let start_col = self.col;
            self.bump();
            self.bump();
            self.bump();
            self.at_line_start = false;
            return Ok(Some(Token {
                kind: TokenKind::NarrativeSeparator,
                lexeme: "---".into(),
                span: Span {
                    start_line,
                    start_col,
                    end_line: self.line,
                    end_col: self.col,
                },
            }));
        }

        // Single-char punctuators.
        match b {
            b'@' => return Ok(Some(self.read_single_char(TokenKind::At, "@"))),
            b':' => return Ok(Some(self.read_single_char(TokenKind::Colon, ":"))),
            b'[' => {
                return Ok(Some(self.read_single_char(TokenKind::LeftBracket, "[")));
            }
            b']' => {
                return Ok(Some(self.read_single_char(TokenKind::RightBracket, "]")));
            }
            b',' => return Ok(Some(self.read_single_char(TokenKind::Comma, ","))),
            b'"' => return self.read_quoted().map(Some),
            _ => {}
        }

        // Identifier or unquoted string.
        if is_ident_start(b) {
            // Try identifier first; if the resulting word has only ident chars,
            // emit Identifier; else emit UnquotedString.
            self.read_word_or_ident().map(Some)
        } else if is_unquoted_start(b) {
            self.read_unquoted_word().map(Some)
        } else {
            Err(Error::InputInjection(format!(
                "unexpected byte 0x{b:02x} at line {}, column {}",
                self.line, self.col
            )))
        }
    }

    fn read_single_char(&mut self, kind: TokenKind, lexeme: &str) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        self.bump();
        self.at_line_start = false;
        Token {
            kind,
            lexeme: lexeme.to_string(),
            span: Span {
                start_line,
                start_col,
                end_line: self.line,
                end_col: self.col,
            },
        }
    }

    fn read_newline(&mut self) -> Token {
        let start_line = self.line;
        let start_col = self.col;
        let mut lexeme = String::new();
        // Collapse contiguous newline characters (handle CR, LF, CRLF runs).
        loop {
            match self.peek() {
                Some(b'\r') => {
                    lexeme.push('\r');
                    self.bump();
                    if self.peek() == Some(b'\n') {
                        lexeme.push('\n');
                        self.bump();
                    }
                }
                Some(b'\n') => {
                    lexeme.push('\n');
                    self.bump();
                }
                _ => break,
            }
        }
        self.at_line_start = true;
        Token {
            kind: TokenKind::Newline,
            lexeme,
            span: Span {
                start_line,
                start_col,
                end_line: self.line,
                end_col: self.col,
            },
        }
    }

    fn read_quoted(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_col = self.col;
        // Consume opening quote.
        self.bump();
        let mut buf = String::new();
        loop {
            let Some(b) = self.peek() else {
                return Err(Error::UnexpectedToken {
                    expected: "closing '\"'".into(),
                    found: "EOF".into(),
                    line: self.line,
                    column: self.col,
                });
            };
            if b == 0 {
                return Err(Error::InputInjection(format!(
                    "null byte inside quoted string at line {}, column {}",
                    self.line, self.col
                )));
            }
            if b == b'"' {
                self.bump();
                break;
            }
            if b == b'\\' {
                self.bump();
                let Some(esc) = self.peek() else {
                    return Err(Error::UnexpectedToken {
                        expected: "escape char".into(),
                        found: "EOF".into(),
                        line: self.line,
                        column: self.col,
                    });
                };
                match esc {
                    b'"' => buf.push('"'),
                    b'\\' => buf.push('\\'),
                    b'n' => buf.push('\n'),
                    b'r' => buf.push('\r'),
                    b't' => buf.push('\t'),
                    other => {
                        return Err(Error::InputInjection(format!(
                            "unknown escape '\\{}' at line {}, column {}",
                            other as char, self.line, self.col
                        )));
                    }
                }
                self.bump();
                continue;
            }
            // Disallow raw control characters inside quoted strings
            // (newlines/tabs require the explicit escape form).
            if is_control_byte(b) {
                return Err(Error::InputInjection(format!(
                    "control character 0x{:02x} inside quoted string at line {}, column {}",
                    b, self.line, self.col
                )));
            }
            // Append the next full UTF-8 char.
            if let Some(s) = self.bump_char() {
                buf.push_str(s);
            }
        }
        self.at_line_start = false;
        Ok(Token {
            kind: TokenKind::QuotedString,
            lexeme: buf,
            span: Span {
                start_line,
                start_col,
                end_line: self.line,
                end_col: self.col,
            },
        })
    }

    /// Read a word starting with a letter or `_`. If it consists solely of
    /// `[A-Za-z0-9_]` characters, emit `Identifier`; otherwise emit
    /// `UnquotedString`.
    fn read_word_or_ident(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_col = self.col;
        let start_pos = self.pos;
        let mut all_ident = true;
        while let Some(b) = self.peek() {
            if is_word_terminator(b) {
                break;
            }
            if b == 0 {
                return Err(Error::InputInjection(format!(
                    "null byte at line {}, column {}",
                    self.line, self.col
                )));
            }
            if is_control_byte(b) {
                return Err(Error::InputInjection(format!(
                    "control character 0x{:02x} at line {}, column {}",
                    b, self.line, self.col
                )));
            }
            if !is_ident_continue(b) {
                all_ident = false;
            }
            // Multi-byte UTF-8 chars are not ident-chars.
            if b >= 0x80 {
                all_ident = false;
            }
            if self.bump_char().is_none() {
                break;
            }
        }
        let lexeme = self.src[start_pos..self.pos].to_string();
        if all_ident && lexeme.len() > MAX_TAG_LEN {
            return Err(Error::InputInjection(format!(
                "identifier exceeds {MAX_TAG_LEN}-byte limit at line {start_line}",
            )));
        }
        let kind = if all_ident {
            TokenKind::Identifier
        } else {
            TokenKind::UnquotedString
        };
        self.at_line_start = false;
        Ok(Token {
            kind,
            lexeme,
            span: Span {
                start_line,
                start_col,
                end_line: self.line,
                end_col: self.col,
            },
        })
    }

    /// Read an unquoted word starting with a non-ident, non-special char.
    fn read_unquoted_word(&mut self) -> Result<Token> {
        let start_line = self.line;
        let start_col = self.col;
        let start_pos = self.pos;
        while let Some(b) = self.peek() {
            if is_word_terminator(b) {
                break;
            }
            if b == 0 {
                return Err(Error::InputInjection(format!(
                    "null byte at line {}, column {}",
                    self.line, self.col
                )));
            }
            if is_control_byte(b) {
                return Err(Error::InputInjection(format!(
                    "control character 0x{:02x} at line {}, column {}",
                    b, self.line, self.col
                )));
            }
            if self.bump_char().is_none() {
                break;
            }
        }
        let lexeme = self.src[start_pos..self.pos].to_string();
        self.at_line_start = false;
        Ok(Token {
            kind: TokenKind::UnquotedString,
            lexeme,
            span: Span {
                start_line,
                start_col,
                end_line: self.line,
                end_col: self.col,
            },
        })
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_'
}

fn is_ident_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Bytes that end the current word/atom in the token stream.
fn is_word_terminator(b: u8) -> bool {
    matches!(
        b,
        b' ' | b'\t' | b'\n' | b'\r' | b'@' | b':' | b'"' | b'[' | b']' | b','
    )
}

/// True for non-printable control bytes (excluding tab/newline/CR which
/// are handled separately as whitespace tokens).
fn is_control_byte(b: u8) -> bool {
    (b < 0x20 && b != b'\t' && b != b'\n' && b != b'\r') || b == 0x7f
}

fn is_unquoted_start(b: u8) -> bool {
    // Anything that survives `is_word_terminator` and isn't a control char
    // and isn't ident-start. e.g., digits, punctuation like `.`, `-`, `/`, etc.
    !is_word_terminator(b) && !is_control_byte(b) && !is_ident_start(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_at_construct_carries_span() {
        let t = Token {
            kind: TokenKind::At,
            lexeme: "@".into(),
            span: Span {
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

    #[test]
    fn lex_single_at_tag_followed_by_identifier() {
        use TokenKind::{At, Eof, Identifier, Newline};
        let input = "@document\n";
        let tokens = tokenize(input).unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(kinds, vec![&At, &Identifier, &Newline, &Eof]);
    }

    #[test]
    fn lex_key_value_pair_with_quoted_string() {
        use TokenKind::{Colon, Eof, Identifier, Newline, QuotedString};
        let input = "title: \"Hi there\"\n";
        let tokens = tokenize(input).unwrap();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        assert_eq!(
            kinds,
            vec![&Identifier, &Colon, &QuotedString, &Newline, &Eof]
        );
    }

    #[test]
    fn lex_narrative_separator_three_hyphens() {
        let input = "---\n";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0].kind, TokenKind::NarrativeSeparator);
    }

    #[test]
    fn lex_full_minimal_document() {
        let input = "@ftai v2.0\n@document\ntitle: \"Hi\"\n@end\n";
        let tokens = tokenize(input).unwrap();
        assert_eq!(tokens[0].kind, TokenKind::At);
        assert_eq!(tokens[1].kind, TokenKind::Identifier);
        assert_eq!(tokens[1].lexeme, "ftai");
        assert!(tokens.iter().any(|t| t.lexeme == "v2.0"));
    }

    #[test]
    fn lex_quoted_string_resolves_escape_sequences() {
        let input = "k: \"a\\\"b\"\n";
        let tokens = tokenize(input).unwrap();
        let q = tokens
            .iter()
            .find(|t| t.kind == TokenKind::QuotedString)
            .unwrap();
        assert_eq!(q.lexeme, "a\"b");
    }
}
