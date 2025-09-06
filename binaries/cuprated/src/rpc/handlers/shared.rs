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
use monero_oxide::transaction::Timelock;

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
    service::{blockchain, blockchain_context, txpool},
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

    let outputs = blockchain::outputs_vec(
        &mut state.blockchain_read,
        request.outputs,
        request.get_txid,
    )
    .await?;
    let mut outs = Vec::<OutKeyBin>::with_capacity(outputs.len());
    let blockchain_ctx = state.blockchain_context.blockchain_context();

    for (_, index_vec) in outputs {
        for (_, out) in index_vec {
            let out_key = OutKeyBin {
                key: out.key.0,
                mask: out.commitment.0,
                unlocked: cuprate_consensus_rules::transactions::output_unlocked(
                    &out.time_lock,
                    blockchain_ctx.chain_height,
                    blockchain_ctx.current_adjusted_timestamp_for_time_lock(),
                    blockchain_ctx.current_hf,
                ),
                height: usize_to_u64(out.height),
                txid: out.txid.unwrap_or_default(),
            };

            outs.push(out_key);
        }
    }

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

/// Always returns an [`Error`].
///
/// This is a temporary function used for RPC method/endpoints
/// that are not yet ready - it should be removed when all are ready.
pub(super) fn not_available<T>() -> Result<T, Error> {
    Err(anyhow!("Not available"))
}
