//! ## Encoding module
//! The encoding module contains abstraction and implementation of encoding and decoding behavior. Ideally every type should have its own manual implementation,
//! but due to complexity it can sometimes be desired to use an external implementation. This is why this module also contains submodule used for
//! interoperability between bincode2 and monero-rs implementation. In the future, we hope to remove monero-rs dependency, this module might then be moved
//! to the cuprate-common crate.

use std::{fmt::Debug, array::TryFromSliceError, convert::Infallible, io};
use crate::table::Table;

const BINCODE_CONFIG: bincode::config::Configuration<
    bincode::config::LittleEndian,
    bincode::config::Fixint,
> = bincode::config::standard().with_fixed_int_encoding();

pub mod compat;
pub mod buffer;
pub mod implementation;

/// The `Buffer` trait permit, with generics, to easily generate a new buffer in which an
/// encoded type can be copied into. It is often used in database implementation to separate subkey from value
pub trait Buffer: AsRef<[u8]> + AsMut<[u8]> + Send + Sync + Sized {

	fn new() -> Self;
}

/// The `Encode` trait permit the use of manual encoding implementation for database types
/// or bincode2 implementation otherwise.
pub trait Encode: Send + Sync + Sized + Debug {

	/// The encoded output
	#[cfg(not(feature = "mdbx"))]
	type Output: Buffer;
	#[cfg(feature = "mdbx")]
	type Output: Buffer + for<'a> libmdbx::Decodable<'a>;

	/// Provide an encoded output from a reference
	fn encode(&self) -> Result<Self::Output, Error>;
}

/// The `Decode` trait permit the use of manual decoding implementation for database types
/// or bincode2 implementation otherwise.
pub trait Decode: Send + Sync + Sized + Debug {

	/// Decode the type out of a slice
	fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error>;
}

/// An enum used to either give the Value type of a table or its already serialized to data to the database
/// to proceed. It's size is the same as the generic used, since the serialized data are smaller
/// than the type in memory.
pub enum Value<T: Table> {
	// The type
	Type(<T>::Value),
	// The encoded type
	Raw(<<T>::Value as Encode>::Output),
}

/// Convenient implementation for Interface used. these functions let us get the inner type of the enum.
/// SAFETY: This is relatively safe because when using database methods you now, by setting const B: bool, which 
/// variant of Value you should receive. But be careful on your code, or it might crash.
impl<'a, T: Table> Value<T> {

	pub fn as_type(self) -> <T as Table>::Value {
		assert!(matches!(self, Value::Type(_))); // Hint for the compiler to check the boundaries
		if let Value::Type(value) = self {
			value
		} else {
			unreachable!("as_type must NEVER be used on a Raw type");
		}
	}

	pub fn as_raw(&'a self) -> &'a <<T as Table>::Value as Encode>::Output {
		assert!(matches!(self, Value::Raw(_))); // Hint for the compiler to check the boundaries
        if let Value::Raw(raw) = self {
			raw
		} else {
			unreachable!("as_raw must NEVER be used on a Type type");
		}
    }
}

/// Possible encoding|decoding errors. The majority of these variants are just wrappers of 
/// monero-rs and bincode errors, since their implementations are the most likely to fail.
#[derive(thiserror::Error, Debug)]
pub enum Error {

	#[error("Failed to parse data due to incompatible size: {0}")]
	TryInto(#[from] TryFromSliceError),

	#[error("An error that shouldn't be possible just pop out: {0}")]
	Infallible(#[from] Infallible),

	#[error("An IO procedure failed while encoding or decoding a value: {0}")]
	IoError(#[from] io::Error),

	#[error("Bincode library failed to encode data: {0}")]
	BincodeEncode(#[from] bincode::error::EncodeError),

	#[error("Bincode library failed to decode data: {0}")]
	BincodeDecode(#[from] bincode::error::DecodeError),

	#[error("Monero-rs library failed to parse data: {0}")]
	MoneroEncode(#[from] monero::consensus::encode::Error),

	#[error("Monero-rs can't parse these data as a valid key: {0}")]
	MoneroKey(#[from] monero::util::key::Error),
}