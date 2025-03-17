use std::{collections::HashMap, env::temp_dir, path::PathBuf, sync::Arc};

use monero_serai::{
    block::{Block, BlockHeader},
    transaction::{Input, Output, Timelock, Transaction, TransactionPrefix},
};
use tokio::sync::{oneshot, watch};
use tower::BoxError;

use cuprate_consensus_context::{BlockchainContext, ContextConfig};
use cuprate_consensus_rules::{hard_forks::HFInfo, miner_tx::calculate_block_reward, HFsInfo};
use cuprate_helper::network::Network;
use cuprate_p2p::BroadcastSvc;

use crate::blockchain::{
    check_add_genesis, manager::BlockchainManager, manager::BlockchainManagerCommand,
    ConsensusBlockchainReadHandle,
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
        cuprate_txpool::service::init(txpool_config).unwrap();

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
        txpool_write_handle,
        blockchain_context_service,
        stop_current_block_downloader: Arc::new(Default::default()),
        broadcast_svc: BroadcastSvc::mock(),
    }
}

fn generate_block(context: &BlockchainContext) -> Block {
    Block {
        header: BlockHeader {
            hardfork_version: 16,
            hardfork_signal: 16,
            timestamp: 1000,
            previous: context.top_hash,
            nonce: 0,
        },
        miner_transaction: Transaction::V2 {
            prefix: TransactionPrefix {
                additional_timelock: Timelock::Block(context.chain_height + 60),
                inputs: vec![Input::Gen(context.chain_height)],
                outputs: vec![Output {
                    // we can set the block weight to 1 as even the true value won't get us into te penalty zone.
                    amount: Some(calculate_block_reward(
                        1,
                        context.median_weight_for_block_reward,
                        context.already_generated_coins,
                        context.current_hf,
                    )),
                    key: Default::default(),
                    view_tag: Some(1),
                }],
                extra: rand::random::<[u8; 32]>().to_vec(),
            },
            proofs: None,
        },
        transactions: vec![],
    }
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
        .await;

    manager_2
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_1,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await;

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
        .await;

    manager_2
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_2b.clone(),
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await;

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
        .await;
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
        .await;

    manager_2
        .handle_command(BlockchainManagerCommand::AddBlock {
            block: block_3,
            prepped_txs: HashMap::new(),
            response_tx: oneshot::channel().0,
        })
        .await;

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
