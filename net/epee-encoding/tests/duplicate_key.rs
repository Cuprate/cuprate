use cuprate_epee_encoding::{epee_object, from_bytes};

struct T {
    a: u8,
}

epee_object!(
    T,
    a: u8,
);

struct TT {
    a: u8,
}

epee_object!(
    TT,
    a: u8 = 0,
);

#[test]
fn duplicate_key() {
    let data = [
        0x01, 0x11, 0x01, 0x1, 0x01, 0x01, 0x02, 0x1, 0x1, 0x08, 0x01, b'a', 0x0B, 0x00, 0x01,
        b'a', 0x0B, 0x00,
    ];

    assert!(from_bytes::<T, _>(&mut &data[..]).is_err());
}

#[test]
fn duplicate_key_with_default() {
    let data = [
        0x01, 0x11, 0x01, 0x1, 0x01, 0x01, 0x02, 0x1, 0x1, 0x08, 0x01, b'a', 0x0B, 0x00, 0x01,
        b'a', 0x0B, 0x00,
    ];

    assert!(from_bytes::<TT, _>(&mut &data[..]).is_err());
}
