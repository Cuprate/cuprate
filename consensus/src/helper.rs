use std::{
    ops::{Add, Div, Mul, Sub},
    time::{SystemTime, UNIX_EPOCH},
};

use curve25519_dalek::edwards::CompressedEdwardsY;

/// Spawns a task for the rayon thread pool and awaits the result without blocking the async runtime.
pub(crate) async fn rayon_spawn_async<F, R>(f: F) -> R
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    let (tx, rx) = tokio::sync::oneshot::channel();
    rayon::spawn(|| {
        let _ = tx.send(f());
    });
    rx.await.expect("The sender must not be dropped")
}

pub(crate) fn get_mid<T>(a: T, b: T) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + Copy + From<u8>,
{
    let two: T = 2_u8.into();

    // https://github.com/monero-project/monero/blob/90294f09ae34ef96f3dea5fea544816786df87c8/contrib/epee/include/misc_language.h#L43
    (a / two) + (b / two) + ((a - two * (a / two)) + (b - two * (b / two))) / two
}

/// Gets the median from a sorted slice.
///
/// If not sorted the output will be invalid.
pub(crate) fn median<T>(array: &[T]) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + Copy + From<u8>,
{
    let mid = array.len() / 2;

    if array.len() == 1 {
        return array[0];
    }

    if array.len() % 2 == 0 {
        get_mid(array[mid - 1], array[mid])
    } else {
        array[mid]
    }
}

pub(crate) fn current_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Checks that a point is canonical.
///
/// https://github.com/dalek-cryptography/curve25519-dalek/issues/380
pub(crate) fn check_point(point: &CompressedEdwardsY) -> bool {
    let bytes = point.as_bytes();

    point
        .decompress()
        // Ban points which are either unreduced or -0
        .filter(|point| point.compress().as_bytes() == bytes)
        .is_some()
}
