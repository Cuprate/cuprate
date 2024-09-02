use cuprate_rpc_types::bin::{
    GetBlocksByHeightRequest, GetBlocksByHeightResponse, GetBlocksRequest, GetBlocksResponse,
    GetHashesRequest, GetHashesResponse, GetOutputDistributionRequest,
    GetOutputDistributionResponse, GetOutputIndexesRequest, GetOutputIndexesResponse,
    GetOutsRequest, GetOutsResponse, GetTransactionPoolHashesRequest,
    GetTransactionPoolHashesResponse,
};

use crate::rpc::CupratedRpcHandler;

pub(super) async fn map_request(state: CupratedRpcHandler, request: BinRpcRequest) -> BinRpcResponse {
    use BinRpcRequest as Req;
    use BinRpcResponse as Resp;

    match request {
        Req::GetBlocks(r) => Resp::GetBlocks(get_blocks(state, r)),
        Req::GetBlocksByHeight(r) => Resp::GetBlocksByHeight(get_blocks_by_height(state, r)),
        Req::GetHashes(r) => Resp::GetHashes(get_hashes(state, r)),
        Req::GetOutputIndexes(r) => Resp::GetOutputIndexes(get_output_indexes(state, r)),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r)),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r))
        }
        Req::GetOutputDistribution(r) => {
            Resp::GetOutputDistribution(get_output_distribution(state, r))
        }
    }
}

async fn get_blocks(state: CupratedRpcHandler, request: GetBlocksRequest) -> GetBlocksResponse {
    todo!()
}

async fn get_blocks_by_height(
    state: CupratedRpcHandler,
    request: GetBlocksByHeightRequest,
) -> GetBlocksByHeightResponse {
    todo!()
}

async fn get_hashes(state: CupratedRpcHandler, request: GetHashesRequest) -> GetHashesResponse {
    todo!()
}

async fn get_output_indexes(
    state: CupratedRpcHandler,
    request: GetOutputIndexesRequest,
) -> GetOutputIndexesResponse {
    todo!()
}

async fn get_outs(state: CupratedRpcHandler, request: GetOutsRequest) -> GetOutsResponse {
    todo!()
}

async fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> GetTransactionPoolHashesResponse {
    todo!()
}

async fn get_output_distribution(
    state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> GetOutputDistributionResponse {
    todo!()
}
