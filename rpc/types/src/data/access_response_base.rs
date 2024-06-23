//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

use crate::data::ResponseBase;

//---------------------------------------------------------------------------------------------------- ResponseBase
/// TODO
///
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L124-L136>.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessResponseBase {
    /// TODO
    #[serde(flatten)]
    response_base: ResponseBase,
    /// TODO
    credits: u64,
    /// TODO
    top_hash: String,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
