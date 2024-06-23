//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

use crate::Status;

//---------------------------------------------------------------------------------------------------- ResponseBase
/// TODO
///
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L101-L112>.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct ResponseBase {
    /// TODO
    status: Status,
    /// TODO
    untrusted: bool,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
