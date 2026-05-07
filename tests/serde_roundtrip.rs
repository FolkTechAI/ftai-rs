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
