//! Other endpoint types.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::macros::define_monero_rpc_struct;

//---------------------------------------------------------------------------------------------------- TODO
define_monero_rpc_struct! {
    save_bc,
    core_rpc_server_commands_defs.h => 898..=916,
    SaveBc,
    #[derive(Copy)]
    Request {},
    Response {
        status: crate::Status,
        untrusted: bool,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}