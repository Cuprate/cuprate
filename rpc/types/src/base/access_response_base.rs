//! TODO

//---------------------------------------------------------------------------------------------------- Import
use serde::{Deserialize, Serialize};

use crate::base::ResponseBase;

//---------------------------------------------------------------------------------------------------- ResponseBase
/// TODO
///
/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server_commands_defs.h#L124-L136>.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessResponseBase {
    /// TODO
    #[serde(flatten)]
    pub response_base: ResponseBase,
    /// TODO
    pub credits: u64,
    /// TODO
    pub top_hash: String,
}

cuprate_epee_encoding::epee_object! {
    AccessResponseBase,
    credits: u64,
    top_hash: String,
    !flatten: response_base: ResponseBase,
}

//---------------------------------------------------------------------------------------------------- Tests
#[cfg(test)]
mod test {
    // use super::*;
}
