//! Parser: token stream → Document AST.
//!
//! Recursive-descent over the EBNF in `ftai-spec/grammar/ftai.ebnf`,
//! extended pragmatically to handle the v2.0 corpus (same-line tag values,
//! narrative content inside sections, bullet/table lines preserved as
//! narrative children).

use crate::ast::{Block, Document, FtaiVersion, InlineTag, Section, Span, Value};
use crate::error::{Error, Result};
use crate::lexer::{Token, TokenKind};

/// Default nesting depth limit. See Task 8 for hardening.
pub const DEFAULT_NESTING_LIMIT: usize = 64;

/// Parse a token stream into a Document.
///
/// # Errors
/// Returns `Err` on the first structural failure (unterminated block,
/// unsupported version, unexpected token, depth-limit breach).
pub fn parse_tokens(tokens: &[Token]) -> Result<Document> {
    let mut p = Parser::new(tokens);
    p.parse_document()
}

/// Parse leniently — accumulates errors instead of returning on the first.
pub fn parse_tokens_lenient(tokens: &[Token]) -> (Document, Vec<Error>) {
    let mut p = Parser::new(tokens);
    p.lenient = true;
    let doc = p.parse_document().unwrap_or_default();
    (doc, std::mem::take(&mut p.errors))
}

pub(crate) struct Parser<'a> {
    tokens: &'a [Token],
    pos: usize,
    depth: usize,
    nesting_limit: usize,
    lenient: bool,
    errors: Vec<Error>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Self {
        Self {
            tokens,
            pos: 0,
            depth: 0,
            nesting_limit: DEFAULT_NESTING_LIMIT,
            lenient: false,
            errors: Vec::new(),
        }
    }

    fn peek(&self) -> &Token {
        // Safe: tokenize always appends Eof.
        &self.tokens[self.pos.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos.min(self.tokens.len() - 1)];
        if !matches!(t.kind, TokenKind::Eof) {
            self.pos += 1;
        }
        t
    }

    fn expect(&mut self, kind: &TokenKind, expected_label: &str) -> Result<&Token> {
        if self.peek_kind() == kind {
            Ok(self.advance())
        } else {
            let t = self.peek();
            Err(Error::UnexpectedToken {
                expected: expected_label.to_string(),
                found: format!("{:?} ({:?})", t.kind, t.lexeme),
                line: t.span.start_line,
                column: t.span.start_col,
            })
        }
    }

    fn current_line(&self) -> usize {
        self.peek().span.start_line
    }

    /// Skip any number of consecutive Newline tokens.
    fn skip_newlines(&mut self) {
        while matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    fn parse_document(&mut self) -> Result<Document> {
        // Optional leading newlines/whitespace are not produced by the lexer
        // for spaces/tabs alone, but blank lines are.
        self.skip_newlines();
        let mut doc = Document::default();

        // @ftai header
        self.expect(&TokenKind::At, "@")?;
        let ftai_id = self.expect(&TokenKind::Identifier, "identifier 'ftai'")?;
        if !ftai_id.lexeme.eq_ignore_ascii_case("ftai") {
            return Err(Error::UnexpectedToken {
                expected: "identifier 'ftai'".into(),
                found: ftai_id.lexeme.clone(),
                line: ftai_id.span.start_line,
                column: ftai_id.span.start_col,
            });
        }
        // Read header tail: a sequence of value tokens until newline.
        let header_parts = self.collect_value_tokens_until_newline();
        let mut version_set = false;
        let mut schema: Option<String> = None;
        for part in &header_parts {
            let normalized = part.trim().trim_start_matches('v').to_lowercase();
            if normalized == "2.0" {
                doc.version = FtaiVersion::V2_0;
                version_set = true;
            } else if part.starts_with('v') && part.contains('.') {
                // Looks like a version other than v2.0.
                return Err(Error::UnsupportedVersion(part.clone()));
            } else if !part.is_empty() {
                // Treat any non-version token as a schema/name.
                schema = Some(part.clone());
            }
        }
        // If no explicit version was found, accept v2.0 (header may be free-form).
        if !version_set && header_parts.is_empty() {
            return Err(Error::UnexpectedToken {
                expected: "version after @ftai".into(),
                found: "newline".into(),
                line: self.current_line(),
                column: 1,
            });
        }
        doc.schema = schema;
        // Consume trailing newline of header.
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }

        // Top-level blocks.
        loop {
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::At => {
                    let block = self.parse_at_block()?;
                    doc.blocks.push(block);
                }
                TokenKind::NarrativeSeparator => {
                    let block = self.parse_narrative()?;
                    doc.blocks.push(block);
                }
                _ => {
                    // Stray content at top level → treat as narrative line for
                    // resilience; in strict mode, error.
                    if self.lenient {
                        let block = self.parse_narrative_line_run();
                        doc.blocks.push(block);
                    } else {
                        let t = self.peek();
                        return Err(Error::UnexpectedToken {
                            expected: "block start ('@' or '---')".into(),
                            found: format!("{:?}", t.kind),
                            line: t.span.start_line,
                            column: t.span.start_col,
                        });
                    }
                }
            }
        }

        Ok(doc)
    }

    /// Parse a `@tag ... @end` section or a tag-single line.
    #[allow(clippy::too_many_lines)]
    fn parse_at_block(&mut self) -> Result<Block> {
        let at_tok = self.advance().clone();
        // Allow `@@subsection` (corpus pattern) by collapsing leading `@` runs.
        while matches!(self.peek_kind(), TokenKind::At) {
            self.advance();
        }
        let id_tok = self.expect(&TokenKind::Identifier, "tag identifier")?.clone();
        let tag_name = id_tok.lexeme.to_lowercase();
        let opening_line = at_tok.span.start_line;

        // Same-line value (optional).
        let header_parts = self.collect_value_tokens_until_newline();
        let header_value = if header_parts.is_empty() {
            None
        } else {
            Some(Value::Unquoted(header_parts.join(" ")))
        };
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }

        // Distinguish section (`@tag ... @end`) from tag-single (`@tag value`).
        //
        // Rule:
        //   - If a matching `@end` exists ahead at this depth, it's a section.
        //   - Else if the next content line is `Identifier ":"` (an attribute),
        //     it's an unterminated section (will produce `UnterminatedBlock`).
        //   - Else it's a tag-single whose body is the same-line value plus
        //     subsequent narrative content lines until the next `@tag`.
        let has_end = self.has_matching_end_ahead();
        let next_is_attribute = matches!(self.peek_kind(), TokenKind::Identifier)
            && matches!(self.peek_at(1).map(|t| &t.kind), Some(TokenKind::Colon));

        if !has_end && !next_is_attribute {
            // Consume any trailing narrative lines that belong to this
            // tag-single (until the next `@tag`, `---`, or EOF).
            let mut narrative_buf: Vec<String> = Vec::new();
            loop {
                self.skip_newlines();
                match self.peek_kind() {
                    TokenKind::Eof | TokenKind::At | TokenKind::NarrativeSeparator => break,
                    _ => {
                        let line = self.consume_line_as_text();
                        if !line.is_empty() {
                            narrative_buf.push(line);
                        }
                    }
                }
            }
            let children = if narrative_buf.is_empty() {
                Vec::new()
            } else {
                let text = narrative_buf.join("\n");
                let inline_tags = extract_inline_tags(&text);
                vec![Block::Narrative {
                    text,
                    inline_tags,
                    span: Span::synthetic(),
                }]
            };
            return Ok(Block::Section(Section {
                tag: tag_name,
                header_value,
                attributes: Vec::new(),
                children,
                span: Span {
                    start_line: opening_line,
                    start_col: at_tok.span.start_col,
                    end_line: self.current_line(),
                    end_col: 1,
                },
            }));
        }

        self.depth += 1;
        if self.depth > self.nesting_limit {
            self.depth -= 1;
            return Err(Error::NestingTooDeep {
                limit: self.nesting_limit,
                line: opening_line,
            });
        }
        let mut attributes: Vec<(String, Value)> = Vec::new();
        let mut children: Vec<Block> = Vec::new();
        let mut narrative_buf: Vec<String> = Vec::new();

        loop {
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Eof => {
                    if self.lenient {
                        self.errors.push(Error::UnterminatedBlock {
                            tag: tag_name.clone(),
                            line: opening_line,
                        });
                        break;
                    }
                    self.depth -= 1;
                    return Err(Error::UnterminatedBlock {
                        tag: tag_name.clone(),
                        line: opening_line,
                    });
                }
                TokenKind::At => {
                    // @end terminator?
                    if self.is_end_marker() {
                        self.consume_end_marker();
                        break;
                    }
                    // Flush any pending narrative buffer.
                    if !narrative_buf.is_empty() {
                        children.push(Block::Narrative {
                            text: narrative_buf.join("\n"),
                            inline_tags: extract_inline_tags(&narrative_buf.join("\n")),
                            span: Span::synthetic(),
                        });
                        narrative_buf.clear();
                    }
                    let child = self.parse_at_block()?;
                    children.push(child);
                }
                TokenKind::Identifier => {
                    // key: value attribute, or a narrative line.
                    if matches!(self.peek_at(1).map(|t| &t.kind), Some(TokenKind::Colon)) {
                        let (k, v) = self.parse_attribute()?;
                        attributes.push((k, v));
                    } else {
                        let line_text = self.consume_line_as_text();
                        narrative_buf.push(line_text);
                    }
                }
                TokenKind::NarrativeSeparator => {
                    // A `---` inside a section — treat as a horizontal rule
                    // marker preserved in narrative text.
                    self.advance();
                    if matches!(self.peek_kind(), TokenKind::Newline) {
                        self.advance();
                    }
                    narrative_buf.push("---".into());
                }
                _ => {
                    // Any other content → narrative line.
                    let line_text = self.consume_line_as_text();
                    if !line_text.is_empty() {
                        narrative_buf.push(line_text);
                    }
                }
            }
        }

        if !narrative_buf.is_empty() {
            let joined = narrative_buf.join("\n");
            children.push(Block::Narrative {
                text: joined.clone(),
                inline_tags: extract_inline_tags(&joined),
                span: Span::synthetic(),
            });
        }

        self.depth -= 1;
        Ok(Block::Section(Section {
            tag: tag_name,
            header_value,
            attributes,
            children,
            span: Span {
                start_line: opening_line,
                start_col: at_tok.span.start_col,
                end_line: self.current_line(),
                end_col: 1,
            },
        }))
    }

    fn parse_attribute(&mut self) -> Result<(String, Value)> {
        let key_tok = self
            .expect(&TokenKind::Identifier, "attribute key")?
            .clone();
        self.expect(&TokenKind::Colon, ":")?;
        let value = self.parse_value()?;
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
        Ok((key_tok.lexeme, value))
    }

    fn parse_value(&mut self) -> Result<Value> {
        match self.peek_kind() {
            TokenKind::QuotedString => {
                let t = self.advance().clone();
                Ok(Value::Quoted(t.lexeme))
            }
            TokenKind::LeftBracket => self.parse_list(),
            TokenKind::Newline | TokenKind::Eof => {
                // Empty value.
                Ok(Value::Unquoted(String::new()))
            }
            _ => {
                let parts = self.collect_value_tokens_until_newline();
                Ok(Value::Unquoted(parts.join(" ")))
            }
        }
    }

    fn parse_list(&mut self) -> Result<Value> {
        self.expect(&TokenKind::LeftBracket, "[")?;
        let mut items = Vec::new();
        // Allow leading whitespace/newlines inside list.
        loop {
            self.skip_newlines();
            if matches!(self.peek_kind(), TokenKind::RightBracket) {
                self.advance();
                break;
            }
            let v = self.parse_list_element()?;
            items.push(v);
            self.skip_newlines();
            match self.peek_kind() {
                TokenKind::Comma => {
                    self.advance();
                }
                TokenKind::RightBracket => {
                    self.advance();
                    break;
                }
                TokenKind::Eof => {
                    return Err(Error::UnexpectedToken {
                        expected: "]".into(),
                        found: "EOF".into(),
                        line: self.current_line(),
                        column: 1,
                    });
                }
                _ => {
                    let t = self.peek();
                    return Err(Error::UnexpectedToken {
                        expected: ", or ]".into(),
                        found: format!("{:?}", t.kind),
                        line: t.span.start_line,
                        column: t.span.start_col,
                    });
                }
            }
        }
        Ok(Value::List(items))
    }

    fn parse_list_element(&mut self) -> Result<Value> {
        match self.peek_kind() {
            TokenKind::QuotedString => {
                let t = self.advance().clone();
                Ok(Value::Quoted(t.lexeme))
            }
            TokenKind::LeftBracket => self.parse_list(),
            _ => {
                // Collect tokens until comma, ], or newline.
                let mut buf = Vec::new();
                loop {
                    match self.peek_kind() {
                        TokenKind::Comma | TokenKind::RightBracket | TokenKind::Eof => break,
                        TokenKind::Newline => {
                            self.advance();
                        }
                        TokenKind::At => {
                            // `@medication` inside a list — store as the
                            // verbatim lexeme `@<id>`.
                            self.advance();
                            if let TokenKind::Identifier = self.peek_kind() {
                                let id = self.advance().clone();
                                buf.push(format!("@{}", id.lexeme));
                            } else {
                                buf.push("@".into());
                            }
                        }
                        _ => {
                            let t = self.advance().clone();
                            buf.push(t.lexeme);
                        }
                    }
                }
                Ok(Value::Unquoted(buf.join(" ").trim().to_string()))
            }
        }
    }

    fn collect_value_tokens_until_newline(&mut self) -> Vec<String> {
        let mut parts = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline
                | TokenKind::Eof
                | TokenKind::At
                | TokenKind::LeftBracket
                | TokenKind::RightBracket
                | TokenKind::NarrativeSeparator => break,
                TokenKind::QuotedString
                | TokenKind::Identifier
                | TokenKind::UnquotedString
                | TokenKind::Colon
                | TokenKind::Comma => {
                    let t = self.advance().clone();
                    parts.push(t.lexeme);
                }
            }
        }
        parts
    }

    fn consume_line_as_text(&mut self) -> String {
        let mut parts = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Newline | TokenKind::Eof => break,
                TokenKind::At => {
                    // Inline `@something` within narrative text.
                    self.advance();
                    if let TokenKind::Identifier = self.peek_kind() {
                        let id = self.advance().clone();
                        parts.push(format!("@{}", id.lexeme));
                    } else {
                        parts.push("@".into());
                    }
                }
                TokenKind::QuotedString => {
                    let t = self.advance().clone();
                    parts.push(format!("\"{}\"", t.lexeme));
                }
                _ => {
                    let t = self.advance().clone();
                    parts.push(t.lexeme);
                }
            }
        }
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
        parts.join(" ")
    }

    fn parse_narrative(&mut self) -> Result<Block> {
        let start_line = self.current_line();
        // Consume opening `---`.
        self.expect(&TokenKind::NarrativeSeparator, "---")?;
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
        let mut text_lines: Vec<String> = Vec::new();
        loop {
            match self.peek_kind() {
                TokenKind::Eof => break,
                TokenKind::NarrativeSeparator => {
                    // Closing separator.
                    self.advance();
                    if matches!(self.peek_kind(), TokenKind::Newline) {
                        self.advance();
                    }
                    break;
                }
                TokenKind::At => {
                    // Narrative ends at next top-level `@`.
                    break;
                }
                _ => {
                    let line = self.consume_line_as_text();
                    text_lines.push(line);
                }
            }
        }
        let text = text_lines.join("\n");
        let inline_tags = extract_inline_tags(&text);
        Ok(Block::Narrative {
            text,
            inline_tags,
            span: Span {
                start_line,
                start_col: 1,
                end_line: self.current_line(),
                end_col: 1,
            },
        })
    }

    fn parse_narrative_line_run(&mut self) -> Block {
        let start_line = self.current_line();
        let line = self.consume_line_as_text();
        Block::Narrative {
            text: line.clone(),
            inline_tags: extract_inline_tags(&line),
            span: Span {
                start_line,
                start_col: 1,
                end_line: self.current_line(),
                end_col: 1,
            },
        }
    }

    fn is_end_marker(&self) -> bool {
        if !matches!(self.peek_kind(), TokenKind::At) {
            return false;
        }
        matches!(
            self.peek_at(1).map(|t| (&t.kind, t.lexeme.as_str())),
            Some((TokenKind::Identifier, "end" | "End" | "END"))
        )
    }

    fn consume_end_marker(&mut self) {
        // Consumes `@end` and trailing newline.
        if matches!(self.peek_kind(), TokenKind::At) {
            self.advance();
        }
        if matches!(self.peek_kind(), TokenKind::Identifier) {
            self.advance();
        }
        // Optional trailing same-line content (e.g. corpus has `@end // ...`).
        let _ = self.collect_value_tokens_until_newline();
        if matches!(self.peek_kind(), TokenKind::Newline) {
            self.advance();
        }
    }

    /// Look ahead through the remaining tokens, accounting for nesting,
    /// to see if there's a matching `@end` at our current depth.
    fn has_matching_end_ahead(&self) -> bool {
        let mut depth: i32 = 0;
        let mut i = self.pos;
        while i < self.tokens.len() {
            let t = &self.tokens[i];
            if matches!(t.kind, TokenKind::Eof) {
                break;
            }
            if matches!(t.kind, TokenKind::At) {
                // Skip leading `@@`s.
                let mut j = i + 1;
                while j < self.tokens.len() && matches!(self.tokens[j].kind, TokenKind::At) {
                    j += 1;
                }
                if let Some(next) = self.tokens.get(j) {
                    if matches!(next.kind, TokenKind::Identifier) {
                        if next.lexeme.eq_ignore_ascii_case("end") {
                            if depth == 0 {
                                return true;
                            }
                            depth -= 1;
                            i = j + 1;
                            continue;
                        }
                        // It's a tag opener — check if THIS opener has its
                        // own `@end` ahead by recursing implicitly via depth.
                        depth += 1;
                        i = j + 1;
                        continue;
                    }
                }
            }
            i += 1;
        }
        false
    }
}

/// Extract `[name:value]` inline tags from a narrative text. Best-effort
/// scan; the original `text` is preserved verbatim by the caller.
fn extract_inline_tags(text: &str) -> Vec<InlineTag> {
    let mut tags = Vec::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'[' {
            if let Some(close) = text[i + 1..].find(']') {
                let inside = &text[i + 1..i + 1 + close];
                if let Some(colon) = inside.find(':') {
                    let name = inside[..colon].trim().to_string();
                    let value = inside[colon + 1..].trim().to_string();
                    if !name.is_empty() {
                        tags.push(InlineTag { name, value });
                    }
                }
                i += 1 + close + 1;
                continue;
            }
        }
        i += 1;
    }
    tags
}

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
