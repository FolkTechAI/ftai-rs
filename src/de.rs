//! Implements `serde::Deserializer` so `Deserialize` types can be read
//! from FTAI text.
//!
//! Symmetric strategy with `ser.rs`:
//!   - Top-level value MUST be a struct, map, or tagged enum → reads from
//!     the first `@document`-style section in the parsed [`Document`].
//!   - Primitive values are parsed from `Value::Quoted` / `Value::Unquoted`.
//!   - `Vec`/sequence is parsed from `Value::List`.
//!   - Nested struct value is parsed from a child `Section` whose tag
//!     matches the field name.

use std::fmt::Display;

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, IntoDeserializer, MapAccess, SeqAccess,
    VariantAccess, Visitor,
};

use crate::ast::{Block, Document, Section, Value};
use crate::error::{Error, Result};
use crate::lexer::tokenize;
use crate::parser::parse_tokens;

/// Deserialize a `.ftai` source string into a value implementing
/// [`serde::de::DeserializeOwned`].
///
/// # Errors
/// Returns `Err` if the input cannot be parsed or shaped into `T`.
pub fn from_str<T: de::DeserializeOwned>(input: &str) -> Result<T> {
    let tokens = tokenize(input)?;
    let doc = parse_tokens(&tokens)?;
    let mut de = DocumentDeserializer { doc };
    T::deserialize(&mut de)
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serde(msg.to_string())
    }
}

struct DocumentDeserializer {
    doc: Document,
}

impl<'de> Deserializer<'de> for &mut DocumentDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        // Default: present root section as a map.
        let section = std::mem::take(&mut self.doc).blocks;
        let mut owner = DocumentDeserializer {
            doc: Document {
                version: crate::ast::FtaiVersion::V2_0,
                schema: None,
                blocks: section,
            },
        };
        let s = std::mem::take(&mut owner.doc)
            .blocks
            .into_iter()
            .find_map(|b| match b {
                Block::Section(s) => Some(s),
                Block::Narrative { .. } => None,
            })
            .ok_or_else(|| Error::Serde("FTAI document had no top-level section".into()))?;
        SectionDeserializer::new(s).deserialize_any(visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let s = std::mem::take(&mut self.doc)
            .blocks
            .into_iter()
            .find_map(|b| match b {
                Block::Section(s) => Some(s),
                Block::Narrative { .. } => None,
            })
            .ok_or_else(|| Error::Serde("expected top-level section".into()))?;
        SectionDeserializer::new(s).deserialize_map(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_struct("", &[], visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        let s = std::mem::take(&mut self.doc)
            .blocks
            .into_iter()
            .find_map(|b| match b {
                Block::Section(s) => Some(s),
                Block::Narrative { .. } => None,
            })
            .ok_or_else(|| Error::Serde("expected top-level section".into()))?;
        SectionDeserializer::new(s).deserialize_enum(name, variants, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct identifier ignored_any
    }
}

/// Deserializes a single Section as a struct or map.
struct SectionDeserializer {
    section: Section,
}

impl SectionDeserializer {
    fn new(section: Section) -> Self {
        Self { section }
    }
}

impl<'de> Deserializer<'de> for SectionDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let access = SectionMapAccess::new(self.section);
        visitor.visit_map(access)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        // FTAI tags are case-insensitive (spec rule 4). The serializer
        // emits lowercased section tags; here we recover the original
        // case from the static `variants` list so serde's case-sensitive
        // discriminant matcher accepts the variant.
        let tag = self.section.tag.clone();
        let canonical = variants
            .iter()
            .find(|v| v.eq_ignore_ascii_case(&tag))
            .map(std::string::ToString::to_string)
            .unwrap_or(tag);
        visitor.visit_enum(SectionEnumAccess {
            variant: canonical,
            section: self.section,
        })
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }
    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf seq tuple tuple_struct identifier ignored_any
    }
}

struct SectionMapAccess {
    /// Pending entries: each is (key, `FieldSource`) pulled from attributes
    /// and child sections.
    entries: std::vec::IntoIter<(String, FieldSource)>,
    pending_value: Option<FieldSource>,
}

enum FieldSource {
    Value(Value),
    /// One or more child sections sharing the same tag. A single section
    /// is presented as a `Sections` of length 1 so consumers asking for
    /// `Vec<X>` and consumers asking for `X` are both satisfied by the
    /// `SectionsDeserializer` below.
    Sections(Vec<Section>),
}

impl SectionMapAccess {
    fn new(section: Section) -> Self {
        let mut items: Vec<(String, FieldSource)> = Vec::new();
        for (k, v) in section.attributes {
            items.push((k, FieldSource::Value(v)));
        }
        // Group child sections by tag (preserving first-occurrence order)
        // so a `Vec<struct>` field deserializes from repeated children that
        // share the field name as tag.
        let mut tag_order: Vec<String> = Vec::new();
        let mut grouped: std::collections::HashMap<String, Vec<Section>> =
            std::collections::HashMap::new();
        for child in section.children {
            if let Block::Section(child_section) = child {
                let tag = child_section.tag.clone();
                if !grouped.contains_key(&tag) {
                    tag_order.push(tag.clone());
                }
                grouped.entry(tag).or_default().push(child_section);
            }
        }
        for tag in tag_order {
            if let Some(sections) = grouped.remove(&tag) {
                items.push((tag, FieldSource::Sections(sections)));
            }
        }
        Self {
            entries: items.into_iter(),
            pending_value: None,
        }
    }
}

impl<'de> MapAccess<'de> for SectionMapAccess {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>> {
        match self.entries.next() {
            None => Ok(None),
            Some((key, source)) => {
                self.pending_value = Some(source);
                let de = key.into_deserializer();
                seed.deserialize(de).map(Some)
            }
        }
    }

    fn next_value_seed<S: DeserializeSeed<'de>>(&mut self, seed: S) -> Result<S::Value> {
        let source = self
            .pending_value
            .take()
            .ok_or_else(|| Error::Serde("map value without preceding key".into()))?;
        match source {
            FieldSource::Value(v) => seed.deserialize(ValueDeserializer { value: v }),
            FieldSource::Sections(sections) => seed.deserialize(SectionsDeserializer { sections }),
        }
    }
}

/// Deserializer for one or more child sections sharing a tag.
///
/// Routes by what serde asks for:
/// - `deserialize_seq` (when target is `Vec<T>`): yield each section.
/// - `deserialize_struct` / `deserialize_map` / `deserialize_enum` /
///   `deserialize_any` (when target is `T`): take the first section.
///
/// Single-element groups (the common case for nested structs) work
/// transparently on either path.
struct SectionsDeserializer {
    sections: Vec<Section>,
}

impl SectionsDeserializer {
    fn first(self) -> Result<Section> {
        self.sections
            .into_iter()
            .next()
            .ok_or_else(|| Error::Serde("expected child section, found none".into()))
    }
}

impl<'de> Deserializer<'de> for SectionsDeserializer {
    type Error = Error;

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_seq(SectionsSeqAccess {
            iter: self.sections.into_iter(),
        })
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_map(visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_struct(name, fields, visitor)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_enum(name, variants, visitor)
    }

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_any(visitor)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        if self.sections.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_newtype_struct(name, visitor)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_unit(visitor)
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        SectionDeserializer::new(self.first()?).deserialize_unit_struct(name, visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 u8 u16 u32 u64 f32 f64 char str string
        bytes byte_buf identifier ignored_any
    }
}

struct SectionsSeqAccess {
    iter: std::vec::IntoIter<Section>,
}

impl<'de> SeqAccess<'de> for SectionsSeqAccess {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        match self.iter.next() {
            Some(s) => seed.deserialize(SectionDeserializer::new(s)).map(Some),
            None => Ok(None),
        }
    }
}

struct ValueDeserializer {
    value: Value,
}

impl ValueDeserializer {
    fn as_str(&self) -> &str {
        match &self.value {
            Value::Quoted(s) | Value::Unquoted(s) => s,
            Value::List(_) => "",
        }
    }

    fn parse_int<T: std::str::FromStr>(&self) -> Result<T>
    where
        T::Err: Display,
    {
        self.as_str()
            .trim()
            .parse::<T>()
            .map_err(|e| Error::Serde(format!("integer parse error: {e}")))
    }

    fn parse_float<T: std::str::FromStr>(&self) -> Result<T>
    where
        T::Err: Display,
    {
        self.as_str()
            .trim()
            .parse::<T>()
            .map_err(|e| Error::Serde(format!("float parse error: {e}")))
    }
}

impl<'de> Deserializer<'de> for ValueDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            // Quoted strings are unambiguously strings.
            Value::Quoted(s) => visitor.visit_string(s),
            // Unquoted values are ambiguous — could be int, float, bool,
            // or identifier-like string. For self-describing-format
            // contexts (e.g., serde's internally-tagged-enum content
            // visitor calling deserialize_any on each field), try int →
            // float → bool → string in that priority. The first
            // successful parse wins. This matches the behaviour
            // consumers expect when they call `from_str::<MyType>` and
            // `MyType` has a numeric field on a nested enum variant.
            Value::Unquoted(s) => {
                let trimmed = s.trim();
                // Integer first (signed then unsigned). Try i64 to cover
                // the widest signed range; for u64 values that exceed
                // i64::MAX we fall through to u64.
                if let Ok(i) = trimmed.parse::<i64>() {
                    return visitor.visit_i64(i);
                }
                if let Ok(u) = trimmed.parse::<u64>() {
                    return visitor.visit_u64(u);
                }
                // Float next.
                if let Ok(f) = trimmed.parse::<f64>() {
                    return visitor.visit_f64(f);
                }
                // Bool tokens.
                match trimmed {
                    "true" | "True" | "TRUE" => return visitor.visit_bool(true),
                    "false" | "False" | "FALSE" => return visitor.visit_bool(false),
                    _ => {}
                }
                // Fall back to string.
                visitor.visit_string(s)
            }
            Value::List(items) => visitor.visit_seq(ListSeqAccess {
                iter: items.into_iter(),
            }),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.as_str().trim();
        let b = match s {
            "true" | "True" | "TRUE" | "yes" | "1" => true,
            "false" | "False" | "FALSE" | "no" | "0" => false,
            other => return Err(Error::Serde(format!("not a bool: {other:?}"))),
        };
        visitor.visit_bool(b)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i8(self.parse_int()?)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i16(self.parse_int()?)
    }
    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i32(self.parse_int()?)
    }
    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_i64(self.parse_int()?)
    }
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u8(self.parse_int()?)
    }
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u16(self.parse_int()?)
    }
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u32(self.parse_int()?)
    }
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_u64(self.parse_int()?)
    }
    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f32(self.parse_float()?)
    }
    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_f64(self.parse_float()?)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        let s = self.as_str();
        let mut chars = s.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(Error::Serde(format!("not a char: {s:?}"))),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::Quoted(s) | Value::Unquoted(s) => visitor.visit_string(s),
            Value::List(_) => Err(Error::Serde("expected string, got list".into())),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::List(items) => {
                let mut bytes = Vec::with_capacity(items.len());
                for item in items {
                    let n: u8 = match item {
                        Value::Quoted(s) | Value::Unquoted(s) => s
                            .trim()
                            .parse()
                            .map_err(|e| Error::Serde(format!("byte parse: {e}")))?,
                        Value::List(_) => {
                            return Err(Error::Serde("expected byte, got nested list".into()));
                        }
                    };
                    bytes.push(n);
                }
                visitor.visit_byte_buf(bytes)
            }
            _ => Err(Error::Serde("expected list of bytes".into())),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        // Treat an empty string value as None; everything else is Some.
        // (Empty lists are still Some(vec![]).)
        let is_none = match &self.value {
            Value::Quoted(s) | Value::Unquoted(s) => s.is_empty(),
            Value::List(_) => false,
        };
        if is_none {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        match self.value {
            Value::List(items) => visitor.visit_seq(ListSeqAccess {
                iter: items.into_iter(),
            }),
            _ => Err(Error::Serde("expected list".into())),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _len: usize, visitor: V) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value> {
        Err(Error::Serde(
            "cannot deserialize map from scalar value; expected a section".into(),
        ))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value> {
        Err(Error::Serde(
            "cannot deserialize struct from scalar value; expected a section".into(),
        ))
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        // Unit-variant case: the value is the variant name string.
        let s = match self.value {
            Value::Quoted(s) | Value::Unquoted(s) => s,
            Value::List(_) => return Err(Error::Serde("cannot deserialize enum from list".into())),
        };
        visitor.visit_enum(s.into_deserializer())
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_str(visitor)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value> {
        self.deserialize_any(visitor)
    }
}

struct ListSeqAccess {
    iter: std::vec::IntoIter<Value>,
}

impl<'de> SeqAccess<'de> for ListSeqAccess {
    type Error = Error;
    fn next_element_seed<T: DeserializeSeed<'de>>(&mut self, seed: T) -> Result<Option<T::Value>> {
        match self.iter.next() {
            Some(value) => seed.deserialize(ValueDeserializer { value }).map(Some),
            None => Ok(None),
        }
    }
}

struct SectionEnumAccess {
    variant: String,
    section: Section,
}

impl<'de> EnumAccess<'de> for SectionEnumAccess {
    type Error = Error;
    type Variant = SectionEnumVariant;

    fn variant_seed<V: DeserializeSeed<'de>>(self, seed: V) -> Result<(V::Value, Self::Variant)> {
        let variant = seed.deserialize(self.variant.into_deserializer())?;
        Ok((
            variant,
            SectionEnumVariant {
                section: self.section,
            },
        ))
    }
}

struct SectionEnumVariant {
    section: Section,
}

impl<'de> VariantAccess<'de> for SectionEnumVariant {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value> {
        // Find a single attribute named "value", or any sole attribute.
        let attr = self
            .section
            .attributes
            .into_iter()
            .find(|(k, _)| k == "value")
            .ok_or_else(|| Error::Serde("newtype variant missing 'value' attribute".into()))?;
        seed.deserialize(ValueDeserializer { value: attr.1 })
    }

    fn tuple_variant<V: Visitor<'de>>(self, _len: usize, _visitor: V) -> Result<V::Value> {
        Err(Error::Serde("tuple variants not supported".into()))
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value> {
        SectionDeserializer::new(self.section).deserialize_map(visitor)
    }
}
