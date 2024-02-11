//! (De)serialization trait implementations for `heed`.

//---------------------------------------------------------------------------------------------------- Import
use crate::pod::Pod;

use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Serde
/// Implement `heed` (de)serialization traits
/// for anything that implements [`crate::pod::Pod`].
///
/// Blanket implementation breaks orphan impl rules, so this is used instead.
macro_rules! impl_heed {
    ($(
        $name:ident => // The name that implements [`crate::pod::Pod`]
        $t:ident       // The type to (de)serialize into/from
    ),* $(,)?) => {
        $(
            // `heed` Encode.
            impl<'a> heed::BytesEncode<'a> for $name {
                type EItem = $t;

                #[inline]
                fn bytes_encode(item: &'a Self::EItem) -> Result<Cow<'a, [u8]>, heed::BoxedError> {
                    Ok(item.into_bytes())
                }
            }

            // `heed` Decode.
            impl<'a> heed::BytesDecode<'a> for $name {
                type DItem = $t;

                #[inline]
                fn bytes_decode(bytes: &'a [u8]) -> Result<Self::DItem, heed::BoxedError> {
                    match Pod::from_bytes(bytes) {
                        Ok(o) => Ok(o),
                        Err(e) => todo!(),
                    }
                }
            }
        )*
    };
}

/// TODO
struct Test;

impl_heed! {
    Test => u8,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
