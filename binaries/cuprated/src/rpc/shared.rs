//! RPC handler functions that are shared between different endpoint/methods.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Error};
use monero_serai::transaction::Timelock;

use cuprate_constants::rpc::MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT;
use cuprate_helper::cast::usize_to_u64;
use cuprate_hex::Hex;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{GetOutsRequest, GetOutsResponse},
    misc::OutKeyBin,
};

use crate::rpc::{helper, request::blockchain, CupratedRpcHandler};

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L912-L957>
///
/// Shared between:
/// - Other JSON's `/get_outs`
/// - Binary's `/get_outs.bin`
pub(super) async fn get_outs(
    mut state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    if state.is_restricted() && request.outputs.len() > MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT {
        return Err(anyhow!("Too many outs requested"));
    }

    let outputs = {
        let mut outputs = HashMap::<u64, HashSet<u64>>::with_capacity(request.outputs.len());

        for out in request.outputs {
            outputs
                .entry(out.amount)
                .and_modify(|set| {
                    set.insert(out.index);
                })
                .or_insert_with(|| HashSet::from([out.index]));
        }

        outputs
    };

    let outs = blockchain::outputs(&mut state.blockchain_read, outputs)
        .await?
        .into_iter()
        .flat_map(|(amount, index_map)| {
            index_map.into_iter().map(|(_, out)| OutKeyBin {
                key: out.key.map_or([0; 32], |e| e.compress().0),
                mask: out.commitment.compress().0,
                unlocked: matches!(out.time_lock, Timelock::None),
                height: usize_to_u64(out.height),
                txid: if request.get_txid { out.txid } else { [0; 32] },
            })
        })
        .collect::<Vec<OutKeyBin>>();

    Ok(GetOutsResponse {
        base: helper::access_response_base(false),
        outs,
    })
}
