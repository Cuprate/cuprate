//! JSON types from the [`other`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#other-daemon-rpc-calls) endpoints.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::{base::ResponseBase, macros::define_request_and_response};

//---------------------------------------------------------------------------------------------------- TODO
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
