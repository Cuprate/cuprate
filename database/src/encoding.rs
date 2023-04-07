//! ### Encoding module
//! The encoding module contains a trait that permit compatibility between `monero-rs` consensus encoding/decoding logic and `bincode` traits.
//! The database tables only accept types that implement [`bincode::Encode`] and [`bincode::Decode`] and since we can't implement these on `monero-rs` types directly
//! we use a wrapper struct `Compat<T>` that permit us to use `monero-rs`'s `consensus_encode`/`consensus_decode` functions under bincode traits.
//! The choice of using `bincode` comes from performance measurement at encoding. Sometimes `bincode` implementations was 5 times faster than `monero-rs` impl.

use std::{ops::Deref, io::Read, fmt::Debug};
use bincode::{de::read::Reader, enc::write::Writer};
use monero::consensus::{Encodable, Decodable};

#[derive(Debug, Clone)]
/// A single-tuple struct, used to contains monero-rs types that implement [`monero::consensus::Encodable`] and [`monero::consensus::Decodable`]
pub struct Compat<T: Encodable +Decodable>(T);

/// A wrapper around a [`bincode::de::read::Reader`] type. Permit us to use [`std::io::Read`] and feed monero-rs functions with an actual `&[u8]`
pub struct ReaderCompat<'src, R: Reader>(pub &'src mut R);

// Actual implementation of `std::io::read` for `bincode`'s `Reader` types
impl<'src, R: Reader> Read for ReaderCompat<'src, R> {
    	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        	self.0.read(buf).map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "bincode reader Error"))?;
		Ok(buf.len())
    	}
}

// Convenient implementation. `Deref` and `From`
impl<T: Encodable + Decodable> Deref for Compat<T> {
	type Target = T;

      	fn deref(&self) -> &Self::Target {
	    	&self.0
      	}
}

impl<T: Encodable + Decodable> From<T> for Compat<T> {
	fn from(value: T) -> Self {
        	Compat(value)
    	}
}

// TODO: Investigate specialization optimization
// Implementation of `bincode::Decode` for monero-rs `Decodable` type
impl<T: Encodable + Decodable + Debug> bincode::Decode for Compat<T> {
	fn decode<D: bincode::de::Decoder>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
		Ok(Compat(Decodable::consensus_decode(&mut ReaderCompat(decoder.reader())).map_err(|_|bincode::error::DecodeError::Other("Monero-rs decoding failed"))?))
	}
}

// Implementation of `bincode::BorrowDecode` for monero-rs `Decodable` type
impl<'de, T: Encodable + Decodable + Debug> bincode::BorrowDecode<'de> for Compat<T> {
	fn borrow_decode<D: bincode::de::BorrowDecoder<'de>>(decoder: &mut D) -> Result<Self, bincode::error::DecodeError> {
	    	Ok(Compat(Decodable::consensus_decode(&mut ReaderCompat(decoder.borrow_reader())).map_err(|_| bincode::error::DecodeError::Other("Monero-rs decoding failed"))?))
	}
}

// Implementation of `bincode::Encode` for monero-rs `Encodable` type
impl<T: Encodable + Decodable + Debug> bincode::Encode for Compat<T> {
    	fn encode<E: bincode::enc::Encoder>(&self, encoder: &mut E) -> Result<(), bincode::error::EncodeError> {
        	let writer = encoder.writer();
		let buf = monero::consensus::serialize(&self.0);
		writer.write(&buf)
    	}
}