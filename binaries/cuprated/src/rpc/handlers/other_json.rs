//! RPC request handler functions (other JSON endpoints).
//!
//! TODO:
//! Some handlers have `todo!()`s for other Cuprate internals that must be completed, see:
//! <https://github.com/Cuprate/cuprate/pull/355>

use std::{
    borrow::Cow,
    collections::{BTreeSet, HashMap, HashSet},
};

use anyhow::{anyhow, Error};
use monero_oxide::transaction::{Input, Timelock, Transaction};

use cuprate_constants::rpc::{
    MAX_RESTRICTED_GLOBAL_FAKE_OUTS_COUNT, RESTRICTED_SPENT_KEY_IMAGES_COUNT,
    RESTRICTED_TRANSACTIONS_COUNT,
};
use cuprate_dandelion_tower::TxState;
use cuprate_helper::cast::usize_to_u64;
use cuprate_hex::{Hex, HexVec};
use cuprate_p2p_core::{client::handshaker::builder::DummyAddressBook, ClearNet};
use cuprate_rpc_interface::RpcHandler;
use cuprate_rpc_types::{
    base::{AccessResponseBase, ResponseBase},
    misc::{Status, TxEntry, TxEntryType},
    other::{
        GetAltBlocksHashesRequest, GetAltBlocksHashesResponse, GetHeightRequest, GetHeightResponse,
        GetLimitRequest, GetLimitResponse, GetNetStatsRequest, GetNetStatsResponse, GetOutsRequest,
        GetOutsResponse, GetPeerListRequest, GetPeerListResponse, GetPublicNodesRequest,
        GetPublicNodesResponse, GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
        GetTransactionPoolRequest, GetTransactionPoolResponse, GetTransactionPoolStatsRequest,
        GetTransactionPoolStatsResponse, GetTransactionsRequest, GetTransactionsResponse,
        InPeersRequest, InPeersResponse, IsKeyImageSpentRequest, IsKeyImageSpentResponse,
        MiningStatusRequest, MiningStatusResponse, OtherRequest, OtherResponse, OutPeersRequest,
        OutPeersResponse, PopBlocksRequest, PopBlocksResponse, SaveBcRequest, SaveBcResponse,
        SendRawTransactionRequest, SendRawTransactionResponse, SetBootstrapDaemonRequest,
        SetBootstrapDaemonResponse, SetLimitRequest, SetLimitResponse, SetLogCategoriesRequest,
        SetLogCategoriesResponse, SetLogHashRateRequest, SetLogHashRateResponse,
        SetLogLevelRequest, SetLogLevelResponse, StartMiningRequest, StartMiningResponse,
        StopDaemonRequest, StopDaemonResponse, StopMiningRequest, StopMiningResponse,
        UpdateRequest, UpdateResponse,
    },
};
use cuprate_types::{
    rpc::{KeyImageSpentStatus, PoolInfo, PoolTxInfo, PublicNode},
    TxInPool, TxRelayChecks,
};

use crate::{
    logging,
    rpc::{
        constants::UNSUPPORTED_RPC_CALL,
        handlers::{
            helper,
            shared::{self, not_available},
        },
        service::{
            address_book, blockchain, blockchain_context, blockchain_manager, tx_handler, txpool,
        },
        CupratedRpcHandler,
    },
    statics::START_INSTANT_UNIX,
    txpool::IncomingTxs,
};

/// Map a [`OtherRequest`] to the function that will lead to a [`OtherResponse`].
pub async fn map_request(
    state: CupratedRpcHandler,
    request: OtherRequest,
) -> Result<OtherResponse, Error> {
    use OtherRequest as Req;
    use OtherResponse as Resp;

    Ok(match request {
        Req::GetHeight(r) => Resp::GetHeight(get_height(state, r).await?),
        Req::GetTransactions(r) => Resp::GetTransactions(not_available()?),
        Req::GetAltBlocksHashes(r) => Resp::GetAltBlocksHashes(not_available()?),
        Req::IsKeyImageSpent(r) => Resp::IsKeyImageSpent(not_available()?),
        Req::SendRawTransaction(r) => {
            Resp::SendRawTransaction(send_raw_transaction(state, r).await?)
        }
        Req::SaveBc(r) => Resp::SaveBc(not_available()?),
        Req::GetPeerList(r) => Resp::GetPeerList(not_available()?),
        Req::SetLogLevel(r) => Resp::SetLogLevel(set_log_level(state, r).await?),
        Req::SetLogCategories(r) => Resp::SetLogCategories(set_log_categories(state, r).await?),
        Req::GetTransactionPool(r) => Resp::GetTransactionPool(not_available()?),
        Req::GetTransactionPoolStats(r) => Resp::GetTransactionPoolStats(not_available()?),
        Req::StopDaemon(r) => Resp::StopDaemon(not_available()?),
        Req::GetLimit(r) => Resp::GetLimit(not_available()?),
        Req::SetLimit(r) => Resp::SetLimit(not_available()?),
        Req::OutPeers(r) => Resp::OutPeers(not_available()?),
        Req::InPeers(r) => Resp::InPeers(not_available()?),
        Req::GetNetStats(r) => Resp::GetNetStats(not_available()?),
        Req::GetOuts(r) => Resp::GetOuts(not_available()?),
        Req::PopBlocks(r) => Resp::PopBlocks(not_available()?),
        Req::GetTransactionPoolHashes(r) => Resp::GetTransactionPoolHashes(not_available()?),
        Req::GetPublicNodes(r) => Resp::GetPublicNodes(not_available()?),

        // Unsupported requests.
        Req::SetBootstrapDaemon(_)
        | Req::Update(_)
        | Req::StartMining(_)
        | Req::StopMining(_)
        | Req::MiningStatus(_)
        | Req::SetLogHashRate(_) => return Err(anyhow!(UNSUPPORTED_RPC_CALL)),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L486-L499>
async fn get_height(
    mut state: CupratedRpcHandler,
    _: GetHeightRequest,
) -> Result<GetHeightResponse, Error> {
    let (height, hash) = helper::top_height(&mut state).await?;
    let hash = Hex(hash);

    Ok(GetHeightResponse {
        base: helper::response_base(false),
        height,
        hash,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L979-L1227>
async fn get_transactions(
    mut state: CupratedRpcHandler,
    request: GetTransactionsRequest,
) -> Result<GetTransactionsResponse, Error> {
    if state.is_restricted() && request.txs_hashes.len() > RESTRICTED_TRANSACTIONS_COUNT {
        return Err(anyhow!(
            "Too many transactions requested in restricted mode"
        ));
    }

    let (txs_in_blockchain, missed_txs) = {
        let requested_txs = request.txs_hashes.into_iter().map(|tx| tx.0).collect();
        blockchain::transactions(&mut state.blockchain_read, requested_txs).await?
    };

    let missed_tx = missed_txs.clone().into_iter().map(Hex).collect();

    // Check the txpool for missed transactions.
    let txs_in_pool = if missed_txs.is_empty() {
        vec![]
    } else {
        let include_sensitive_txs = !state.is_restricted();
        txpool::txs_by_hash(&mut state.txpool_read, missed_txs, include_sensitive_txs).await?
    };

    let (txs, txs_as_hex, txs_as_json) = {
        // Prepare the final JSON output.
        let len = txs_in_blockchain.len() + txs_in_pool.len();
        let mut txs = Vec::with_capacity(len);
        let mut txs_as_hex = Vec::with_capacity(len);
        let mut txs_as_json = Vec::with_capacity(if request.decode_as_json { len } else { 0 });

        // Map all blockchain transactions.
        for tx in txs_in_blockchain {
            let tx_hash = Hex(tx.tx_hash);
            let prunable_hash = Hex(tx.prunable_hash);

            let (pruned_as_hex, prunable_as_hex) = if tx.pruned_blob.is_empty() {
                (HexVec::new(), HexVec::new())
            } else {
                (HexVec(tx.pruned_blob), HexVec(tx.prunable_blob))
            };

            let as_hex = if pruned_as_hex.is_empty() {
                // `monerod` will insert a `""` into the `txs_as_hex` array for pruned transactions.
                // curl http://127.0.0.1:18081/get_transactions -d '{"txs_hashes":["4c8b98753d1577d225a497a50f453827cff3aa023a4add60ec4ce4f923f75de8"]}' -H 'Content-Type: application/json'
                HexVec::new()
            } else {
                HexVec(tx.tx_blob)
            };

            txs_as_hex.push(as_hex.clone());

            let as_json = if request.decode_as_json {
                let tx = Transaction::read(&mut as_hex.as_slice())?;
                let json_type = cuprate_types::json::tx::Transaction::from(tx);
                let json = serde_json::to_string(&json_type).unwrap();
                txs_as_json.push(json.clone());
                json
            } else {
                String::new()
            };

            let tx_entry_type = TxEntryType::Blockchain {
                block_height: tx.block_height,
                block_timestamp: tx.block_timestamp,
                confirmations: tx.confirmations,
                output_indices: tx.output_indices,
                in_pool: false,
            };

            let tx = TxEntry {
                as_hex,
                as_json,
                double_spend_seen: false,
                tx_hash,
                prunable_as_hex,
                prunable_hash,
                pruned_as_hex,
                tx_entry_type,
            };

            txs.push(tx);
        }

        // Map all txpool transactions.
        for tx_in_pool in txs_in_pool {
            let TxInPool {
                tx_blob,
                tx_hash,
                double_spend_seen,
                received_timestamp,
                relayed,
            } = tx_in_pool;

            let tx_hash = Hex(tx_hash);
            let tx = Transaction::read(&mut tx_blob.as_slice())?;

            let pruned_as_hex = HexVec::new();
            let prunable_as_hex = HexVec::new();
            let prunable_hash = Hex([0; 32]);

            let as_hex = HexVec(tx_blob);
            txs_as_hex.push(as_hex.clone());

            let as_json = if request.decode_as_json {
                let json_type = cuprate_types::json::tx::Transaction::from(tx);
                let json = serde_json::to_string(&json_type).unwrap();
                txs_as_json.push(json.clone());
                json
            } else {
                String::new()
            };

            let tx_entry_type = TxEntryType::Pool {
                relayed,
                received_timestamp,
                in_pool: true,
            };

            let tx = TxEntry {
                as_hex,
                as_json,
                double_spend_seen,
                tx_hash,
                prunable_as_hex,
                prunable_hash,
                pruned_as_hex,
                tx_entry_type,
            };

            txs.push(tx);
        }

        (txs, txs_as_hex, txs_as_json)
    };

    Ok(GetTransactionsResponse {
        base: helper::access_response_base(false),
        txs_as_hex,
        txs_as_json,
        missed_tx,
        txs,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L790-L815>
async fn get_alt_blocks_hashes(
    mut state: CupratedRpcHandler,
    _: GetAltBlocksHashesRequest,
) -> Result<GetAltBlocksHashesResponse, Error> {
    let blks_hashes = blockchain::alt_chains(&mut state.blockchain_read)
        .await?
        .into_iter()
        .map(|info| Hex(info.block_hash))
        .collect();

    Ok(GetAltBlocksHashesResponse {
        base: helper::access_response_base(false),
        blks_hashes,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1229-L1305>
async fn is_key_image_spent(
    mut state: CupratedRpcHandler,
    request: IsKeyImageSpentRequest,
) -> Result<IsKeyImageSpentResponse, Error> {
    let restricted = state.is_restricted();

    if restricted && request.key_images.len() > RESTRICTED_SPENT_KEY_IMAGES_COUNT {
        return Err(anyhow!("Too many key images queried in restricted mode"));
    }

    let key_images = request
        .key_images
        .into_iter()
        .map(|k| k.0)
        .collect::<Vec<[u8; 32]>>();

    let mut spent_status = Vec::with_capacity(key_images.len());

    // Check the blockchain for key image spend status.
    blockchain::key_images_spent_vec(&mut state.blockchain_read, key_images.clone())
        .await?
        .into_iter()
        .for_each(|ki| {
            if ki {
                spent_status.push(KeyImageSpentStatus::SpentInBlockchain);
            } else {
                spent_status.push(KeyImageSpentStatus::Unspent);
            }
        });

    assert_eq!(spent_status.len(), key_images.len(), "key_images_spent() should be returning a Vec with an equal length to the input, the below zip() relies on this.");

    // Filter the remaining unspent key images out from the vector.
    let key_images = key_images
        .into_iter()
        .zip(&spent_status)
        .filter_map(|(ki, status)| match status {
            KeyImageSpentStatus::Unspent => Some(ki),
            KeyImageSpentStatus::SpentInBlockchain => None,
            KeyImageSpentStatus::SpentInPool => unreachable!(),
        })
        .collect::<Vec<[u8; 32]>>();

    // Check if the remaining unspent key images exist in the transaction pool.
    if !key_images.is_empty() {
        txpool::key_images_spent_vec(&mut state.txpool_read, key_images, !restricted)
            .await?
            .into_iter()
            .for_each(|ki| {
                if ki {
                    spent_status.push(KeyImageSpentStatus::SpentInPool);
                } else {
                    spent_status.push(KeyImageSpentStatus::Unspent);
                }
            });
    }

    let spent_status = spent_status
        .into_iter()
        .map(KeyImageSpentStatus::to_u8)
        .collect();

    Ok(IsKeyImageSpentResponse {
        base: helper::access_response_base(false),
        spent_status,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1307-L1411>
async fn send_raw_transaction(
    mut state: CupratedRpcHandler,
    request: SendRawTransactionRequest,
) -> Result<SendRawTransactionResponse, Error> {
    let mut resp = SendRawTransactionResponse {
        base: helper::access_response_base(false),
        double_spend: false,
        fee_too_low: false,
        invalid_input: false,
        invalid_output: false,
        low_mixin: false,
        nonzero_unlock_time: false,
        not_relayed: request.do_not_relay,
        overspend: false,
        reason: String::new(),
        sanity_check_failed: false,
        too_big: false,
        too_few_outputs: false,
        tx_extra_too_big: false,
    };

    let tx = Transaction::read(&mut request.tx_as_hex.as_slice())?;

    if request.do_sanity_checks {
        /// FIXME: these checks could be defined elsewhere.
        ///
        /// <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/src/cryptonote_core/tx_sanity_check.cpp#L42>
        fn tx_sanity_check(tx: &Transaction, rct_outs_available: u64) -> Result<(), String> {
            let Some(input) = tx.prefix().inputs.first() else {
                return Err("No inputs".to_string());
            };

            let mut rct_indices = vec![];
            let mut n_indices: usize = 0;

            for input in &tx.prefix().inputs {
                match input {
                    Input::Gen(_) => return Err("Transaction is coinbase".to_string()),
                    Input::ToKey {
                        amount,
                        key_offsets,
                        key_image,
                    } => {
                        let Some(amount) = amount else {
                            continue;
                        };

                        /// <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/src/cryptonote_basic/cryptonote_format_utils.cpp#L1526>
                        fn relative_output_offsets_to_absolute(mut offsets: Vec<u64>) -> Vec<u64> {
                            assert!(!offsets.is_empty());

                            for i in 1..offsets.len() {
                                offsets[i] += offsets[i - 1];
                            }

                            offsets
                        }

                        n_indices += key_offsets.len();
                        let absolute = relative_output_offsets_to_absolute(key_offsets.clone());
                        rct_indices.extend(absolute);
                    }
                }
            }

            if n_indices <= 10 {
                return Ok(());
            }

            if rct_outs_available < 10_000 {
                return Ok(());
            }

            let rct_indices_len = rct_indices.len();
            if rct_indices_len < n_indices * 8 / 10 {
                return Err(format!("amount of unique indices is too low (amount of rct indices is {rct_indices_len} out of total {n_indices} indices."));
            }

            let median = cuprate_helper::num::median(rct_indices);
            if median < rct_outs_available * 6 / 10 {
                return Err(format!("median offset index is too low (median is {median} out of total {rct_outs_available} offsets). Transactions should contain a higher fraction of recent outputs."));
            }

            Ok(())
        }

        let rct_outs_available = blockchain::total_rct_outputs(&mut state.blockchain_read).await?;

        if let Err(e) = tx_sanity_check(&tx, rct_outs_available) {
            resp.base.response_base.status = Status::Failed;
            resp.reason.push_str(&format!("Sanity check failed: {e}"));
            resp.sanity_check_failed = true;
            return Ok(resp);
        }
    }

    if state.is_restricted() && request.do_not_relay {
        // FIXME: implement something like `/check_tx` in `cuprated/monerod`.
        // boog900:
        // > making nodes hold txs in their pool that don't get passed
        // > around the network can cause issues, like targeted tx pool double spends
        // > there is also no reason to have this for public RPC
        return Err(anyhow!("do_not_relay is not supported on restricted RPC"));
    }

    let txs = vec![tx.serialize().into()];

    let mut txs = IncomingTxs {
        txs,
        state: TxState::Local,
        drop_relay_rule_errors: false,
        do_not_relay: request.do_not_relay,
    };

    let tx_relay_checks = tx_handler::handle_incoming_txs(&mut state.tx_handler, txs).await?;

    if tx_relay_checks.is_empty() {
        return Ok(resp);
    }

    resp.not_relayed = true;

    // <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L124>
    fn add_reason(reasons: &mut String, reason: &'static str) {
        if !reasons.is_empty() {
            reasons.push_str(", ");
        }
        reasons.push_str(reason);
    }

    let mut reasons = String::new();

    #[rustfmt::skip]
    let array = [
        (&mut resp.double_spend, TxRelayChecks::DOUBLE_SPEND, "double spend"),
        (&mut resp.fee_too_low, TxRelayChecks::FEE_TOO_LOW, "fee too low"),
        (&mut resp.invalid_input, TxRelayChecks::INVALID_INPUT, "invalid input"),
        (&mut resp.invalid_output, TxRelayChecks::INVALID_OUTPUT, "invalid output"),
        (&mut resp.low_mixin, TxRelayChecks::LOW_MIXIN, "bad ring size"),
        (&mut resp.nonzero_unlock_time, TxRelayChecks::NONZERO_UNLOCK_TIME, "tx unlock time is not zero"),
        (&mut resp.overspend, TxRelayChecks::OVERSPEND, "overspend"),
        (&mut resp.too_big, TxRelayChecks::TOO_BIG, "too big"),
        (&mut resp.too_few_outputs, TxRelayChecks::TOO_FEW_OUTPUTS, "too few outputs"),
        (&mut resp.tx_extra_too_big, TxRelayChecks::TX_EXTRA_TOO_BIG, "tx-extra too big"),
    ];

    for (field, flag, reason) in array {
        if tx_relay_checks.contains(flag) {
            *field = true;
            add_reason(&mut reasons, reason);
        }
    }

    Ok(resp)
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1525-L1535>
async fn save_bc(mut state: CupratedRpcHandler, _: SaveBcRequest) -> Result<SaveBcResponse, Error> {
    blockchain_manager::sync(todo!()).await?;

    Ok(SaveBcResponse {
        base: ResponseBase::OK,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1537-L1582>
async fn get_peer_list(
    mut state: CupratedRpcHandler,
    request: GetPeerListRequest,
) -> Result<GetPeerListResponse, Error> {
    let (white_list, gray_list) = address_book::peerlist::<ClearNet>(&mut DummyAddressBook).await?;

    Ok(GetPeerListResponse {
        base: helper::response_base(false),
        white_list,
        gray_list,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1663-L1687>
async fn get_transaction_pool(
    mut state: CupratedRpcHandler,
    _: GetTransactionPoolRequest,
) -> Result<GetTransactionPoolResponse, Error> {
    let include_sensitive_txs = !state.is_restricted();

    let (transactions, spent_key_images) =
        txpool::pool(&mut state.txpool_read, include_sensitive_txs).await?;

    Ok(GetTransactionPoolResponse {
        base: helper::access_response_base(false),
        transactions,
        spent_key_images,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1741-L1756>
async fn get_transaction_pool_stats(
    mut state: CupratedRpcHandler,
    _: GetTransactionPoolStatsRequest,
) -> Result<GetTransactionPoolStatsResponse, Error> {
    let include_sensitive_txs = !state.is_restricted();

    let pool_stats = txpool::pool_stats(&mut state.txpool_read, include_sensitive_txs).await?;

    Ok(GetTransactionPoolStatsResponse {
        base: helper::access_response_base(false),
        pool_stats,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1780-L1788>
async fn stop_daemon(
    mut state: CupratedRpcHandler,
    _: StopDaemonRequest,
) -> Result<StopDaemonResponse, Error> {
    blockchain_manager::stop(todo!()).await?;
    Ok(StopDaemonResponse { status: Status::Ok })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3066-L3077>
async fn get_limit(
    mut state: CupratedRpcHandler,
    _: GetLimitRequest,
) -> Result<GetLimitResponse, Error> {
    todo!("waiting on p2p service");

    Ok(GetLimitResponse {
        base: helper::response_base(false),
        limit_down: todo!(),
        limit_up: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3079-L3117>
async fn set_limit(
    mut state: CupratedRpcHandler,
    request: SetLimitRequest,
) -> Result<SetLimitResponse, Error> {
    todo!("waiting on p2p service");

    Ok(SetLimitResponse {
        base: helper::response_base(false),
        limit_down: todo!(),
        limit_up: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3119-L3127>
async fn out_peers(
    mut state: CupratedRpcHandler,
    request: OutPeersRequest,
) -> Result<OutPeersResponse, Error> {
    todo!("waiting on p2p service");

    Ok(OutPeersResponse {
        base: helper::response_base(false),
        out_peers: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3129-L3137>
async fn in_peers(
    mut state: CupratedRpcHandler,
    request: InPeersRequest,
) -> Result<InPeersResponse, Error> {
    todo!("waiting on p2p service");

    Ok(InPeersResponse {
        base: helper::response_base(false),
        in_peers: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L584-L599>
async fn get_net_stats(
    mut state: CupratedRpcHandler,
    _: GetNetStatsRequest,
) -> Result<GetNetStatsResponse, Error> {
    todo!("waiting on p2p service");

    Ok(GetNetStatsResponse {
        base: helper::response_base(false),
        start_time: *START_INSTANT_UNIX,
        total_packets_in: todo!(),
        total_bytes_in: todo!(),
        total_packets_out: todo!(),
        total_bytes_out: todo!(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L912-L957>
async fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    let outs = shared::get_outs(
        state,
        cuprate_rpc_types::bin::GetOutsRequest {
            outputs: request.outputs,
            get_txid: request.get_txid,
        },
    )
    .await?
    .outs
    .into_iter()
    .map(Into::into)
    .collect();

    Ok(GetOutsResponse {
        base: helper::response_base(false),
        outs,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3242-L3252>
async fn pop_blocks(
    mut state: CupratedRpcHandler,
    request: PopBlocksRequest,
) -> Result<PopBlocksResponse, Error> {
    let height = blockchain_manager::pop_blocks(todo!(), request.nblocks).await?;

    Ok(PopBlocksResponse {
        base: helper::response_base(false),
        height,
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1713-L1739>
async fn get_transaction_pool_hashes(
    mut state: CupratedRpcHandler,
    _: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    Ok(GetTransactionPoolHashesResponse {
        base: helper::response_base(false),
        tx_hashes: shared::get_transaction_pool_hashes(state)
            .await?
            .into_iter()
            .map(Hex)
            .collect(),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L193-L225>
async fn get_public_nodes(
    mut state: CupratedRpcHandler,
    request: GetPublicNodesRequest,
) -> Result<GetPublicNodesResponse, Error> {
    let (white, gray) = address_book::peerlist::<ClearNet>(&mut DummyAddressBook).await?;

    fn map(peers: Vec<cuprate_types::rpc::Peer>) -> Vec<PublicNode> {
        peers
            .into_iter()
            .map(|peer| {
                let cuprate_types::rpc::Peer {
                    host,
                    rpc_port,
                    rpc_credits_per_hash,
                    last_seen,
                    ..
                } = peer;

                PublicNode {
                    host,
                    rpc_port,
                    rpc_credits_per_hash,
                    last_seen,
                }
            })
            .collect()
    }

    let white = map(white);
    let gray = map(gray);

    Ok(GetPublicNodesResponse {
        base: helper::response_base(false),
        white,
        gray,
    })
}

//---------------------------------------------------------------------------------------------------- Unsupported RPC calls (for now)

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1758-L1778>
async fn set_bootstrap_daemon(
    state: CupratedRpcHandler,
    request: SetBootstrapDaemonRequest,
) -> Result<SetBootstrapDaemonResponse, Error> {
    todo!();
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L3139-L3240>
async fn update(
    state: CupratedRpcHandler,
    request: UpdateRequest,
) -> Result<UpdateResponse, Error> {
    todo!();
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1641-L1652>
async fn set_log_level(
    _state: CupratedRpcHandler,
    request: SetLogLevelRequest,
) -> Result<SetLogLevelResponse, Error> {
    use tracing::level_filters::LevelFilter;

    let level = match request.level {
        0 => LevelFilter::ERROR,
        1 => LevelFilter::WARN,
        2 => LevelFilter::INFO,
        3 => LevelFilter::DEBUG,
        4 => LevelFilter::TRACE,
        _ => {
            return Err(anyhow!(
                "Invalid log level: {}. Valid range is 0-4",
                request.level
            ))
        }
    };

    logging::modify_stdout_output(|filter| {
        filter.level = level;
    });
    logging::modify_file_output(|filter| {
        filter.level = level;
    });

    Ok(SetLogLevelResponse {
        base: helper::response_base(false),
    })
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1654-L1661>
async fn set_log_categories(
    _state: CupratedRpcHandler,
    request: SetLogCategoriesRequest,
) -> Result<SetLogCategoriesResponse, Error> {
    use tracing::level_filters::LevelFilter;
    use tracing_subscriber::filter::EnvFilter;

    let categories = if request.categories.is_empty() {
        "*:INFO".to_string()
    } else {
        request.categories.clone()
    };

    let level_filter = if let Ok(_env_filter) = EnvFilter::try_new(&categories) {
        if categories.starts_with("*:") {
            let level_str = categories.strip_prefix("*:").unwrap_or("INFO");
            level_str
                .parse::<LevelFilter>()
                .unwrap_or(LevelFilter::INFO)
        } else {
            LevelFilter::INFO
        }
    } else {
        categories
            .parse::<LevelFilter>()
            .unwrap_or(LevelFilter::INFO)
    };

    logging::modify_stdout_output(|filter| {
        filter.level = level_filter;
    });
    logging::modify_file_output(|filter| {
        filter.level = level_filter;
    });

    Ok(SetLogCategoriesResponse {
        base: helper::response_base(false),
        categories,
    })
}

//---------------------------------------------------------------------------------------------------- Unsupported RPC calls (forever)

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1413-L1462>
async fn start_mining(
    state: CupratedRpcHandler,
    request: StartMiningRequest,
) -> Result<StartMiningResponse, Error> {
    unreachable!()
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1464-L1482>
async fn stop_mining(
    state: CupratedRpcHandler,
    request: StopMiningRequest,
) -> Result<StopMiningResponse, Error> {
    unreachable!();
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1484-L1523>
async fn mining_status(
    state: CupratedRpcHandler,
    request: MiningStatusRequest,
) -> Result<MiningStatusResponse, Error> {
    unreachable!();
}

/// <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/rpc/core_rpc_server.cpp#L1626-L1639>
async fn set_log_hash_rate(
    state: CupratedRpcHandler,
    request: SetLogHashRateRequest,
) -> Result<SetLogHashRateResponse, Error> {
    unreachable!();
}
