#![cfg_attr(not(feature = "std"), no_std)]
//! Epee Encoding
//!
//! This library contains the Epee binary format found in Monero, unlike other
//! crates this crate does not use serde.
//!
//! See [`epee_object`] for how to easily implement [`EpeeObject`] for your types.
//!
//! example without macro:
//! ```rust
//! # use cuprate_epee_encoding::{EpeeObject, EpeeObjectBuilder, read_epee_value, write_field, to_bytes, from_bytes};
//! # use bytes::{Buf, BufMut};
//!
//! pub struct Test {
//!     val: u64
//! }
//!
//! #[derive(Default)]
//! pub struct __TestEpeeBuilder {
//!     val: Option<u64>,
//! }
//!
//! impl EpeeObjectBuilder<Test> for __TestEpeeBuilder {
//!     fn add_field<B: Buf>(&mut self, name: &str, r: &mut B) -> cuprate_epee_encoding::error::Result<bool> {
//!         match name {
//!             "val" => {self.val = Some(read_epee_value(r)?);}
//!             _ => return Ok(false),
//!         }
//!         Ok(true)
//!     }
//!
//!     fn finish(self) -> cuprate_epee_encoding::error::Result<Test> {
//!         Ok(
//!             Test {
//!                 val: self.val.ok_or_else(|| cuprate_epee_encoding::error::Error::Format("Required field was not found!"))?
//!             }
//!         )
//!     }
//! }
//!
//! impl EpeeObject for Test {
//!     type Builder = __TestEpeeBuilder;
//!
//!     fn number_of_fields(&self) -> u64 {
//!         1
//!     }
//!
//!     fn write_fields<B: BufMut>(self, w: &mut B) -> cuprate_epee_encoding::error::Result<()> {
//!        // write the fields
//!        write_field(self.val, "val", w)
//!    }
//! }
//!
//!
//! let data = [1, 17, 1, 1, 1, 1, 2, 1, 1, 4, 3, 118, 97, 108, 5, 4, 0, 0, 0, 0, 0, 0, 0]; // the data to decode;
//! let val: Test = from_bytes(&mut data.as_slice()).unwrap();
//! let data = to_bytes(val).unwrap();
//!
//!
//! ```

#[cfg(test)]
use hex as _;

extern crate alloc;

use alloc::string::ToString;
use core::str::from_utf8 as str_from_utf8;

use bytes::{Buf, BufMut, Bytes, BytesMut};

pub mod container_as_blob;
pub mod error;
mod io;
pub mod macros;
pub mod marker;
mod value;
mod varint;

pub use error::*;
use io::{checked_read, checked_read_primitive, checked_write, checked_write_primitive};
pub use marker::{InnerMarker, Marker};
pub use value::EpeeValue;
pub use varint::{read_varint, write_varint};

/// Header that needs to be at the beginning of every binary blob that follows
/// this binary serialization format.
const HEADER: &[u8] = b"\x01\x11\x01\x01\x01\x01\x02\x01\x01";
/// The maximum length a byte array (marked as a string) can be.
const MAX_STRING_LEN_POSSIBLE: usize = 2000000000;
/// The maximum depth of skipped objects.
const MAX_DEPTH_OF_SKIPPED_OBJECTS: u8 = 20;
/// The maximum number of fields in an object.
const MAX_NUM_FIELDS: u64 = 1000;

/// A trait for an object that can build a type `T` from the epee format.
pub trait EpeeObjectBuilder<T>: Default + Sized {
    /// Called when a field names has been read no other bytes following the field
    /// name will have been read.
    ///
    /// Returns a bool if true then the field has been read otherwise the field is not
    /// needed and has not been read.
    fn add_field<B: Buf>(&mut self, name: &str, b: &mut B) -> Result<bool>;

    /// Called when the number of fields has been read.
    fn finish(self) -> Result<T>;
}

/// A trait for an object that can be turned into epee bytes.
pub trait EpeeObject: Sized {
    type Builder: EpeeObjectBuilder<Self>;

    /// Returns the number of fields to be encoded.
    fn number_of_fields(&self) -> u64;

    /// write the objects fields into the writer.
    fn write_fields<B: BufMut>(self, w: &mut B) -> Result<()>;
}

/// Read the object `T` from a byte array.
pub fn from_bytes<T: EpeeObject, B: Buf>(buf: &mut B) -> Result<T> {
    read_head_object(buf)
}

/// Turn the object into epee bytes.
pub fn to_bytes<T: EpeeObject>(val: T) -> Result<BytesMut> {
    let mut buf = BytesMut::new();
    write_head_object(val, &mut buf)?;
    Ok(buf)
}

fn read_header<B: Buf>(r: &mut B) -> Result<()> {
    let buf = checked_read(r, |b: &mut B| b.copy_to_bytes(HEADER.len()), HEADER.len())?;

    if &*buf != HEADER {
        return Err(Error::Format("Data does not contain header"));
    }
    Ok(())
}

fn write_header<B: BufMut>(w: &mut B) -> Result<()> {
    checked_write(w, BufMut::put_slice, HEADER, HEADER.len())
}

fn write_head_object<T: EpeeObject, B: BufMut>(val: T, w: &mut B) -> Result<()> {
    write_header(w)?;
    val.write(w)
}

fn read_head_object<T: EpeeObject, B: Buf>(r: &mut B) -> Result<T> {
    read_header(r)?;
    let mut skipped_objects = 0;
    read_object(r, &mut skipped_objects)
}

fn read_field_name_bytes<B: Buf>(r: &mut B) -> Result<Bytes> {
    let len: usize = r.get_u8().into();

    checked_read(r, |b: &mut B| b.copy_to_bytes(len), len)
}

fn write_field_name<B: BufMut>(val: &str, w: &mut B) -> Result<()> {
    checked_write_primitive(w, BufMut::put_u8, val.len().try_into()?)?;
    let slice = val.as_bytes();
    checked_write(w, BufMut::put_slice, slice, slice.len())
}

/// Write an epee field.
pub fn write_field<T: EpeeValue, B: BufMut>(val: T, field_name: &str, w: &mut B) -> Result<()> {
    if val.should_write() {
        write_field_name(field_name, w)?;
        write_epee_value(val, w)?;
    }
    Ok(())
}

fn read_object<T: EpeeObject, B: Buf>(r: &mut B, skipped_objects: &mut u8) -> Result<T> {
    let mut object_builder = T::Builder::default();

    let number_o_field = read_varint(r)?;

    if number_o_field > MAX_NUM_FIELDS {
        return Err(Error::Format(
            "Data has object with more fields than the maximum allowed",
        ));
    }

    for _ in 0..number_o_field {
        let field_name_bytes = read_field_name_bytes(r)?;
        let field_name = str_from_utf8(&field_name_bytes)?;

        if !object_builder.add_field(field_name, r)? {
            skip_epee_value(r, skipped_objects)?;
        }
    }
    object_builder.finish()
}

/// Read a marker from the [`Buf`], this function should only be used for
/// custom serialisation based on the marker otherwise just use [`read_epee_value`].
pub fn read_marker<B: Buf>(r: &mut B) -> Result<Marker> {
    Marker::try_from(checked_read_primitive(r, Buf::get_u8)?)
}

/// Read an epee value from the stream, an epee value is the part after the key
/// including the marker.
pub fn read_epee_value<T: EpeeValue, B: Buf>(r: &mut B) -> Result<T> {
    let marker = read_marker(r)?;
    T::read(r, &marker)
}

/// Write an epee value to the stream, an epee value is the part after the key
/// including the marker.
fn write_epee_value<T: EpeeValue, B: BufMut>(val: T, w: &mut B) -> Result<()> {
    checked_write_primitive(w, BufMut::put_u8, T::MARKER.as_u8())?;
    val.write(w)
}

/// Write a byte array to `w` with [`write_varint`].
///
/// This function:
/// - Writes the length of `t`'s bytes into `w` using [`write_varint`]
/// - Writes `t`'s bytes into `w`
///
/// It is used as the internal [`EpeeValue::write`]
/// implementation of byte-like containers such as:
/// - [`EpeeValue::<Vec<u8>>::write`]
/// - [`EpeeValue::<String>::write`]
///
/// # Errors
/// This will error if:
/// - [`write_varint`] fails
/// - `w` does not have enough capacity
///
/// # Example
/// ```rust
/// let t: [u8; 8] = [3, 0, 0, 0, 1, 0, 0, 0];
/// let mut w = vec![];
///
/// cuprate_epee_encoding::write_bytes(t, &mut w).unwrap();
///
/// assert_eq!(w.len(), 9); // length of bytes + bytes
/// assert_eq!(w[1..], t);
/// ```
pub fn write_bytes<T: AsRef<[u8]>, B: BufMut>(t: T, w: &mut B) -> Result<()> {
    let bytes = t.as_ref();
    let len = bytes.len();

    write_varint(len, w)?;

    if w.remaining_mut() < len {
        return Err(Error::IO("Not enough capacity to write bytes"));
    }

    w.put_slice(bytes);

    Ok(())
}

/// Write an [`Iterator`] of [`EpeeValue`]s to `w` with [`write_varint`].
///
/// This function:
/// - Writes the length of the `iterator`, into `w` using [`write_varint`]
/// - [`EpeeValue::write`]s each `T` of the iterator into `w`
///
/// It is used as the internal [`EpeeValue::write`]
/// implementation of containers such as [`EpeeValue::<Vec<T>>::write`].
///
/// # Errors
/// This will error if:
/// - [`write_varint`] fails
/// - [`EpeeValue::<T>::write`] fails
///
/// # Example
/// ```rust
/// let t: u64 = 3;
/// let vec: Vec<u64> = vec![t, t];
/// let mut w = vec![];
///
/// let iter: std::vec::IntoIter<u64> = vec.into_iter();
/// cuprate_epee_encoding::write_iterator(iter, &mut w).unwrap();
///
/// assert_eq!(w.len(), 17);
/// assert_eq!(w[1..9], [3, 0, 0, 0, 0, 0, 0, 0]);
/// assert_eq!(w[9..], [3, 0, 0, 0, 0, 0, 0, 0]);
/// ```
pub fn write_iterator<T, I, B>(iterator: I, w: &mut B) -> Result<()>
where
    T: EpeeValue,
    I: Iterator<Item = T> + ExactSizeIterator,
    B: BufMut,
{
    write_varint(iterator.len(), w)?;
    for item in iterator {
        item.write(w)?;
    }
    Ok(())
}

/// A helper object builder that just skips every field.
#[derive(Default)]
struct SkipObjectBuilder;

impl EpeeObjectBuilder<SkipObject> for SkipObjectBuilder {
    fn add_field<B: Buf>(&mut self, _name: &str, _r: &mut B) -> Result<bool> {
        Ok(false)
    }

    fn finish(self) -> Result<SkipObject> {
        Ok(SkipObject)
    }
}

/// A helper object that just skips every field.
struct SkipObject;

impl EpeeObject for SkipObject {
    type Builder = SkipObjectBuilder;

    fn number_of_fields(&self) -> u64 {
        panic!("This is a helper function to use when de-serialising")
    }

    fn write_fields<B: BufMut>(self, _w: &mut B) -> Result<()> {
        panic!("This is a helper function to use when de-serialising")
    }
}

/// Skip an epee value, should be used when you do not need the value
/// stored at a key.
fn skip_epee_value<B: Buf>(r: &mut B, skipped_objects: &mut u8) -> Result<()> {
    let marker = read_marker(r)?;

    let len = if marker.is_seq { read_varint(r)? } else { 1 };

    if let Some(size) = marker.inner_marker.size() {
        let bytes_to_skip = size
            .checked_mul(len.try_into()?)
            .ok_or(Error::Value("List is too big".to_string()))?;
        return advance(bytes_to_skip, r);
    };

    for _ in 0..len {
        match marker.inner_marker {
            InnerMarker::I64
            | InnerMarker::U64
            | InnerMarker::F64
            | InnerMarker::I32
            | InnerMarker::U32
            | InnerMarker::I16
            | InnerMarker::U16
            | InnerMarker::I8
            | InnerMarker::U8
            | InnerMarker::Bool => unreachable!("These types are constant size."),
            InnerMarker::String => {
                let len = read_varint(r)?;
                advance(len, r)?;
            }
            InnerMarker::Object => {
                *skipped_objects += 1;
                if *skipped_objects > MAX_DEPTH_OF_SKIPPED_OBJECTS {
                    return Err(Error::Format("Depth of skipped objects exceeded maximum"));
                }
                read_object::<SkipObject, _>(r, skipped_objects)?;
                *skipped_objects -= 1;
            }
        };
    }
    Ok(())
}

fn advance<B: Buf>(n: usize, b: &mut B) -> Result<()> {
    checked_read(b, |b: &mut B| b.advance(n), n)
}
