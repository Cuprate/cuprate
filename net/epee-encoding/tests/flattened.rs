use epee_encoding::{epee_object, from_bytes, to_bytes};

struct Child {
    val: u64,
    val2: Vec<u8>,
}

epee_object!(
    Child,
    val: u64,
    val2: Vec<u8>,
);
struct Parent {
    child: Child,
    h: f64,
}

epee_object!(
    Parent,
    h: f64,
    !flatten:
        child: Child,
);

#[derive(Clone)]
struct ParentChild {
    h: f64,
    val: u64,
    val2: Vec<u8>,
}

epee_object!(
    ParentChild,
    h: f64,
    val: u64,
    val2: Vec<u8>,
);

#[test]
fn epee_flatten() {
    let val2 = ParentChild {
        h: 38.9,
        val: 94,
        val2: vec![4, 5],
    };
    let mut bytes = to_bytes(val2.clone()).unwrap();

    let val: Parent = from_bytes(&mut bytes).unwrap();

    assert_eq!(val.child.val2, val2.val2);
    assert_eq!(val.child.val, val2.val);
    assert_eq!(val.h, val2.h);
}

#[derive(Debug, Default, Clone, PartialEq)]
struct Child1 {
    val: u64,
    val2: Vec<u8>,
}

epee_object!(
    Child1,
    val: u64,
    val2: Vec<u8>,
);

#[derive(Debug, Default, Clone, PartialEq)]
struct Child2 {
    buz: u16,
    fiz: String,
}

epee_object!(
    Child2,
    buz: u16,
    fiz: String,
);

#[derive(Debug, Default, Clone, PartialEq)]
struct Parent12 {
    child1: Child1,
    child2: Child2,
    h: f64,
}

epee_object!(
    Parent12,
    h: f64,
    !flatten: child1: Child1,
    !flatten: child2: Child2,
);

#[test]
fn epee_double_flatten() {
    let val = Parent12::default();

    let mut bytes = to_bytes(val.clone()).unwrap();
    let val1: Parent12 = from_bytes(&mut bytes).unwrap();

    assert_eq!(val, val1);
}
