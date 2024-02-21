//! General free functions (related to the database).

//---------------------------------------------------------------------------------------------------- Import
#[allow(unused_imports)] // docs
use crate::{env::Env, ConcreteEnv};

//---------------------------------------------------------------------------------------------------- Free functions
/// The function/algorithm used by the
/// database when resizing the memory map.
///
/// This is only used by [`ConcreteEnv`] if [`Env::MANUAL_RESIZE`] is `true`.
///
/// # Method
/// This function mostly matches `monerod`'s current resize implementation,
/// and will increase `current_size_bytes` by a fixed `1_073_745_920` exactly
/// then rounded to the nearest multiple of the OS page size.
///
/// <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L549>
///
/// TODO:
/// We could test around with different algorithms.
/// Calling [`heed::Env::resize`] is surprisingly fast,
/// around `0.0000082s` on my machine. We could probably
/// get away with smaller and more frequent resizes.
///
/// ```rust
/// # use cuprate_database::*;
/// // The value this function will increment by
/// // (assuming page multiple of 4096).
/// const N: usize = 1_073_741_824;
///
/// // 0 returns the minimum value.
/// assert_eq!(resize_memory_map(0), N);
/// // Rounds up to nearest OS page size.
/// assert_eq!(resize_memory_map(1), N + page_size::get());
/// ```
pub fn resize_memory_map(current_size_bytes: usize) -> usize {
    /// The exact expression used by `monerod`
    /// when calculating how many bytes to add.
    ///
    /// The nominal value is `1_073_741_824`.
    /// Not actually 1 GB but close enough I guess.
    ///
    /// <https://github.com/monero-project/monero/blob/059028a30a8ae9752338a7897329fe8012a310d5/src/blockchain_db/lmdb/db_lmdb.cpp#L553>
    const ADD_SIZE: usize = 1_usize << 30;

    // SAFETY: If this overflows, we should definitely panic.
    // `u64::MAX` bytes is... ~18,446,744 terabytes.
    let new_size_bytes = current_size_bytes + ADD_SIZE;

    // INVARIANT: Round up to the nearest OS page size multiple.
    // LMDB/heed will error if this is not the case:
    // <http://www.lmdb.tech/doc/group__mdb.html#gaa2506ec8dab3d969b0e609cd82e619e5>
    //
    // `page_size` itself caches the result, so we don't need to,
    // this function is cheap after 1st call: <https://docs.rs/page_size>.
    let os_page_size = page_size::get();

    // Round up the new size to the
    // nearest multiple of the OS page size.
    //
    // Note that on `resize_memory_map(0)`, the page size
    // will be added to the base `ADD_SIZE`, so actually
    // the minimum value we can return is `ADD_SIZE + os_page_size`.
    let remainder = new_size_bytes % os_page_size;
    if remainder == 0 {
        new_size_bytes
    } else {
        (new_size_bytes + os_page_size) - remainder
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
