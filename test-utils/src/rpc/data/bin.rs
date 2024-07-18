//! Binary data from [`.bin` endpoints](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#get_blocksbin).
//!
//! TODO: Not implemented yet.

//---------------------------------------------------------------------------------------------------- Import
use crate::rpc::data::macros::define_request_and_response;

//---------------------------------------------------------------------------------------------------- TODO
define_request_and_response! {
    get_blocksbin,
    GET_BLOCKS: &[u8],
    Request = &[];
    Response = &[];
}

define_request_and_response! {
    get_blocks_by_heightbin,
    GET_BLOCKS_BY_HEIGHT: &[u8],
    Request = &[];
    Response = &[];
}

define_request_and_response! {
    get_hashesbin,
    GET_HASHES: &[u8],
    Request = &[];
    Response = &[];
}

define_request_and_response! {
    get_o_indexesbin,
    GET_O_INDEXES: &[u8],
    Request = &[];
    Response = &[];
}

define_request_and_response! {
    get_outsbin,
    GET_OUTS: &[u8],
    Request = &[];
    Response = &[];
}

define_request_and_response! {
    get_transaction_pool_hashesbin,
    GET_TRANSACTION_POOL_HASHES: &[u8],
    Request = &[];
    Response = &[];
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
