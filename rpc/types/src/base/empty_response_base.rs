//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

//---------------------------------------------------------------------------------------------------- ResponseBase
/// TODO
///
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L101-L112>.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct EmptyResponseBase;

cuprate_epee_encoding::epee_object! {
    EmptyResponseBase,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
