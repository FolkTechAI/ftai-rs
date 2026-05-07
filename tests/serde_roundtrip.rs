//! Round-trip tests for serde-derive types via FTAI.

use serde::{Deserialize, Serialize};

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
