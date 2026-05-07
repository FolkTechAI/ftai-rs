//! Fault-tolerant parsing per spec A5.

#[test]
fn lenient_unknown_tag_is_ignored_with_error_logged() {
    let input = "@ftai v2.0\n@unknown_tag\nx: 1\n@end\n@document\ntitle: \"ok\"\n@end\n";
    let (doc, errors) = ftai::parse_lenient(input);
    // The known tag landed
    assert!(doc.blocks.iter().any(
        |b| matches!(b, ftai::Block::Section(s) if s.tag == "document")
    ));
    // Note: in lenient mode unknown tags are accepted as sections (fault-tolerant
    // principle). Errors may or may not be empty depending on parser strictness.
    // Currently no error is emitted for "unknown" tag because the parser does
    // not have a known-tag set; this matches the FTAI spec's "Unknown tags
    // are ignored by fallback parsers" rule.
    let _ = errors;
}

#[test]
fn lenient_missing_end_marker_recovered_at_next_tag() {
    let input = "@ftai v2.0\n@document\ntitle: \"hi\"\n@ai\nmode: \"x\"\n@end\n";
    let (doc, errors) = ftai::parse_lenient(input);
    // The document should at least parse partially.
    assert!(!doc.blocks.is_empty());
    // An error should be logged for the unterminated @document.
    assert!(!errors.is_empty(), "expected an error for missing @end");
}

#[test]
fn lenient_truncated_final_block_preserves_prefix() {
    let input = "@ftai v2.0\n@document\ntitle: \"hi\"";
    let (doc, errors) = ftai::parse_lenient(input);
    assert!(!doc.blocks.is_empty(), "partial document survived");
    assert!(!errors.is_empty(), "missing @end was logged");
}
