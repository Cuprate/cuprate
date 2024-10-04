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

use crate::rpc::CupratedRpcHandlerState;

/// Map a [`BinRequest`] to the function that will lead to a [`BinResponse`].
pub(super) async fn map_request(
    state: CupratedRpcHandlerState,
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
    state: CupratedRpcHandlerState,
    request: GetBlocksRequest,
) -> Result<GetBlocksResponse, Error> {
    todo!()
}

async fn get_blocks_by_height(
    state: CupratedRpcHandlerState,
    request: GetBlocksByHeightRequest,
) -> Result<GetBlocksByHeightResponse, Error> {
    todo!()
}

async fn get_hashes(
    state: CupratedRpcHandlerState,
    request: GetHashesRequest,
) -> Result<GetHashesResponse, Error> {
    todo!()
}

async fn get_output_indexes(
    state: CupratedRpcHandlerState,
    request: GetOutputIndexesRequest,
) -> Result<GetOutputIndexesResponse, Error> {
    todo!()
}

async fn get_outs(
    state: CupratedRpcHandlerState,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, Error> {
    todo!()
}

async fn get_transaction_pool_hashes(
    state: CupratedRpcHandlerState,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, Error> {
    todo!()
}

async fn get_output_distribution(
    state: CupratedRpcHandlerState,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, Error> {
    todo!()
}
