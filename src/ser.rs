//! Implements `serde::Serializer` so any `Serialize` type can be written
//! out as FTAI text.
//!
//! Strategy:
//!   - Top-level value MUST be a struct, map, or tagged enum variant →
//!     becomes a `Document` with a single `@document` section.
//!   - Primitive fields → `Value::Unquoted` for numbers/bools, `Value::Quoted`
//!     for strings.
//!   - `Vec`/sequence → `Value::List`.
//!   - Nested struct → child `Section` (tag = field name).
//!   - `Option::None` → field omitted; `Option::Some(x)` → `x` serialized.
//!   - Tagged enum (`#[serde(tag = "...")]`) → `Section` whose tag is the
//!     variant name.

use std::fmt::Display;

use serde::ser::{
    self, Impossible, SerializeMap, SerializeSeq, SerializeStruct, SerializeStructVariant,
    SerializeTuple, SerializeTupleStruct, SerializeTupleVariant,
};

use crate::ast::{Block, Document, FtaiVersion, Section, Span, Value};
use crate::error::{Error, Result};
use crate::serializer::serialize_document;

/// Serialize a value implementing [`serde::Serialize`] to FTAI text.
///
/// # Errors
/// Returns `Err(Error::Serde)` if the value is not representable as FTAI
/// (e.g. the top-level value is not a struct, map, or named variant).
pub fn to_string<T: ser::Serialize + ?Sized>(value: &T) -> Result<String> {
    let mut root = RootSerializer::default();
    value.serialize(&mut root)?;
    let doc = root.finish();
    serialize_document(&doc)
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serde(msg.to_string())
    }
}

#[derive(Default)]
struct RootSerializer {
    document: Option<Document>,
}

impl RootSerializer {
    fn finish(self) -> Document {
        self.document.unwrap_or_default()
    }

    fn set_section(&mut self, section: Section) {
        let mut doc = Document {
            version: FtaiVersion::V2_0,
            schema: None,
            blocks: Vec::new(),
        };
        doc.blocks.push(Block::Section(section));
        self.document = Some(doc);
    }
}

fn unsupported_top_level(kind: &str) -> Error {
    Error::Serde(format!(
        "top-level FTAI value must be a struct, map, or named variant; got {kind}"
    ))
}

impl<'a> ser::Serializer for &'a mut RootSerializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = MapBuilder<'a>;
    type SerializeStruct = StructBuilder<'a>;
    type SerializeStructVariant = StructBuilder<'a>;

    fn serialize_bool(self, _v: bool) -> Result<()> {
        Err(unsupported_top_level("bool"))
    }
    fn serialize_i8(self, _v: i8) -> Result<()> {
        Err(unsupported_top_level("i8"))
    }
    fn serialize_i16(self, _v: i16) -> Result<()> {
        Err(unsupported_top_level("i16"))
    }
    fn serialize_i32(self, _v: i32) -> Result<()> {
        Err(unsupported_top_level("i32"))
    }
    fn serialize_i64(self, _v: i64) -> Result<()> {
        Err(unsupported_top_level("i64"))
    }
    fn serialize_u8(self, _v: u8) -> Result<()> {
        Err(unsupported_top_level("u8"))
    }
    fn serialize_u16(self, _v: u16) -> Result<()> {
        Err(unsupported_top_level("u16"))
    }
    fn serialize_u32(self, _v: u32) -> Result<()> {
        Err(unsupported_top_level("u32"))
    }
    fn serialize_u64(self, _v: u64) -> Result<()> {
        Err(unsupported_top_level("u64"))
    }
    fn serialize_f32(self, _v: f32) -> Result<()> {
        Err(unsupported_top_level("f32"))
    }
    fn serialize_f64(self, _v: f64) -> Result<()> {
        Err(unsupported_top_level("f64"))
    }
    fn serialize_char(self, _v: char) -> Result<()> {
        Err(unsupported_top_level("char"))
    }
    fn serialize_str(self, _v: &str) -> Result<()> {
        Err(unsupported_top_level("str"))
    }
    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(unsupported_top_level("bytes"))
    }
    fn serialize_none(self) -> Result<()> {
        Err(unsupported_top_level("None"))
    }
    fn serialize_some<T: ser::Serialize + ?Sized>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<()> {
        Err(unsupported_top_level("unit"))
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Err(unsupported_top_level("unit struct"))
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        // Treat as a section with tag = variant, no fields.
        self.set_section(Section {
            tag: variant.to_lowercase(),
            header_value: None,
            attributes: Vec::new(),
            children: Vec::new(),
            span: Span::synthetic(),
        });
        Ok(())
    }
    fn serialize_newtype_struct<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        let v = value_of(value)?;
        let attributes = vec![("value".to_string(), v)];
        self.set_section(Section {
            tag: variant.to_lowercase(),
            header_value: None,
            attributes,
            children: Vec::new(),
            span: Span::synthetic(),
        });
        Ok(())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(unsupported_top_level("seq"))
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(unsupported_top_level("tuple"))
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(unsupported_top_level("tuple struct"))
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(unsupported_top_level("tuple variant"))
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(MapBuilder {
            root: self,
            section: Section {
                tag: "document".into(),
                header_value: None,
                attributes: Vec::new(),
                children: Vec::new(),
                span: Span::synthetic(),
            },
            pending_key: None,
        })
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Ok(StructBuilder {
            root: self,
            section: Section {
                tag: "document".into(),
                header_value: None,
                attributes: Vec::new(),
                children: Vec::new(),
                span: Span::synthetic(),
            },
        })
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(StructBuilder {
            root: self,
            section: Section {
                tag: variant.to_lowercase(),
                header_value: None,
                attributes: Vec::new(),
                children: Vec::new(),
                span: Span::synthetic(),
            },
        })
    }
}

struct StructBuilder<'a> {
    root: &'a mut RootSerializer,
    section: Section,
}

impl SerializeStruct for StructBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        add_field(&mut self.section, key.to_string(), value)
    }
    fn end(self) -> Result<()> {
        self.root.set_section(self.section);
        Ok(())
    }
}

impl SerializeStructVariant for StructBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        add_field(&mut self.section, key.to_string(), value)
    }
    fn end(self) -> Result<()> {
        self.root.set_section(self.section);
        Ok(())
    }
}

struct MapBuilder<'a> {
    root: &'a mut RootSerializer,
    section: Section,
    pending_key: Option<String>,
}

impl SerializeMap for MapBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ser::Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        let k = key_of(key)?;
        self.pending_key = Some(k);
        Ok(())
    }
    fn serialize_value<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| Error::Serde("map value without key".into()))?;
        add_field(&mut self.section, key, value)
    }
    fn end(self) -> Result<()> {
        self.root.set_section(self.section);
        Ok(())
    }
}

fn add_field<T: ser::Serialize + ?Sized>(
    section: &mut Section,
    key: String,
    value: &T,
) -> Result<()> {
    let mut node = NodeSerializer::default();
    value.serialize(&mut node)?;
    match node.into_outcome() {
        Outcome::Skipped => Ok(()),
        Outcome::Value(v) => {
            section.attributes.push((key, v));
            Ok(())
        }
        Outcome::SubSection(mut child) => {
            child.tag = key.to_lowercase();
            section.children.push(Block::Section(child));
            Ok(())
        }
    }
}

/// Outcome of serializing a single value (for an attribute, or a child section).
enum Outcome {
    Skipped,
    Value(Value),
    SubSection(Section),
}

#[derive(Default)]
struct NodeSerializer {
    outcome: Option<Outcome>,
}

impl NodeSerializer {
    fn set(&mut self, o: Outcome) {
        self.outcome = Some(o);
    }
    fn into_outcome(self) -> Outcome {
        self.outcome.unwrap_or(Outcome::Skipped)
    }
}

fn value_of<T: ser::Serialize + ?Sized>(value: &T) -> Result<Value> {
    let mut node = NodeSerializer::default();
    value.serialize(&mut node)?;
    match node.into_outcome() {
        Outcome::Value(v) => Ok(v),
        Outcome::Skipped => Ok(Value::Unquoted(String::new())),
        Outcome::SubSection(_) => Err(Error::Serde("expected scalar value, got struct".into())),
    }
}

fn key_of<T: ser::Serialize + ?Sized>(key: &T) -> Result<String> {
    let v = value_of(key)?;
    match v {
        Value::Quoted(s) | Value::Unquoted(s) => Ok(s),
        Value::List(_) => Err(Error::Serde("map keys must be scalars, got list".into())),
    }
}

impl<'a> ser::Serializer for &'a mut NodeSerializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SeqBuilder<'a>;
    type SerializeTuple = SeqBuilder<'a>;
    type SerializeTupleStruct = SeqBuilder<'a>;
    type SerializeTupleVariant = SeqBuilder<'a>;
    type SerializeMap = NodeMapBuilder<'a>;
    type SerializeStruct = NodeStructBuilder<'a>;
    type SerializeStructVariant = NodeStructBuilder<'a>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(v.to_string())));
        Ok(())
    }
    fn serialize_char(self, v: char) -> Result<()> {
        self.set(Outcome::Value(Value::Quoted(v.to_string())));
        Ok(())
    }
    fn serialize_str(self, v: &str) -> Result<()> {
        self.set(Outcome::Value(Value::Quoted(v.to_string())));
        Ok(())
    }
    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        // Encode as a list of u8 numbers (deterministic, lossless for round-trip).
        let items = v
            .iter()
            .map(|b| Value::Unquoted(b.to_string()))
            .collect();
        self.set(Outcome::Value(Value::List(items)));
        Ok(())
    }
    fn serialize_none(self) -> Result<()> {
        self.set(Outcome::Skipped);
        Ok(())
    }
    fn serialize_some<T: ser::Serialize + ?Sized>(self, value: &T) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(String::new())));
        Ok(())
    }
    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        self.serialize_unit()
    }
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.set(Outcome::Value(Value::Unquoted(variant.to_string())));
        Ok(())
    }
    fn serialize_newtype_struct<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<()> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ser::Serialize + ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<()> {
        // Encode as a child section named after the variant with one attr "value".
        let v = value_of(value)?;
        let mut section = Section {
            tag: variant.to_lowercase(),
            header_value: None,
            attributes: Vec::new(),
            children: Vec::new(),
            span: Span::synthetic(),
        };
        section.attributes.push(("value".into(), v));
        self.set(Outcome::SubSection(section));
        Ok(())
    }
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Ok(SeqBuilder {
            node: self,
            items: Vec::new(),
        })
    }
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SeqBuilder {
            node: self,
            items: Vec::new(),
        })
    }
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SeqBuilder {
            node: self,
            items: Vec::new(),
        })
    }
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Ok(SeqBuilder {
            node: self,
            items: Vec::new(),
        })
    }
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Ok(NodeMapBuilder {
            node: self,
            section: Section {
                tag: "map".into(),
                header_value: None,
                attributes: Vec::new(),
                children: Vec::new(),
                span: Span::synthetic(),
            },
            pending_key: None,
        })
    }
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        Ok(NodeStructBuilder {
            node: self,
            section: Section {
                tag: "struct".into(),
                header_value: None,
                attributes: Vec::new(),
                children: Vec::new(),
                span: Span::synthetic(),
            },
        })
    }
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Ok(NodeStructBuilder {
            node: self,
            section: Section {
                tag: variant.to_lowercase(),
                header_value: None,
                attributes: Vec::new(),
                children: Vec::new(),
                span: Span::synthetic(),
            },
        })
    }
}

struct SeqBuilder<'a> {
    node: &'a mut NodeSerializer,
    items: Vec<Value>,
}

impl SeqBuilder<'_> {
    fn push_serialize<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let v = value_of(value)?;
        self.items.push(v);
        Ok(())
    }

    fn finish(self) {
        self.node.set(Outcome::Value(Value::List(self.items)));
    }
}

impl SerializeSeq for SeqBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.push_serialize(value)
    }
    fn end(self) -> Result<()> {
        self.finish();
        Ok(())
    }
}

impl SerializeTuple for SeqBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_element<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.push_serialize(value)
    }
    fn end(self) -> Result<()> {
        self.finish();
        Ok(())
    }
}

impl SerializeTupleStruct for SeqBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.push_serialize(value)
    }
    fn end(self) -> Result<()> {
        self.finish();
        Ok(())
    }
}

impl SerializeTupleVariant for SeqBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        self.push_serialize(value)
    }
    fn end(self) -> Result<()> {
        self.finish();
        Ok(())
    }
}

struct NodeStructBuilder<'a> {
    node: &'a mut NodeSerializer,
    section: Section,
}

impl SerializeStruct for NodeStructBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        add_field(&mut self.section, key.to_string(), value)
    }
    fn end(self) -> Result<()> {
        self.node.set(Outcome::SubSection(self.section));
        Ok(())
    }
}

impl SerializeStructVariant for NodeStructBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_field<T: ser::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<()> {
        add_field(&mut self.section, key.to_string(), value)
    }
    fn end(self) -> Result<()> {
        self.node.set(Outcome::SubSection(self.section));
        Ok(())
    }
}

struct NodeMapBuilder<'a> {
    node: &'a mut NodeSerializer,
    section: Section,
    pending_key: Option<String>,
}

impl SerializeMap for NodeMapBuilder<'_> {
    type Ok = ();
    type Error = Error;
    fn serialize_key<T: ser::Serialize + ?Sized>(&mut self, key: &T) -> Result<()> {
        let k = key_of(key)?;
        self.pending_key = Some(k);
        Ok(())
    }
    fn serialize_value<T: ser::Serialize + ?Sized>(&mut self, value: &T) -> Result<()> {
        let key = self
            .pending_key
            .take()
            .ok_or_else(|| Error::Serde("map value without key".into()))?;
        add_field(&mut self.section, key, value)
    }
    fn end(self) -> Result<()> {
        self.node.set(Outcome::SubSection(self.section));
        Ok(())
    }
}
