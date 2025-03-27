//! String formatting.

/// A type that can be represented in hexadecimal (with a `0x` prefix).
pub trait HexPrefix {
    /// Turn `self` into a hexadecimal string prefixed with `0x`.
    fn hex_prefix(self) -> String;
}

macro_rules! impl_hex_prefix {
    ($(
        $t:ty
    ),*) => {
        $(
            impl HexPrefix for $t {
                fn hex_prefix(self) -> String {
                    format!("{:#x}", self)
                }
            }
        )*
    };
}

impl_hex_prefix!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, usize, isize);

impl HexPrefix for (u64, u64) {
    /// Combine the low and high bits of a [`u128`] as a lower-case hexadecimal string prefixed with `0x`.
    ///
    /// ```rust
    /// # use cuprate_helper::fmt::HexPrefix;
    /// assert_eq!((0, 0).hex_prefix(), "0x0");
    /// assert_eq!((0, u64::MAX).hex_prefix(), "0xffffffffffffffff0000000000000000");
    /// assert_eq!((u64::MAX, 0).hex_prefix(), "0xffffffffffffffff");
    /// assert_eq!((u64::MAX, u64::MAX).hex_prefix(), "0xffffffffffffffffffffffffffffffff");
    /// ```
    fn hex_prefix(self) -> String {
        format!(
            "{:#x}",
            crate::map::combine_low_high_bits_to_u128(self.0, self.1)
        )
    }
}

#[cfg(test)]
mod tests {}
