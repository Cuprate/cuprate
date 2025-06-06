use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    io::Read,
};

use bytes::Buf;
use serde_json::{Map, Number};

use crate::{
    epee_object, read_epee_value, read_field_name_bytes, read_head_object, read_header,
    read_marker, read_object, read_varint, EpeeObjectBuilder, EpeeValue, Error, InnerMarker,
    Marker, Result,
};

#[derive(Debug, Clone)]
pub struct EpeeObject {
    pub entry_count: usize,
    pub entries: Vec<Entry>,
}

impl From<&EpeeObject> for Map<String, serde_json::Value> {
    fn from(value: &EpeeObject) -> Self {
        value
            .entries
            .clone()
            .into_iter()
            .map(|entry| (entry.name, entry.value.into()))
            .collect()
    }
}

impl Display for EpeeObject {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(
            &serde_json::to_string_pretty(&Map::<String, serde_json::Value>::from(self)).unwrap(),
        )
    }
}

impl EpeeObject {
    pub fn new<B: Buf>(b: &mut B) -> Result<Self> {
        read_header(b).unwrap();

        let entry_count = read_varint(b)?;
        let mut entries = Vec::with_capacity(entry_count);

        for _ in 0..entry_count {
            let name_bytes = read_field_name_bytes(b)?;
            let name = String::from_utf8(name_bytes.to_vec()).unwrap();
            let marker = read_marker(b)?;

            let (count, len) = if let Some(t) = marker.inner_marker.size() {
                (Some(t as u64), t)
            } else {
                let len = read_varint::<_, u64>(b)?;
                (None, len as usize)
            };

            let value = match marker.inner_marker {
                InnerMarker::I64 => Value::I64(b.get_i64_le()),
                InnerMarker::I32 => Value::I32(b.get_i32_le()),
                InnerMarker::I16 => Value::I16(b.get_i16_le()),
                InnerMarker::I8 => Value::I8(b.get_i8()),
                InnerMarker::U64 => Value::U64(b.get_u64_le()),
                InnerMarker::U32 => Value::U32(b.get_u32_le()),
                InnerMarker::U16 => Value::U16(b.get_u16_le()),
                InnerMarker::U8 => Value::U8(b.get_u8()),
                InnerMarker::F64 => Value::F64(b.get_f64_le()),
                InnerMarker::String => {
                    let s = b.copy_to_bytes(len).to_vec();
                    Value::String(String::from_utf8(s).unwrap())
                }
                InnerMarker::Bool => {
                    let b = match b.get_u8() {
                        0 => false,
                        1 => true,
                        _ => panic!(),
                    };
                    Value::Bool(b)
                }
                InnerMarker::Object => Value::Object(Self::new(b)?),
            };

            entries.push(Entry {
                name,
                marker,
                count,
                value,
            });
        }

        Ok(Self {
            entry_count,
            entries,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub marker: Marker,
    pub count: Option<u64>,
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum Value {
    I64(i64),
    I32(i32),
    I16(i16),
    I8(i8),
    U64(u64),
    U32(u32),
    U16(u16),
    U8(u8),
    F64(f64),
    String(String),
    Bool(bool),
    Object(EpeeObject),
}

#[expect(clippy::fallible_impl_from)]
impl From<Value> for serde_json::Value {
    fn from(value: Value) -> Self {
        match value {
            Value::I64(t) => Self::Number(Number::from(t)),
            Value::I32(t) => Self::Number(Number::from(t)),
            Value::I16(t) => Self::Number(Number::from(t)),
            Value::I8(t) => Self::Number(Number::from(t)),
            Value::U64(t) => Self::Number(Number::from(t)),
            Value::U32(t) => Self::Number(Number::from(t)),
            Value::U16(t) => Self::Number(Number::from(t)),
            Value::U8(t) => Self::Number(Number::from(t)),
            Value::F64(t) => Self::Number(Number::from_f64(t).unwrap()),
            Value::String(t) => Self::String(t),
            Value::Bool(t) => Self::Bool(t),
            Value::Object(t) => Self::Object(Map::<String, Self>::from(&t)),
        }
    }
}
