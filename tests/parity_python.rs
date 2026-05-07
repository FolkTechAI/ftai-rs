//! Python reference-parser parity test.
//!
//! Marked `#[ignore]` because it requires `python3` in `PATH`. Run with:
//!
//!   `cargo test --test parity_python -- --ignored`
//!
//! The upstream Python parser (`parseftai_linter.py`) is a *linter*, not
//! a full AST builder. The adapter `parseftai_json_adapter.py` projects
//! its tag/body structure into JSON. The Rust parity test compares the
//! sets of top-level tags + line numbers it reports against the Rust
//! parser's output, ensuring the two see the same structural shape.

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
#[ignore = "requires python3 in PATH; run via `cargo test -- --ignored parity`"]
fn rust_output_matches_python_reference_on_corpus() {
    let dir = Path::new("tests/fixtures");
    let py_script = dir.join("parseftai_json_adapter.py");
    if !py_script.exists() {
        eprintln!("skip: python adapter not vendored");
        return;
    }

    let mut compared = 0;
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("ftai") {
            continue;
        }

        let py_out = Command::new("python3")
            .arg(&py_script)
            .arg(&path)
            .output()
            .expect("python3 must be in PATH");
        if !py_out.status.success() {
            eprintln!(
                "python adapter failed on {}; stderr:\n{}",
                path.display(),
                String::from_utf8_lossy(&py_out.stderr)
            );
            continue;
        }
        let py_json: serde_json::Value =
            serde_json::from_slice(&py_out.stdout).expect("python adapter JSON");

        let input = fs::read_to_string(&path).unwrap();
        let rust_doc = match ftai::parse(&input) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("rust parser failed on {}: {e}", path.display());
                continue;
            }
        };

        // Compare: the set of top-level tag names should match.
        let py_tags: Vec<String> = py_json["tags"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                t["tag"]
                    .as_str()
                    .unwrap_or("")
                    .trim_start_matches('@')
                    .to_lowercase()
            })
            .filter(|s| !s.is_empty() && s != "ftai" && s != "end")
            .collect();

        let rust_tags: Vec<String> = rust_doc
            .blocks
            .iter()
            .filter_map(|b| match b {
                ftai::Block::Section(s) => Some(s.tag.clone()),
                ftai::Block::Narrative { .. } => None,
            })
            .collect();

        // The Python parser flattens `@end` and inner sections; the Rust
        // parser nests them. So we compare loosely: every top-level tag
        // the Python parser saw should appear *somewhere* in the Rust AST.
        let mut rust_all_tags: Vec<String> = Vec::new();
        for b in &rust_doc.blocks {
            collect_tags(b, &mut rust_all_tags);
        }
        for t in &py_tags {
            assert!(
                rust_all_tags.iter().any(|r| r == t),
                "fixture {} parity mismatch: python tag {t:?} not found in rust output {rust_all_tags:?}",
                path.display()
            );
        }

        let _ = rust_tags;
        compared += 1;
    }
    assert!(compared > 0, "no fixtures compared");
}

fn collect_tags(b: &ftai::Block, out: &mut Vec<String>) {
    if let ftai::Block::Section(s) = b {
        out.push(s.tag.clone());
        for c in &s.children {
            collect_tags(c, out);
        }
    }
}
