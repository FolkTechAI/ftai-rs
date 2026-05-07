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
    /// Key-value attributes in source order. Vec (not `HashMap`) preserves order
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
    #[must_use]
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
            "{}:{}-{}:{}",
            self.start_line, self.start_col, self.end_line, self.end_col
        )
    }
}

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
            inline_tags: vec![],
            span: Span::synthetic(),
        };
        match n {
            Block::Narrative { text, .. } => {
                assert_eq!(text, "  raw text with  spaces\n");
            }
            Block::Section(_) => panic!("expected Narrative"),
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
        let s = Span {
            start_line: 10,
            start_col: 4,
            end_line: 10,
            end_col: 12,
        };
        let f = format!("{s}");
        assert!(f.contains("10:4"));
    }
}
