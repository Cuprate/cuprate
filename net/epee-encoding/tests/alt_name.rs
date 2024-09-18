#![expect(clippy::tests_outside_test_module, unused_crate_dependencies, reason = "outer test module")]

use cuprate_epee_encoding::{epee_object, from_bytes, to_bytes};

struct AltName {
    val: u8,
    d: u64,
}

epee_object!(
    AltName,
    val("val2"): u8,
    d: u64,
);

struct AltName2 {
    val2: u8,
    d: u64,
}

epee_object!(
    AltName2,
    val2: u8,
    d: u64,
);

#[test]
fn epee_alt_name() {
    let val2 = AltName2 { val2: 40, d: 30 };
    let bytes = to_bytes(val2).unwrap();

    let val: AltName = from_bytes(&mut bytes.clone()).unwrap();

    let bytes2 = to_bytes(val).unwrap();

    assert_eq!(bytes, bytes2);
}
