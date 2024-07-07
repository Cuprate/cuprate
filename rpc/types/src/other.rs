//! JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
//!
//! Most (if not all) of these types are defined here:
//! - <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h>

//---------------------------------------------------------------------------------------------------- Import
use crate::{
    base::ResponseBase, defaults::default_bool, macros::define_request_and_response, misc::TxEntry,
};

//---------------------------------------------------------------------------------------------------- TODO
define_request_and_response! {
    get_height,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 138..=160,
    GetHeight,
    Request {},
    ResponseBase {
        hash: String,
        height: u64,
    }
}

define_request_and_response! {
    get_transactions,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 370..=451,
    GetTransactions,
    Request {
        txs_hashes: Vec<String>,
        // FIXME: this is documented as optional but it isn't serialized as an optional
        // but it is set _somewhere_ to false in `monerod`
        // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L382>
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        decode_as_json: bool = default_bool(),
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        prune: bool = default_bool(),
        #[cfg_attr(feature = "serde", serde(default = "default_bool"))]
        split: bool = default_bool(),
    },
    ResponseBase {
        txs_as_hex: Vec<String>,
        txs_as_json: Vec<String>,
        missed_tx: Vec<String>,
        txs: Vec<TxEntry>,
    }
}

define_request_and_response! {
    save_bc,
    cc73fe71162d564ffda8e549b79a350bca53c454 =>
    core_rpc_server_commands_defs.h => 898..=916,
    SaveBc,
    Request {},
    ResponseBase {}
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
