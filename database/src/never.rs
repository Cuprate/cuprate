//! The never `!` type, in stable Rust.

//---------------------------------------------------------------------------------------------------- Table
/// Unconstructable never `!` type.
///
/// This is just a newtype around [`Infallible`], it is needed
/// because `cuprate_database` needs to implement serialization
/// traits (even if not used) to satisfy traits bounds, but
/// we cannot implement a foreign trait (e.g [`sanakirja::Storable`])
/// on a foreign type ([`Infallible`]).
///
/// This needs to be a local type to `cuprate_database`,
/// so unfortunately it cannot be in `cuprate_helper`.
///
/// `Infallible` is a 0 variant enum that is unconstructable,
/// it "has the same role as the ! “never” type":
/// <https://doc.rust-lang.org/std/convert/enum.Infallible.html#future-compatibility>.
///
/// FIXME: Use the `!` type when stable.
#[derive(Debug)]
pub(crate) struct Never(std::convert::Infallible);

//---------------------------------------------------------------------------------------------------- Impl
cfg_if::cfg_if! {
    if #[cfg(all(feature = "sanakirja", not(feature = "heed")))] {
        impl sanakirja::Storable for Never {
            type PageReferences = core::iter::Empty<u64>;

            fn page_references(&self) -> Self::PageReferences {
                unreachable!()
            }
        }
    }
}
