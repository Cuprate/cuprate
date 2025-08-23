/// Extracts a fixed-size subarray from an array, slice, or vector of any type.
/// No copy is made.
///
/// # Parameters
/// - `array`: Input array, slice, or vector of values.
/// - `start`: Starting index of the subarray.
///
/// # Returns
/// A reference to a fixed-size subarray of type `[U; LEN]`.
///
/// # Panics
/// Panics if `start + LEN > array.as_ref().len()`.
#[inline]
pub(crate) fn subarray<T: AsRef<[U]> + ?Sized, U, const LEN: usize>(
    array: &T,
    start: usize,
) -> &[U; LEN] {
    array.as_ref()[start..start + LEN].try_into().unwrap()
}

/// Creates a new fixed-size array copying the elements from the specified subarray
/// of a parent array, slice, or vector.
///
/// # Parameters
/// - `array`: Input array, slice, or vector of copyable values.
/// - `start`: Starting index of the subarray.
///
/// # Returns
/// A new fixed-size array of type `[u8; LEN]`.
///
/// # Panics
/// Panics if `start + LEN > array.as_ref().len()`.
#[inline]
pub(crate) fn subarray_copy<T: AsRef<[U]> + ?Sized, U: Copy, const LEN: usize>(
    array: &T,
    start: usize,
) -> [U; LEN] {
    array.as_ref()[start..start + LEN].try_into().unwrap()
}

/// Extracts a mutable subarray from an array, slice, or vector of any type.
/// Changes to the subarray will be reflected in the original array.
///
/// # Parameters
/// - `array`: Input array, slice, or vector of values.
/// - `start`: Starting index of the subarray.
///
/// # Returns
/// A mutable reference to a fixed-size subarray of type `[U; LEN]`.
///
/// # Panics
/// Panics if `start + LEN > array.as_mut().len()`.
#[inline]
pub(crate) fn subarray_mut<T: AsMut<[U]> + ?Sized, U, const LEN: usize>(
    array: &mut T,
    start: usize,
) -> &mut [U; LEN] {
    (&mut array.as_mut()[start..start + LEN])
        .try_into()
        .unwrap()
}

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
    use super::*;

    #[test]
    fn test_subarray() {
        let array = [1_u8, 2, 3, 4, 5];
        let sub: &[u8; 3] = subarray(&array, 1);
        assert_eq!(sub, &[2, 3, 4]);
        assert!(std::ptr::eq(&raw const array[1], &raw const sub[0])); // same memory, not copy
    }

    #[test]
    fn test_subarray_copy() {
        let mut array = [1_u8, 2, 3, 4, 5];
        let sub_copied: [u8; 3] = subarray_copy(&array, 1);
        assert_eq!(sub_copied, [2, 3, 4]);
        array[1] = 10;
        assert_eq!(sub_copied, [2, 3, 4]); // copy, not affected
    }

    #[test]
    fn test_subarray_mut() {
        let mut array = [1_u8, 2, 3, 4, 5];
        let sub: &mut [u8; 2] = subarray_mut(&mut array, 1);
        assert_eq!(sub, &[2_u8, 3]);
        sub[0] = 10;
        assert_eq!(array, [1_u8, 10, 3, 4, 5]); // original array modified
    }
    #[test]
    #[should_panic(expected = "range end index 4 out of range for slice of length 1")]
    fn subarray_panic() {
        let array = [1_u8];
        let _: &[u8; 3] = subarray(&array, 1);
    }

    #[test]
    #[should_panic(expected = "range end index 4 out of range for slice of length 1")]
    fn subarray_copy_panic() {
        let array = [1_u8];
        let _: [u8; 3] = subarray_copy(&array, 1);
    }

    #[test]
    #[should_panic(expected = "range end index 4 out of range for slice of length 1")]
    fn subarray_mut_panic() {
        let mut array = [1_u8];
        let _: &mut [u8; 3] = subarray_mut(&mut array, 1);
    }
}
