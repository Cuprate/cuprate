/// Extracts a fixed-size subarray from an array, slice or vector of u8.
/// No copy is made.
///
/// # Parameters
/// - `$array`: Input array, slice or vector of u8 values.
/// - `$start`: Starting index of the subarray.
/// - `$len`: Length of the subarray.
///
/// # Returns
/// A reference to a fixed-size subarray of type `[u8; $len]`.
///
/// # Panics
/// Panics if $start + $len > $`array.len()`.
macro_rules! subarray {
    ($array:expr, $start:expr, $len:expr) => {{
        let sub: &[u8; $len] = (&$array[$start..$start + $len]).try_into().unwrap();
        sub
    }};
}
pub(crate) use subarray;

/// Creates a new fixed-size array copying the bytes from the specified subarray
/// of a parent array, slice or vector uf u8.
///
/// # Parameters
/// - `$array`: Input array, slice or vector of u8 values.
/// - `$start`: Starting index of the subarray.
/// - `$len`: Length of the subarray.
///
/// # Returns
/// A new fixed-size array of type `[u8; $len]`.
///
/// # Panics
/// Panics if $start + $len > $`array.len()`.
macro_rules! subarray_copy {
    ($array:expr, $start:expr, $len:expr) => {{
        let sub: [u8; $len] = $array[$start..$start + $len].try_into().unwrap();
        sub
    }};
}
pub(crate) use subarray_copy;

/// Extracts a mutable subarray from a given array. Changes to the subarray will
/// be reflected in the original array.
///
/// # Parameters
/// - `$array`: Input array, slice or vector of u8 values.
/// - `$start`: The starting index of the subarray.
/// - `$len`: The length of the subarray.
///
/// # Returns
/// A mutable reference to a fixed-size array of type `[u8; $len]`.
///
/// # Panics
/// Panics if $start + $len > $`array.len()`.
macro_rules! subarray_mut {
    ($array:expr, $start:expr, $len:expr) => {{
        let sub: &mut [u8; $len] = (&mut $array[$start..$start + $len]).try_into().unwrap();
        sub
    }};
}
pub(crate) use subarray_mut;

#[cfg(test)]
pub(crate) fn hex_to_array<const N: usize>(hex: &str) -> [u8; N] {
    assert_eq!(
        hex.len(),
        N * 2,
        "Hex string length must be twice the array size"
    );
    hex::decode(hex).unwrap().try_into().unwrap()
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_subarray() {
        let array = [1_u8, 2, 3, 4, 5];
        let sub = subarray!(array, 1, 3);
        assert_eq!(sub, &[2, 3, 4]);
        assert!(std::ptr::eq(&array[1], &sub[0])); // same memory, not copy
    }

    #[test]
    fn test_subarray_copy() {
        let mut array = [1_u8, 2, 3, 4, 5];
        let sub_copied = subarray_copy!(array, 1, 3);
        assert_eq!(sub_copied, [2, 3, 4]);
        array[1] = 10;
        assert_eq!(sub_copied, [2, 3, 4]); // copy, not affected
    }

    #[test]
    fn test_subarray_mut() {
        let mut array = [1_u8, 2, 3, 4, 5];
        let sub = subarray_mut!(array, 1, 2);
        assert_eq!(sub, &[2_u8, 3]);
        sub[0] = 10;
        assert_eq!(array, [1_u8, 10, 3, 4, 5]); // original array modified
    }
}
