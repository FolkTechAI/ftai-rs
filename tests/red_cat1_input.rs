//! CAT 1 (Input Injection) red tests for the lexer.
//!
//! Each input MUST be rejected by `tokenize` with a precise error
//! category — never panic, never silently accept.

use ftai::error::{Error, ErrorCategory};

fn ftai_internal_tokenize(input: &str) -> Result<Vec<ftai::__testing::Token>, Error> {
    ftai::__testing::tokenize(input)
}

#[test]
fn red_null_byte_rejected() {
    let input = "@doc\0\n";
    let result = ftai_internal_tokenize(input);
    let err = result.expect_err("null byte must be rejected");
    assert_eq!(err.category(), ErrorCategory::InputInjection);
}

#[test]
fn red_oversize_tag_name_rejected() {
    let oversize_tag: String = "@".to_string() + &"a".repeat(257);
    let input = format!("{oversize_tag}\n");
    let err = ftai_internal_tokenize(&input).expect_err("oversize tag must be rejected");
    assert!(matches!(
        err.category(),
        ErrorCategory::InputInjection | ErrorCategory::LimitExceeded
    ));
}

#[test]
fn red_control_character_in_value_rejected() {
    let input = "k: ab\x07cd\n"; // bell character \x07
    let err = ftai_internal_tokenize(input).expect_err("control char must be rejected");
    assert_eq!(err.category(), ErrorCategory::InputInjection);
}

#[test]
fn red_malformed_utf8_rejected_at_str_boundary() {
    // Note: &str guarantees UTF-8 validity; the only way to feed malformed
    // UTF-8 to a public API is via a byte-buffer entry point. v1 takes &str
    // only, so this test asserts the construction precondition.
    // Constructed via Vec to bypass the `invalid_from_utf8` lint that
    // fires on byte-array literals known at compile time.
    let bad: Vec<u8> = vec![b'@', 0xFF, 0xFE, b'\n'];
    let result = std::str::from_utf8(&bad);
    assert!(
        result.is_err(),
        "construction precondition: bad UTF-8 doesn't form &str"
    );
}

#[test]
fn red_nesting_depth_exceeded_returns_clean_error() {
    // Build a document with 100 levels of nested @block @end ... — exceeds default 64.
    use std::fmt::Write as _;
    let mut s = String::from("@ftai v2.0\n");
    for i in 0..100 {
        writeln!(s, "@nest{i}").unwrap();
    }
    for _ in 0..100 {
        s.push_str("@end\n");
    }
    let err = ftai::parse(&s).expect_err("excessive nesting must be rejected");
    assert_eq!(err.category(), ErrorCategory::LimitExceeded);
}

#[test]
fn red_nesting_depth_in_lexer_path_does_not_panic_on_pathological_brackets() {
    // Lexer doesn't enforce nesting (parser does), but pathological bracket
    // sequences must not cause stack overflow during lexing.
    let input = "[".repeat(10_000);
    // Tokenize must complete without panic; correctness is parser's concern.
    let _ = ftai_internal_tokenize(&input);
}
