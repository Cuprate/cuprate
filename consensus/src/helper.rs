use std::{
    io::{Cursor, Error, ErrorKind},
    ops::{Add, Div, Mul, Sub},
    time::{SystemTime, UNIX_EPOCH},
};

/// Deserializes an object using the give `des` function, checking that all the bytes
/// are consumed.
pub(crate) fn size_check_decode<T>(
    buf: &[u8],
    des: impl Fn(&mut Cursor<&[u8]>) -> Result<T, Error>,
) -> Result<T, Error> {
    let mut cur = Cursor::new(buf);
    let t = des(&mut cur)?;
    if TryInto::<usize>::try_into(cur.position()).unwrap() != buf.len() {
        return Err(Error::new(
            ErrorKind::Other,
            "Data not fully consumed while decoding!",
        ));
    }
    Ok(t)
}

pub(crate) fn get_mid<T>(a: T, b: T) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Div<Output = T> + Mul<Output = T> + Copy + From<u8>,
{
    let two: T = 2_u8.into();

    // https://github.com/monero-project/monero/blob/90294f09ae34ef96f3dea5fea544816786df87c8/contrib/epee/include/misc_language.h#L43
    (a / two) + (b / two) + ((a - two * (a / two)) + (b - two * (b / two))) / two
}

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
