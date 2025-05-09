#![no_main]

use libfuzzer_sys::fuzz_target;

use bytes::{Bytes, BytesMut};

use cuprate_epee_encoding::{epee_object, from_bytes};

const HEADER: &[u8] = b"\x01\x11\x01\x01\x01\x01\x02\x01\x01";

struct T {
    a: u8,
    b: u16,
    c: u32,
    d: u64,
    e: i8,
    f: i16,
    g: i32,
    h: i64,
    i: f64,
    j: String,
    k: bool,

    l: Vec<u8>,
    m: Vec<u16>,
    n: Vec<u32>,
    o: Vec<u64>,
    p: Vec<i8>,
    q: Vec<i16>,
    r: Vec<i32>,
    s: Vec<i64>,
    t: Vec<f64>,
    u: Vec<String>,
    v: Vec<bool>,
    w: Vec<T>,
    x: Vec<[u8; 32]>,

    y: Bytes,
    z: BytesMut,
}
epee_object! (
    T,
    a: u8,
    b: u16,
    c: u32,
    d: u64,
    e: i8,
    f: i16,
    g: i32,
    h: i64,
    i: f64,
    j: String,
    k: bool,

    l: Vec<u8>,
    m: Vec<u16>,
    n: Vec<u32>,
    o: Vec<u64>,
    p: Vec<i8>,
    q: Vec<i16>,
    r: Vec<i32>,
    s: Vec<i64>,
    t: Vec<f64>,
    u: Vec<String>,
    v: Vec<bool>,
    w: Vec<T>,
    x: Vec<[u8; 32]>,

    y: Bytes,
    z: BytesMut,
);

fuzz_target!(|data: &[u8]| {
    let data = [HEADER, data].concat();

    drop(from_bytes::<T, _>(&mut data.as_slice()));
});
