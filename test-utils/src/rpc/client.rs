//! HTTP RPC client.

//---------------------------------------------------------------------------------------------------- Use
use monero_oxide::block::Block;
use monero_rpc::Rpc;
use monero_simple_request_rpc::SimpleRequestRpc;
use serde::Deserialize;
use serde_json::json;
use tokio::task::spawn_blocking;

use cuprate_helper::tx::tx_fee;
use cuprate_types::{VerifiedBlockInformation, VerifiedTransactionInformation};

//---------------------------------------------------------------------------------------------------- Constants
/// The default URL used for Monero RPC connections.
pub const LOCALHOST_RPC_URL: &str = "http://127.0.0.1:18081";

//---------------------------------------------------------------------------------------------------- HttpRpcClient
/// An HTTP RPC client for Monero.
pub struct HttpRpcClient {
    address: String,
    rpc: SimpleRequestRpc,
}

impl HttpRpcClient {
    /// Create an [`HttpRpcClient`].
    ///
    /// `address` should be an HTTP URL pointing to a `monerod`.
    ///
    /// If `None` is provided the default is used: [`LOCALHOST_RPC_URL`].
    ///
    /// Note that for [`Self::get_verified_block_information`] to work, the `monerod`
    /// must be in unrestricted mode such that some fields (e.g. `pow_hash`) appear
    /// in the JSON response.
    ///
    /// # Panics
    /// This panics if the `address` is invalid or a connection could not be made.
    pub async fn new(address: Option<String>) -> Self {
        let address = address.unwrap_or_else(|| LOCALHOST_RPC_URL.to_string());

        Self {
            rpc: SimpleRequestRpc::new(address.clone()).await.unwrap(),
            address,
        }
    }

    /// The address used for this [`HttpRpcClient`].
    #[allow(clippy::allow_attributes, dead_code, reason = "expect doesn't work")]
    const fn address(&self) -> &String {
        &self.address
    }

    /// Access to the inner RPC client for other usage.
    #[expect(dead_code)]
    const fn rpc(&self) -> &SimpleRequestRpc {
        &self.rpc
    }

    /// Request data and map the response to a [`VerifiedBlockInformation`].
    ///
    /// # Panics
    /// This function will panic at any error point, e.g.,
    /// if the node cannot be connected to, if deserialization fails, etc.
    pub async fn get_verified_block_information(&self, height: usize) -> VerifiedBlockInformation {
        #[derive(Debug, Deserialize)]
        struct Result {
            blob: String,
            block_header: BlockHeader,
        }

        #[derive(Debug, Deserialize)]
        struct BlockHeader {
            block_weight: usize,
            long_term_weight: usize,
            cumulative_difficulty: u128,
            hash: String,
            height: usize,
            pow_hash: String,
            reward: u64, // generated_coins + total_tx_fees
        }

        let result = self
            .rpc
            .json_rpc_call::<Result>(
                "get_block",
                Some(json!(
                    {
                        "height": height,
                        "fill_pow_hash": true
                    }
                )),
            )
            .await
            .unwrap();

        // Make sure this is a trusted, `pow_hash` only works there.
        assert!(
        	!result.block_header.pow_hash.is_empty(),
        	"untrusted node detected, `pow_hash` will not show on these nodes - use a trusted node!"
        );

        let reward = result.block_header.reward;

        let (block_hash, block_blob, block) = spawn_blocking(|| {
            let block_blob = hex::decode(result.blob).unwrap();
            let block = Block::read(&mut block_blob.as_slice()).unwrap();
            (block.hash(), block_blob, block)
        })
        .await
        .unwrap();

        let txs: Vec<VerifiedTransactionInformation> = self
            .get_transaction_verification_data(&block.transactions)
            .await
            .collect();

        let block_header = result.block_header;
        let block_hash_2 = <[u8; 32]>::try_from(hex::decode(&block_header.hash).unwrap()).unwrap();
        let pow_hash = <[u8; 32]>::try_from(hex::decode(&block_header.pow_hash).unwrap()).unwrap();

        // Assert the block hash matches.
        assert_eq!(block_hash, block_hash_2);

        let total_tx_fees = txs.iter().map(|tx| tx.fee).sum::<u64>();
        let generated_coins = block
            .miner_transaction
            .prefix()
            .outputs
            .iter()
            .map(|output| output.amount.expect("miner_tx amount was None"))
            .sum::<u64>()
            - total_tx_fees;
        assert_eq!(
            reward,
            generated_coins + total_tx_fees,
            "generated_coins ({generated_coins}) + total_tx_fees ({total_tx_fees}) != reward ({reward})"
        );

        VerifiedBlockInformation {
            block,
            block_blob,
            txs,
            block_hash,
            pow_hash,
            generated_coins,
            height: block_header.height,
            weight: block_header.block_weight,
            long_term_weight: block_header.long_term_weight,
            cumulative_difficulty: block_header.cumulative_difficulty,
        }
    }

    /// Request data and map the response to a [`VerifiedTransactionInformation`].
    ///
    /// # Panics
    /// This function will panic at any error point, e.g.,
    /// if the node cannot be connected to, if deserialization fails, etc.
    pub async fn get_transaction_verification_data<'a>(
        &self,
        tx_hashes: &'a [[u8; 32]],
    ) -> impl Iterator<Item = VerifiedTransactionInformation> + 'a {
        self.rpc
            .get_transactions(tx_hashes)
            .await
            .unwrap()
            .into_iter()
            .enumerate()
            .map(|(i, tx)| {
                let tx_hash = tx.hash();
                assert_eq!(tx_hash, tx_hashes[i]);
                VerifiedTransactionInformation {
                    tx_blob: tx.serialize(),
                    tx_weight: tx.weight(),
                    tx_hash,
                    fee: tx_fee(&tx),
                    tx,
                }
            })
    }
}

//---------------------------------------------------------------------------------------------------- TESTS
#[cfg(test)]
mod tests {
    use hex_literal::hex;

    use super::*;

    /// Assert the default address is localhost.
    #[tokio::test]
    async fn localhost() {
        assert_eq!(HttpRpcClient::new(None).await.address(), LOCALHOST_RPC_URL);
    }

    /// Assert blocks are correctly received/calculated.
    #[ignore] // FIXME: doesn't work in CI, we need a real unrestricted node
    #[tokio::test]
    async fn get() {
        #[expect(clippy::too_many_arguments)]
        async fn assert_eq(
            rpc: &HttpRpcClient,
            height: usize,
            block_hash: [u8; 32],
            pow_hash: [u8; 32],
            generated_coins: u64,
            weight: usize,
            long_term_weight: usize,
            cumulative_difficulty: u128,
            tx_count: usize,
        ) {
            let block = rpc.get_verified_block_information(height).await;

            println!("block height: {height}");
            assert_eq!(block.txs.len(), tx_count);
            println!("{block:#?}");

            assert_eq!(block.block_hash, block_hash);
            assert_eq!(block.pow_hash, pow_hash);
            assert_eq!(block.height, height);
            assert_eq!(block.generated_coins, generated_coins);
            assert_eq!(block.weight, weight);
            assert_eq!(block.long_term_weight, long_term_weight);
            assert_eq!(block.cumulative_difficulty, cumulative_difficulty);
        }

        let rpc = HttpRpcClient::new(None).await;

        assert_eq(
            &rpc,
            0,                                                                        // height
            hex!("418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3"), // block_hash
            hex!("8a7b1a780e99eec31a9425b7d89c283421b2042a337d5700dfd4a7d6eb7bd774"), // pow_hash
            17592186044415, // generated_coins
            80,             // weight
            80,             // long_term_weight
            1,              // cumulative_difficulty
            0,              // tx_count (miner_tx excluded)
        )
        .await;

        assert_eq(
            &rpc,
            1,
            hex!("771fbcd656ec1464d3a02ead5e18644030007a0fc664c0a964d30922821a8148"),
            hex!("5aeebb3de73859d92f3f82fdb97286d81264ecb72a42e4b9f1e6d62eb682d7c0"),
            17592169267200,
            383,
            383,
            2,
            0,
        )
        .await;

        assert_eq(
            &rpc,
            202612,
            hex!("bbd604d2ba11ba27935e006ed39c9bfdd99b76bf4a50654bc1e1e61217962698"),
            hex!("84f64766475d51837ac9efbef1926486e58563c95a19fef4aec3254f03000000"),
            13138270467918,
            55503,
            55503,
            126654460829362,
            513,
        )
        .await;

        assert_eq(
            &rpc,
            1731606,
            hex!("f910435a5477ca27be1986c080d5476aeab52d0c07cf3d9c72513213350d25d4"),
            hex!("7c78b5b67a112a66ea69ea51477492057dba9cfeaa2942ee7372c61800000000"),
            3403774022163,
            6597,
            6597,
            23558910234058343,
            3,
        )
        .await;

        assert_eq(
            &rpc,
            2751506,
            hex!("43bd1f2b6556dcafa413d8372974af59e4e8f37dbf74dc6b2a9b7212d0577428"),
            hex!("10b473b5d097d6bfa0656616951840724dfe38c6fb9c4adf8158800300000000"),
            600000000000,
            106,
            176470,
            236046001376524168,
            0,
        )
        .await;

        assert_eq(
            &rpc,
            3132285,
            hex!("a999c6ba4d2993541ba9d81561bb8293baa83b122f8aa9ab65b3c463224397d8"),
            hex!("4eaa3b3d4dc888644bc14dc4895ca0b008586e30b186fbaa009d330100000000"),
            600000000000,
            133498,
            176470,
            348189741564698577,
            57,
        )
        .await;
    }
}
