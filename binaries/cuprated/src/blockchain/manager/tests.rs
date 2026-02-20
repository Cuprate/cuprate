use std::{collections::HashMap, env::temp_dir, path::PathBuf, sync::Arc};

use monero_oxide::{
    block::{Block, BlockHeader},
    io::CompressedPoint,
    transaction::{Input, Output, Timelock, Transaction, TransactionPrefix},
};
use tokio::sync::{oneshot, watch};
use tower::BoxError;

use cuprate_consensus_context::{BlockchainContext, ContextConfig};
use cuprate_consensus_rules::{hard_forks::HFInfo, miner_tx::calculate_block_reward, HFsInfo};
use cuprate_helper::network::Network;
use cuprate_p2p::{block_downloader::BlockBatch, BroadcastSvc};
use cuprate_p2p_core::handles::HandleBuilder;
use cuprate_types::{CachedVerificationState, TransactionVerificationData, TxVersion};

use crate::{
    blockchain::{
        check_add_genesis, manager::BlockchainManager, manager::BlockchainManagerCommand,
        ConsensusBlockchainReadHandle,
    },
    txpool::TxpoolManagerHandle,
};

async fn mock_manager(data_dir: PathBuf) -> BlockchainManager {
    let blockchain_config = cuprate_blockchain::config::ConfigBuilder::new()
        .data_directory(data_dir.clone())
        .build();
    let txpool_config = cuprate_txpool::config::ConfigBuilder::new()
        .data_directory(data_dir)
        .build();

    let (mut blockchain_read_handle, mut blockchain_write_handle, _) =
        cuprate_blockchain::service::init(blockchain_config).unwrap();
    let (txpool_read_handle, txpool_write_handle, _) =
        cuprate_txpool::service::init(&txpool_config).unwrap();

    check_add_genesis(
        &mut blockchain_read_handle,
        &mut blockchain_write_handle,
        Network::Mainnet,
    )
    .await;

    let mut context_config = ContextConfig::main_net();
    context_config.difficulty_cfg.fixed_difficulty = Some(1);
    context_config.hard_fork_cfg.info = HFsInfo::new([HFInfo::new(0, 0); 16]);

    let blockchain_read_handle =
        ConsensusBlockchainReadHandle::new(blockchain_read_handle, BoxError::from);

    let blockchain_context_service = cuprate_consensus_context::initialize_blockchain_context(
        context_config,
        blockchain_read_handle.clone(),
    )
    .await
    .unwrap();

    BlockchainManager {
        blockchain_write_handle,
        blockchain_read_handle,
        txpool_manager_handle: TxpoolManagerHandle::mock(),
        blockchain_context_service,
        stop_current_block_downloader: Arc::new(Default::default()),
        broadcast_svc: BroadcastSvc::mock(),
    }
}

fn generate_block(context: &BlockchainContext) -> Block {
    Block::new(
        BlockHeader {
            hardfork_version: 16,
            hardfork_signal: 16,
            timestamp: 1000,
            previous: context.top_hash,
            nonce: 0,
        },
        Transaction::V2 {
            prefix: TransactionPrefix {
                additional_timelock: Timelock::Block(context.chain_height + 60),
                inputs: vec![Input::Gen(context.chain_height)],
                outputs: vec![Output {
                    // we can set the block weight to 1 as the true value won't get us into the penalty zone.
                    amount: Some(calculate_block_reward(
                        1,
                        context.median_weight_for_block_reward,
                        context.already_generated_coins,
                        context.current_hf,
                    )),
                    key: CompressedPoint([0; 32]),
                    view_tag: Some(1),
                }],
                extra: rand::random::<[u8; 32]>().to_vec(),
            },
            proofs: None,
        },
        vec![],
    )
    .unwrap()
}

#[tokio::test]
async fn simple_reorg() {
    // create 2 managers
    let data_dir_1 = tempfile::tempdir().unwrap();
    let mut manager_1 = mock_manager(data_dir_1.path().to_path_buf()).await;

    let data_dir_2 = tempfile::tempdir().unwrap();
    let mut manager_2 = mock_manager(data_dir_2.path().to_path_buf()).await;

    // give both managers the same first non-genesis block
    let block_1 = generate_block(manager_1.blockchain_context_service.blockchain_context());

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_1.clone(),
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    manager_2
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_1,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    assert_eq!(
        manager_1.blockchain_context_service.blockchain_context(),
        manager_2.blockchain_context_service.blockchain_context()
    );

    // give managers different 2nd block
    let block_2a = generate_block(manager_1.blockchain_context_service.blockchain_context());
    let block_2b = generate_block(manager_2.blockchain_context_service.blockchain_context());

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_2a,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    manager_2
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_2b.clone(),
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    let manager_1_context = manager_1
        .blockchain_context_service
        .blockchain_context()
        .clone();
    assert_ne!(
        &manager_1_context,
        manager_2.blockchain_context_service.blockchain_context()
    );

    // give manager 1 missing block

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_2b,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();
    // make sure this didn't change the context
    assert_eq!(
        &manager_1_context,
        manager_1.blockchain_context_service.blockchain_context()
    );

    // give both managers new block (built of manager 2's chain)
    let block_3 = generate_block(manager_2.blockchain_context_service.blockchain_context());

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_3.clone(),
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    manager_2
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_3,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    // make sure manager 1 reorged.
    assert_eq!(
        manager_1.blockchain_context_service.blockchain_context(),
        manager_2.blockchain_context_service.blockchain_context()
    );
    assert_eq!(
        manager_1
            .blockchain_context_service
            .blockchain_context()
            .chain_height,
        4
    );
}

/// Same as [`simple_reorg`] but uses block batches instead.
#[tokio::test]
async fn simple_reorg_block_batch() {
    cuprate_fast_sync::set_fast_sync_hashes(&[]);

    let handle = HandleBuilder::new().build();

    // create 2 managers
    let data_dir_1 = tempfile::tempdir().unwrap();
    let mut manager_1 = mock_manager(data_dir_1.path().to_path_buf()).await;

    let data_dir_2 = tempfile::tempdir().unwrap();
    let mut manager_2 = mock_manager(data_dir_2.path().to_path_buf()).await;

    // give both managers the same first non-genesis block
    let block_1 = generate_block(manager_1.blockchain_context_service.blockchain_context());

    manager_1
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_1.clone(), vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();

    manager_2
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_1, vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();

    assert_eq!(
        manager_1.blockchain_context_service.blockchain_context(),
        manager_2.blockchain_context_service.blockchain_context()
    );

    // give managers different 2nd block
    let block_2a = generate_block(manager_1.blockchain_context_service.blockchain_context());
    let block_2b = generate_block(manager_2.blockchain_context_service.blockchain_context());

    manager_1
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_2a, vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();

    manager_2
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_2b.clone(), vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();

    let manager_1_context = manager_1
        .blockchain_context_service
        .blockchain_context()
        .clone();
    assert_ne!(
        &manager_1_context,
        manager_2.blockchain_context_service.blockchain_context()
    );

    // give manager 1 missing block

    manager_1
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_2b, vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();
    // make sure this didn't change the context
    assert_eq!(
        &manager_1_context,
        manager_1.blockchain_context_service.blockchain_context()
    );

    // give both managers new block (built of manager 2's chain)
    let block_3 = generate_block(manager_2.blockchain_context_service.blockchain_context());

    manager_1
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_3.clone(), vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();

    manager_2
        .handle_incoming_block_batch(BlockBatch {
            blocks: vec![(block_3, vec![])],
            size: 0,
            peer_handle: handle.1.clone(),
        })
        .await
        .unwrap();

    // make sure manager 1 reorged.
    assert_eq!(
        manager_1.blockchain_context_service.blockchain_context(),
        manager_2.blockchain_context_service.blockchain_context()
    );
    assert_eq!(
        manager_1
            .blockchain_context_service
            .blockchain_context()
            .chain_height,
        4
    );
}

#[tokio::test]
async fn recover_bad_reorg() {
    let data_dir_1 = tempfile::tempdir().unwrap();
    let mut manager_1 = mock_manager(data_dir_1.path().to_path_buf()).await;

    let context_1 = manager_1
        .blockchain_context_service
        .blockchain_context()
        .clone();

    // start by building the valid chain.
    let block_1 = generate_block(&context_1);

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_1,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    let context_2 = manager_1
        .blockchain_context_service
        .blockchain_context()
        .clone();

    let mut block_2 = generate_block(&context_2);

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_2,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    // Save this context for later to check the reorg gets reversed correctly.
    let context = manager_1
        .blockchain_context_service
        .blockchain_context()
        .clone();

    // start building the alt chain.
    let mut block_1_alt = generate_block(&context_1);

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_1_alt.clone(),
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    // This tx is invalid and will make the reorg fail.
    let tx = Transaction::V2 {
        prefix: TransactionPrefix {
            additional_timelock: Timelock::None,
            inputs: vec![Input::Gen(1)],
            outputs: vec![],
            extra: vec![],
        },
        proofs: None,
    };

    let tx = TransactionVerificationData {
        version: TxVersion::RingSignatures,
        tx_blob: tx.serialize(),
        tx_weight: 0,
        fee: 0,
        tx_hash: tx.hash(),
        cached_verification_state: CachedVerificationState::NotVerified,
        tx,
    };

    let mut block_2_alt = generate_block(&context_2);
    block_2_alt.transactions = vec![tx.tx_hash];
    block_2_alt.header.previous = block_1_alt.hash();

    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_2_alt.clone(),
            prepped_txs: HashMap::from([(tx.tx_hash, tx)]),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    let mut block_3_alt = generate_block(manager_1.blockchain_context_service.blockchain_context());
    block_3_alt.header.previous = block_2_alt.hash();

    // Currently this is the state of the DB:
    // main chain: Genesis, A, B
    // alt chain: Genesis, AAlt, BAlt
    // BAlt is an invalid block, once we pass in this block below (CAlt) the manager will attempt a reorg,
    // this will fail, and we should stay on the main chain.
    manager_1
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_3_alt,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await
        .unwrap();

    // make sure the reorg failed.
    assert_eq!(
        &context,
        manager_1.blockchain_context_service.blockchain_context()
    );
}
