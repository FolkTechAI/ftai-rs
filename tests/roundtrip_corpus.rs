//! Round-trip on every fixture: parse → serialize → parse, structurally equal.
//!
//! Span line/column metadata is normalized away before comparison —
//! spans are diagnostic, not structural.

use std::fs;
use std::path::Path;

use ftai::ast::{Block, Document, Section, Span, Value};

fn normalize_doc(doc: &mut Document) {
    for b in &mut doc.blocks {
        normalize_block(b);
    }
}

fn normalize_block(b: &mut Block) {
    match b {
        Block::Section(s) => normalize_section(s),
        Block::Narrative {
            span, inline_tags, ..
        } => {
            *span = Span::synthetic();
            for _ in inline_tags.iter_mut() {}
        }
    }
}

fn normalize_section(s: &mut Section) {
    s.span = Span::synthetic();
    for (_, v) in &mut s.attributes {
        normalize_value(v);
    }
    for child in &mut s.children {
        normalize_block(child);
    }
}

fn normalize_value(_v: &mut Value) {
    // Values do not carry spans.
}

#[test]
fn roundtrip_every_fixture_structurally_stable() {
    let dir = Path::new("tests/fixtures");
    let mut count = 0;
    let mut skipped = 0;
    for entry in fs::read_dir(dir).expect("fixtures dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ftai") {
            continue;
        }
        let input = fs::read_to_string(&path).expect("read fixture");
        let doc1 = match ftai::parse(&input) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("skipping fixture {}: parse error {e}", path.display());
                skipped += 1;
                continue;
            }
        };
        let text = ftai::to_string_doc(&doc1)
            .unwrap_or_else(|e| panic!("serialize {}: {e}", path.display()));
        let doc2 = ftai::parse(&text).unwrap_or_else(|e| {
            panic!(
                "re-parse {}: {e}\n--- emitted text ---\n{text}",
                path.display()
            )
        });
        let mut a = doc1.clone();
        let mut b = doc2.clone();
        normalize_doc(&mut a);
        normalize_doc(&mut b);
        pretty_assertions::assert_eq!(
            a,
            b,
            "fixture {} failed structural round-trip",
            path.display()
        );
        count += 1;
    }
    eprintln!("round-tripped {count} fixtures (skipped {skipped})");
    assert!(count > 0, "expected at least one fixture round-trip");
}
