mod batch_handler;

use cuprate_blockchain::service::{BlockchainReadHandle, BlockchainWriteHandle};
use cuprate_consensus::{BlockChainContextService, BlockVerifierService, TxVerifierService};

struct BlockchainManager {
    blockchain_write_handle: BlockchainWriteHandle,
    blockchain_context_service: BlockChainContextService,
    block_verifier_service: BlockVerifierService<
        BlockChainContextService,
        TxVerifierService<BlockchainReadHandle>,
        BlockchainReadHandle,
    >,
}
