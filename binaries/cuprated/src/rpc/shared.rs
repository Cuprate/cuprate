//! RPC handler functions that are shared between different endpoint/methods.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Error};
use monero_serai::transaction::Timelock;

use cuprate_constants::rpc::MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT;
use cuprate_helper::cast::usize_to_u64;
use cuprate_hex::Hex;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{
        GetOutsRequest, GetOutsResponse, GetTransactionPoolHashesRequest,
        GetTransactionPoolHashesResponse,
    },
    json::{GetOutputDistributionRequest, GetOutputDistributionResponse},
    misc::{Distribution, OutKeyBin},
};

use crate::rpc::{
    helper,
    request::{blockchain, txpool},
    CupratedRpcHandler,
};

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
            index_map.into_values().map(|out| OutKeyBin {
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

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1713-L1739>
///
/// Shared between:
/// - Other JSON's `/get_transaction_pool_hashes`
/// - Binary's `/get_transaction_pool_hashes.bin`
///
/// Returns transaction hashes.
pub(super) async fn get_transaction_pool_hashes(
    mut state: CupratedRpcHandler,
) -> Result<Vec<[u8; 32]>, Error> {
    let include_sensitive_txs = !state.is_restricted();

    // FIXME: this request is a bit overkill, we only need the hashes.
    // We could create a separate request for this.
    Ok(txpool::pool(&mut state.txpool_read, include_sensitive_txs)
        .await?
        .0
        .into_iter()
        .map(|tx| tx.id_hash.0)
        .collect())
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3352-L3398>
///
/// Shared between:
/// - JSON-RPC's `get_output_distribution`
/// - Binary's `/get_output_distribution.bin`
pub(super) async fn get_output_distribution(
    mut state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    if state.is_restricted() && request.amounts != [1, 0] {
        return Err(anyhow!(
            "Restricted RPC can only get output distribution for RCT outputs. Use your own node."
        ));
    }

    // 0 is placeholder for the whole chain
    let req_to_height = if request.to_height == 0 {
        helper::top_height(&mut state).await?.0.saturating_sub(1)
    } else {
        request.to_height
    };

    let distributions = request.amounts.into_iter().map(|amount| {
        fn get_output_distribution() -> Result<Distribution, Error> {
            todo!("https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/src/rpc/rpc_handler.cpp#L29");
            Err(anyhow!("Failed to get output distribution"))
        }

        get_output_distribution()
    }).collect::<Result<Vec<Distribution>, _>>()?;

    Ok(GetOutputDistributionResponse {
        base: helper::access_response_base(false),
        distributions,
    })
}
