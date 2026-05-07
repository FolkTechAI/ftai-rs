//! CAT 7 (LLM Output Handling) parser hygiene tests.
//!
//! Per spec A7: parsed output must reflect the bytes that were there.
//! Truncated `@end` markers, unbalanced delimiters, and `@end`-like text
//! inside quoted strings must NEVER cause silent recovery that drops
//! content.

#[test]
fn truncated_at_end_in_ai_block_surfaces_error() {
    // @ai block opened but @end is truncated mid-token.
    let input = "@ftai v2.0\n@ai\nmode: \"x\"\n@en";
    let result = ftai::parse(input);
    assert!(result.is_err(), "must error on truncated @end");
}

#[test]
fn embedded_at_end_inside_quoted_string_does_not_close_section() {
    let input = "@ftai v2.0\n@ai\nnote: \"contains @end token-like text\"\n@end\n";
    let doc = ftai::parse(input).expect("strings can contain @end-like text");
    if let ftai::Block::Section(s) = &doc.blocks[0] {
        assert_eq!(s.tag, "ai");
        assert!(s.attributes.iter().any(|(k, _)| k == "note"));
    } else {
        panic!("expected Section");
    }
}

#[test]
fn unbalanced_quoted_string_in_ai_block_errors() {
    let input = "@ftai v2.0\n@ai\nnote: \"unterminated\n@end\n";
    let result = ftai::parse(input);
    assert!(result.is_err(), "unterminated quoted string must error");
}
