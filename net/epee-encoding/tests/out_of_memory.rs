#![expect(unused_crate_dependencies, reason = "outer test module")]

use cuprate_epee_encoding::{epee_object, from_bytes};

struct T {
    a: Vec<String>,
}

epee_object!(
    T,
    a: Vec<String>,
);

#[test]
fn out_of_memory() {
    #[rustfmt::skip]
    let data = [
        // header
        0x01, 0x11, 0x01, 0x1, 0x01, 0x01, 0x02, 0x1, 0x1,
        // struct + field
        0x04, 0x01, b'a',
        // field tag
        0x80 | 10,
        // varint length of len
        0x03,
        // len, as big as possible
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff
    ]
        .to_vec();

    drop(from_bytes::<T, _>(&mut data.as_slice()));
}
