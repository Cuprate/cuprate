//! Other endpoint types.
//!
//! <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/daemon_messages.h>.

//---------------------------------------------------------------------------------------------------- Import
use crate::macros::define_monero_rpc_struct;

//---------------------------------------------------------------------------------------------------- TODO
define_monero_rpc_struct! {
    get_height,
    daemon_messages.h => 81..=87,
    GetHeight { height: 123 } => r#"{"height":123}"#,
    GetHeight {
        /// A block's height.
        height: u64,
    }
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
