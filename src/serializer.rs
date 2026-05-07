//! Document → canonical FTAI text serializer.
//!
//! Output is deterministic: same input value, same output bytes, every time.

// Wired into the public API by Task 10 (`to_string`).
#![allow(dead_code)]

use crate::ast::{Block, Document, FtaiVersion, Section, Value};
use crate::error::{Error, Result};
use std::fmt::Write;

/// Serialize a [`Document`] to canonical FTAI text.
///
/// # Errors
/// Returns `Err(Error::Io)` if writing into the output buffer fails (effectively never
/// for the in-memory `String` writer, but kept for FFI parity).
pub fn serialize_document(doc: &Document) -> Result<String> {
    let mut out = String::new();
    let v = match doc.version {
        FtaiVersion::V2_0 => "v2.0",
    };
    write!(out, "@ftai {v}").map_err(|e| Error::Io(e.to_string()))?;
    if let Some(s) = &doc.schema {
        write!(out, " {s}").map_err(|e| Error::Io(e.to_string()))?;
    }
    out.push('\n');
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
    write!(out, "{pad}@{}", s.tag).map_err(|e| Error::Io(e.to_string()))?;
    if let Some(hv) = &s.header_value {
        out.push(' ');
        serialize_value_inline(hv, out)?;
    }
    out.push('\n');
    for (k, v) in &s.attributes {
        write!(out, "{pad}{k}: ").map_err(|e| Error::Io(e.to_string()))?;
        serialize_value_inline(v, out)?;
        out.push('\n');
    }
    for child in &s.children {
        serialize_block(child, indent + 1, out)?;
    }
    writeln!(out, "{pad}@end").map_err(|e| Error::Io(e.to_string()))?;
    Ok(())
}

fn serialize_value_inline(v: &Value, out: &mut String) -> Result<()> {
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
                if i > 0 {
                    out.push_str(", ");
                }
                serialize_value_inline(item, out)?;
            }
            out.push(']');
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse_tokens;

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
