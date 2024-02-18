//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import

//---------------------------------------------------------------------------------------------------- Free functions
/// The function/algorithm used the
/// database when resizing the memory map.
///
/// This is only used by [`ConcreteEnv`]'s if [`Env::MANUAL_RESIZE`] is `true`.
///
/// # TODO
/// Create some algorithm for increasing the memory map size.
///
/// Possible candidates:
/// - x2 the current (`Vec`-style)?
/// - Do whatever Monero does
pub const fn resize_memory_map(current_size_bytes: usize) -> usize {
    // SAFETY: If this overflows, we should definitely panic.
    // `u64::MAX` bytes is... ~18,446,744 terabytes.
    current_size_bytes * 2
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
