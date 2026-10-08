//! RPC handler functions that are shared between different endpoint/methods.
//!
//! TODO:
//! Some handlers have `todo!()`s for other Cuprate internals that must be completed, see:
//! <https://github.com/Cuprate/cuprate/pull/355>

use std::num::NonZero;

use anyhow::{anyhow, Error};
use tower::{Service, ServiceExt};

use cuprate_consensus_context::{BlockChainContextRequest, BlockChainContextResponse};
use cuprate_constants::rpc::MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT;
use cuprate_helper::cast::usize_to_u64;
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    bin::{GetOutsRequest, GetOutsResponse},
    json::{GetOutputDistributionRequest, GetOutputDistributionResponse},
    misc::{Distribution, DistributionCompressedBinary, DistributionUncompressed, OutKeyBin},
};
use cuprate_txpool::service::interface::{TxpoolReadRequest, TxpoolReadResponse};
use cuprate_types::{
    blockchain::{BlockchainReadRequest, BlockchainResponse},
    PreRctOutputDistributionInput,
};

use crate::rpc::{handlers::helper, CupratedRpcHandler};

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

    let outputs = request
        .outputs
        .into_iter()
        .map(|output| (output.amount, output.index))
        .collect();

    let BlockchainResponse::OutputsVec(outputs) = state
        .blockchain_read
        .ready()
        .await?
        .call(BlockchainReadRequest::OutputsVec {
            outputs,
            get_txid: request.get_txid,
        })
        .await?
    else {
        unreachable!();
    };

    let mut outs = Vec::<OutKeyBin>::with_capacity(outputs.len());
    let blockchain_ctx = state.blockchain_context.blockchain_context();

    for (_, index_vec) in outputs {
        for (_, out) in index_vec {
            let out_key = OutKeyBin {
                key: out.key.to_bytes(),
                mask: out.commitment.to_bytes(),
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

    let TxpoolReadResponse::AllHashes(hashes) = state
        .txpool_read
        .ready()
        .await?
        .call(TxpoolReadRequest::AllHashes {
            include_sensitive_txs,
        })
        .await?
    else {
        unreachable!();
    };

    Ok(hashes)
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

    // Pre-RCT amounts are served by the database, the RCT
    // distribution by the context service's cache.
    let pre_rct_amounts: Vec<NonZero<u64>> = request
        .amounts
        .iter()
        .copied()
        .filter_map(NonZero::new)
        .collect();

    // 0 / `None` is placeholder for the whole chain.
    let to_height = match NonZero::new(request.to_height) {
        Some(h) => Some(h),
        None if pre_rct_amounts.is_empty() => None,
        None => {
            let BlockchainResponse::ChainHeight(height, _) = state
                .blockchain_read
                .ready()
                .await?
                .call(BlockchainReadRequest::ChainHeight)
                .await?
            else {
                unreachable!();
            };

            NonZero::new(usize_to_u64(height) - 1)
        }
    };

    let mut pre_rct = if pre_rct_amounts.is_empty() {
        Vec::new()
    } else {
        let BlockchainResponse::PreRctOutputDistribution(data) = state
            .blockchain_read
            .ready()
            .await?
            .call(BlockchainReadRequest::PreRctOutputDistribution(
                PreRctOutputDistributionInput {
                    amounts: pre_rct_amounts,
                    cumulative: request.cumulative,
                    from_height: request.from_height,
                    to_height,
                },
            ))
            .await?
        else {
            unreachable!();
        };

        data
    }
    .into_iter();

    let rct = if request.amounts.contains(&0) {
        let BlockChainContextResponse::RctOutputDistribution(data) = state
            .blockchain_context
            .ready()
            .await
            .map_err(|e| anyhow!(e))?
            .call(BlockChainContextRequest::RctOutputDistribution {
                from_height: request.from_height,
                to_height,
                cumulative: request.cumulative,
            })
            .await
            .map_err(|e| anyhow!(e))?
        else {
            unreachable!();
        };

        Some(data)
    } else {
        None
    };

    let mut distributions = Vec::with_capacity(request.amounts.len());
    for &amount in &request.amounts {
        let data = if amount == 0 {
            rct.as_ref()
                .expect("RCT distribution requested above")
                .clone()
        } else {
            pre_rct.next().expect("one distribution per pre-RCT amount")
        };

        distributions.push(data);
    }

    // TODO: <https://github.com/monero-project/monero/issues/9422>.
    let binary = request.binary;
    let compress = request.compress;

    let distributions = distributions
        .into_iter()
        .map(|data| {
            if binary && compress {
                Distribution::CompressedBinary(DistributionCompressedBinary {
                    start_height: data.start_height,
                    base: data.base,
                    distribution: data.distribution,
                    amount: data.amount,
                })
            } else {
                Distribution::Uncompressed(DistributionUncompressed {
                    start_height: data.start_height,
                    base: data.base,
                    distribution: data.distribution,
                    amount: data.amount,
                    binary,
                })
            }
        })
        .collect();

    Ok(GetOutputDistributionResponse {
        base: helper::access_response_base(false),
        distributions,
    })
}

/// Always returns an [`Error`].
///
/// This is a temporary function used for RPC method/endpoints
/// that are not yet ready - it should be removed when all are ready.
pub(super) fn not_available<T>() -> Result<T, Error> {
    Err(anyhow!("Not available"))
}
