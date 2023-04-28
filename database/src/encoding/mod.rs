//! ## Encoding module
//! The encoding module contains abstraction and implementation of encoding and decoding behavior. Ideally every type should have its own manual implementation,
//! but due to complexity it can sometimes be desired to use an external implementation. This is why this module also contains submodule used for
//! interoperability between bincode2 and monero-rs implementation. In the future, we hope to remove monero-rs dependency, this module might then be moved
//! to the cuprate-common crate.

use std::{fmt::Debug, array::TryFromSliceError, convert::Infallible, io};
use crate::table::{DupTable, Table};

pub mod compat;
pub mod implementation;

/// The `Encode` trait permit the use of manual encoding implementation for database types
/// or bincode2 implementation otherwise.
pub trait Encode: Send + Sync + Sized + Debug {

	/// The encoded output
	type Output: AsRef<[u8]> + Send + Sync;

	/// Provide an encoded output from a reference
	fn encode(&self) -> Result<Self::Output, Error>;
}

/// The `Encode` trait permit the use of manual decoding implementation for database types
/// or bincode2 implementation otherwise.
pub trait Decode: Send + Sync + Sized + Debug {

	/// Decode the type out of a slice
	fn decode<S: AsRef<[u8]> + Send + Sync>(src: &S) -> Result<Self, Error>;
}

/// An enum used to either give the Key type of a table or its already serialized to data to the database
/// to proceed. It's size is the same as the generic used, since the serialized data are smaller
/// than the type in memory.
pub enum Key<T: Table> {
	// The type
	Type(<T>::Key),
	// The encoded type
	Raw(<<T>::Key as Encode>::Output),
}

impl<'a, T: Table> Key<T> {

	fn as_type(&'a self) -> &'a <T as Table>::Key {
		if let Key::Type(key) = self {
			key
		} else {
			unreachable!("as_type must NEVER be used on a Raw type");
		}
	}

	fn as_raw(&'a self) -> &'a <<T as Table>::Key as Encode>::Output {
        if let Key::Raw(raw) = self {
			raw
		} else {
			unreachable!("as_raw must NEVER be used on a Type type");
		}
    }
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

impl<'a, T: Table> Value<T> {

	fn as_type(&'a self) -> &'a <T as Table>::Value {
		if let Value::Type(value) = self {
			value
		} else {
			unreachable!("as_type must NEVER be used on a Raw type");
		}
	}

	fn as_raw(&'a self) -> &'a <<T as Table>::Value as Encode>::Output {
        if let Value::Raw(raw) = self {
			raw
		} else {
			unreachable!("as_raw must NEVER be used on a Type type");
		}
    }
}

/// An enum used to either give the SubKey type of a duptable or its already serialized to data to the database
/// to proceed. It's size is the same as the generic used, since the serialized data are smaller
/// than the type in memory.
pub enum SubKey<T: DupTable> {
	// The type
	Type(<T>::SubKey),
	// The encoded type
	Raw(<<T>::SubKey as Encode>::Output),
}

impl<'a, T: DupTable> SubKey<T> {

	fn as_type(&'a self) -> &'a <T as DupTable>::SubKey {
		if let SubKey::Type(subkey) = self {
			subkey
		} else {
			unreachable!("as_type must NEVER be used on a Raw type");
		}
	}

	fn as_raw(&'a self) -> &'a <<T as DupTable>::SubKey as Encode>::Output {
        if let SubKey::Raw(raw) = self {
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