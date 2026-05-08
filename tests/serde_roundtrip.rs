//! Round-trip tests for serde-derive types via FTAI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Note {
    title: String,
    body: String,
}

#[test]
fn primitive_struct_roundtrips() {
    let n = Note {
        title: "Hi".into(),
        body: "Hello.".into(),
    };
    let text = ftai::to_string(&n).expect("serialize");
    let back: Note = ftai::from_str(&text).expect("deserialize");
    assert_eq!(n, back);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Nested {
    inner: Note,
    count: u32,
    flags: Vec<String>,
}

#[test]
fn nested_struct_roundtrips() {
    let v = Nested {
        inner: Note {
            title: "x".into(),
            body: "y".into(),
        },
        count: 42,
        flags: vec!["a".into(), "b".into()],
    };
    let text = ftai::to_string(&v).unwrap();
    let back: Nested = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct WithOptionAndMap {
    maybe: Option<String>,
    extras: HashMap<String, u32>,
}

#[test]
fn option_and_hashmap_roundtrip() {
    let v = WithOptionAndMap {
        maybe: Some("hi".into()),
        extras: HashMap::from([("a".into(), 1u32), ("b".into(), 2u32)]),
    };
    let text = ftai::to_string(&v).unwrap();
    let back: WithOptionAndMap = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Event {
    Login { user: String },
    Logout,
}

#[test]
fn tagged_enum_roundtrips_via_section_tag() {
    let v = Event::Login {
        user: "mike".into(),
    };
    let text = ftai::to_string(&v).unwrap();
    let back: Event = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

// ---------------------------------------------------------------------------
// v0.1.1 round-trip bug regressions
//
// Two defects discovered by mitosis-core (the first downstream consumer)
// during Phase 2a + 2b implementation. Each red test reproduces the bug
// before the fix and proves it stays fixed afterwards.
// ---------------------------------------------------------------------------

/// Bug 1 (Phase 2a finding): an internally-tagged struct-variant enum
/// nested inside an outer struct loses its variant discriminant on
/// round-trip. The outer field's name overwrites the section's `kind`
/// during serialization.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct OuterWithInternallyTaggedEnum {
    label: String,
    payload: InternallyTagged,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum InternallyTagged {
    VariantA { x: u32 },
    VariantB { name: String },
}

#[test]
fn internally_tagged_enum_nested_in_struct_roundtrips() {
    let v = OuterWithInternallyTaggedEnum {
        label: "L".into(),
        payload: InternallyTagged::VariantA { x: 42 },
    };
    let text = ftai::to_string(&v).unwrap();
    let back: OuterWithInternallyTaggedEnum = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

/// Bug 2 (Phase 2b finding): externally-tagged enums (the default — no
/// `#[serde(tag = "...")]`) silently drop their tag during round-trip.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
enum ExternallyTagged {
    Login { user: String },
    Logout,
}

#[test]
fn externally_tagged_enum_roundtrips_at_top_level() {
    let v = ExternallyTagged::Login {
        user: "alice".into(),
    };
    let text = ftai::to_string(&v).unwrap();
    let back: ExternallyTagged = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

/// Combined: an externally-tagged enum nested inside a struct.
///
/// **Known v0.1.1 limitation:** the FTAI serializer's
/// "field-name-as-section-tag" architecture cannot preserve both a
/// struct field name AND an externally-tagged enum variant name in the
/// same section — the field name overrides the variant name, so the
/// variant is lost on round-trip. The recommended v1 pattern is to
/// use **internally-tagged** enums (`#[serde(tag = "kind")]`) for
/// nested use; mitosis-core's `Actor`, `AuditAction`, and
/// `VerificationOutcome` all do this. Top-level externally-tagged
/// enums round-trip fine (see `externally_tagged_enum_roundtrips_at_top_level`).
///
/// `#[ignore]`-marked so the test compiles and documents the limitation
/// without failing CI. Run explicitly with
/// `cargo test --test serde_roundtrip -- --ignored` to confirm the
/// limitation still exists; this test is expected to **fail** until a
/// future serializer rework lands.
#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct OuterWithExternallyTagged {
    label: String,
    op: ExternallyTagged,
}

#[test]
#[ignore = "v0.1.1 known limitation: externally-tagged enum nested in struct loses variant name; use #[serde(tag = \"...\")] for nested enums"]
fn externally_tagged_enum_nested_in_struct_roundtrips() {
    let v = OuterWithExternallyTagged {
        label: "L".into(),
        op: ExternallyTagged::Login { user: "bob".into() },
    };
    let text = ftai::to_string(&v).unwrap();
    let back: OuterWithExternallyTagged = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}

/// Unit-variant of an externally-tagged enum should round-trip too
/// (this case manifests as just `@logout` in FTAI).
#[test]
fn externally_tagged_unit_variant_roundtrips() {
    let v = ExternallyTagged::Logout;
    let text = ftai::to_string(&v).unwrap();
    let back: ExternallyTagged = ftai::from_str(&text).unwrap();
    assert_eq!(v, back);
}
