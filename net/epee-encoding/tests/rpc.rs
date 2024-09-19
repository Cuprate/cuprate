#![expect(unused_crate_dependencies, reason = "outer test module")]

use cuprate_epee_encoding::{epee_object, from_bytes, to_bytes};

#[derive(Clone, Debug, PartialEq)]
struct BaseResponse {
    credits: u64,
    status: String,
    top_hash: String,
    untrusted: bool,
}

epee_object!(
    BaseResponse,
    credits: u64,
    status: String,
    top_hash: String,
    untrusted: bool,
);

#[derive(Clone, Debug, PartialEq)]
struct GetOIndexesResponse {
    base: BaseResponse,
    o_indexes: Vec<u64>,
}

epee_object!(
    GetOIndexesResponse,
    o_indexes: Vec<u64>,
    !flatten:
        base: BaseResponse,
);

#[derive(Clone, Debug, PartialEq)]
struct GetOutsResponse {
    base: BaseResponse,
    outs: Vec<OutKey>,
}

epee_object!(
    GetOutsResponse,
    outs: Vec<OutKey>,
    !flatten:
        base: BaseResponse,
);

#[derive(Clone, Copy, Debug, PartialEq)]
struct OutKey {
    height: u64,
    key: [u8; 32],
    mask: [u8; 32],
    txid: [u8; 32],
    unlocked: bool,
}

epee_object!(
    OutKey,
    height: u64,
    key: [u8; 32],
    mask: [u8; 32],
    txid: [u8; 32],
    unlocked: bool,
);

#[test]
fn rpc_get_outs_response() {
    let bytes = hex::decode("011101010101020101140763726564697473050000000000000000046f7574738c04140668656967687405a100000000000000036b65790a802d392d0be38eb4699c17767e62a063b8d2f989ec15c80e5d2665ab06f8397439046d61736b0a805e8b863c5b267deda13f4bc5d5ec8e59043028380f2431bc8691c15c83e1fea404747869640a80c0646e065a33b849f0d9563673ca48eb0c603fe721dd982720dba463172c246f08756e6c6f636b65640b00067374617475730a084f4b08746f705f686173680a0009756e747275737465640b00").unwrap();
    let val: GetOutsResponse = from_bytes(&mut bytes.as_slice()).unwrap();
    let mut bytes = to_bytes(val.clone()).unwrap();

    assert_eq!(val, from_bytes(&mut bytes).unwrap());
}

#[test]
fn get_out_indexes_response() {
    let bytes: [u8; 61] = [
        1, 17, 1, 1, 1, 1, 2, 1, 1, 16, 7, 99, 114, 101, 100, 105, 116, 115, 5, 0, 0, 0, 0, 0, 0,
        0, 0, 6, 115, 116, 97, 116, 117, 115, 10, 8, 79, 75, 8, 116, 111, 112, 95, 104, 97, 115,
        104, 10, 0, 9, 117, 110, 116, 114, 117, 115, 116, 101, 100, 11, 0,
    ];
    let val: GetOIndexesResponse = from_bytes(&mut bytes.as_slice()).unwrap();
    let mut bytes = to_bytes(val.clone()).unwrap();

    assert_eq!(val, from_bytes(&mut bytes).unwrap());
}
