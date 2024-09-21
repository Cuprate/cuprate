#![expect(unused_crate_dependencies, reason = "outer test module")]

use cuprate_epee_encoding::{epee_object, from_bytes, to_bytes};

pub struct Optional {
    val: u8,
    optional_val: i32,
}

epee_object!(
    Optional,
    val: u8,
    optional_val: i32 = -4_i32,
);
pub struct NotOptional {
    val: u8,
    optional_val: i32,
}

epee_object!(
    NotOptional,
    val: u8,
    optional_val: i32,
);

#[derive(Default)]
pub struct NotPresent {
    val: u8,
}

epee_object!(
    NotPresent,
    val: u8,
);

#[test]
fn epee_default_does_not_encode() {
    let val = Optional {
        val: 1,
        optional_val: -4,
    };
    let mut bytes = to_bytes(val).unwrap().freeze();

    assert!(from_bytes::<NotOptional, _>(&mut bytes.clone()).is_err());

    let val: Optional = from_bytes(&mut bytes).unwrap();
    assert_eq!(val.optional_val, -4);
    assert_eq!(val.val, 1);
}

#[test]
fn epee_non_default_does_encode() {
    let val = Optional {
        val: 8,
        optional_val: -3,
    };
    let mut bytes = to_bytes(val).unwrap().freeze();

    assert!(from_bytes::<NotOptional, _>(&mut bytes.clone()).is_ok());

    let val: Optional = from_bytes(&mut bytes).unwrap();
    assert_eq!(val.optional_val, -3);
    assert_eq!(val.val, 8);
}

#[test]
fn epee_value_not_present_with_default() {
    let val = NotPresent { val: 76 };
    let mut bytes = to_bytes(val).unwrap().freeze();

    assert!(from_bytes::<NotOptional, _>(&mut bytes.clone()).is_err());

    let val: Optional = from_bytes(&mut bytes).unwrap();
    assert_eq!(val.optional_val, -4);
    assert_eq!(val.val, 76);
}
