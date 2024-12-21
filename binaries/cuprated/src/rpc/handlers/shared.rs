//! RPC handler functions that are shared between different endpoint/methods.
//!
//! TODO:
//! Some handlers have `todo!()`s for other Cuprate internals that must be completed, see:
//! <https://github.com/Cuprate/cuprate/pull/355>

use std::{
    collections::{HashMap, HashSet},
    num::NonZero,
};

use anyhow::{anyhow, Error};
use cuprate_types::OutputDistributionInput;
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
    handlers::helper,
    service::{blockchain, txpool},
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
    txpool::all_hashes(&mut state.txpool_read, include_sensitive_txs).await
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3352-L3398>
///
/// Shared between:
/// - Other JSON's `/get_output_distribution`
/// - Binary's `/get_output_distribution.bin`
///
/// Returns transaction hashes.
pub(super) async fn get_output_distribution(
    mut state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    if state.is_restricted() && request.amounts != [0] {
        return Err(anyhow!(
            "Restricted RPC can only get output distribution for RCT outputs. Use your own node."
        ));
    }

    let input = OutputDistributionInput {
        amounts: request.amounts,
        cumulative: request.cumulative,
        from_height: request.from_height,

        // 0 / `None` is placeholder for the whole chain
        to_height: NonZero::new(request.to_height),
    };

    let distributions = blockchain::output_distribution(&mut state.blockchain_read, input).await?;

    Ok(GetOutputDistributionResponse {
        base: helper::access_response_base(false),
        distributions: todo!(
            "This type contains binary strings: <https://github.com/monero-project/monero/issues/9422>"
        ),
    })
}
