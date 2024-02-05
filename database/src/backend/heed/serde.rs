//! TODO

//---------------------------------------------------------------------------------------------------- Import
use crate::pod::Pod;

use std::borrow::Cow;

//---------------------------------------------------------------------------------------------------- Constants

//---------------------------------------------------------------------------------------------------- Serde
/// Implement `heed` (de)serialization traits
/// for anything that implements [`crate::pod::Pod`].
///
/// Blanket implementation runs into some errors, so this is used instead.
macro_rules! impl_heed {
    ($(
        $t:ty // The type that implements [`crate::pod::Pod`], to implement heed serde traits.
    ),* $(,)?) => {
        $(
            impl<'a> heed::BytesEncode<'a> for $t {
                type EItem = $t;

                fn bytes_encode(item: &'a $t) -> Result<Cow<'a, [u8]>, heed::BoxedError> {
                    Ok(Cow::Borrowed(Pod::as_bytes(item).as_ref()))
                }
            }
        )*
    };
}

impl_heed! {}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
