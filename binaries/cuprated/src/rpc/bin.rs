use anyhow::Error;

use cuprate_rpc_types::{
    bin::{
        BinRequest, BinResponse, GetBlocksByHeightRequest, GetBlocksByHeightResponse,
        GetBlocksRequest, GetBlocksResponse, GetHashesRequest, GetHashesResponse,
        GetOutputIndexesRequest, GetOutputIndexesResponse, GetOutsRequest, GetOutsResponse,
        GetTransactionPoolHashesRequest, GetTransactionPoolHashesResponse,
    },
    json::{GetOutputDistributionRequest, GetOutputDistributionResponse},
};

use crate::rpc::CupratedRpcHandler;

/// Map a [`BinRequest`] to the function that will lead to a [`BinResponse`].
pub(super) async fn map_request(
    state: CupratedRpcHandler,
    request: BinRequest,
) -> Result<BinResponse, Error> {
    use BinRequest as Req;
    use BinResponse as Resp;

    Ok(match request {
        Req::GetBlocks(r) => Resp::GetBlocks(get_blocks(state, r).await?),
        Req::GetBlocksByHeight(r) => Resp::GetBlocksByHeight(get_blocks_by_height(state, r).await?),
        Req::GetHashes(r) => Resp::GetHashes(get_hashes(state, r).await?),
        Req::GetOutputIndexes(r) => Resp::GetOutputIndexes(get_output_indexes(state, r).await?),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r).await?),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r).await?)
        }
        Req::GetOutputDistribution(r) => {
            Resp::GetOutputDistribution(get_output_distribution(state, r).await?)
        }
    })
}

async fn get_blocks(
    state: CupratedRpcHandler,
    request: GetBlocksRequest,
) -> Result<GetBlocksResponse, Error> {
    todo!()
}

async fn get_blocks_by_height(
    state: CupratedRpcHandler,
    request: GetBlocksByHeightRequest,
) -> Result<GetBlocksByHeightResponse, Error> {
    todo!()
}

async fn get_hashes(
    state: CupratedRpcHandler,
    request: GetHashesRequest,
) -> Result<GetHashesResponse, Error> {
    todo!()
}

async fn get_output_indexes(
    state: CupratedRpcHandler,
    request: GetOutputIndexesRequest,
) -> Result<GetOutputIndexesResponse, Error> {
    todo!()
}

async fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    todo!()
}

async fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    todo!()
}

async fn get_output_distribution(
    state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    todo!()
}
