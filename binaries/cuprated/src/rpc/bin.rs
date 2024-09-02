use cuprate_rpc_interface::{RpcError, RpcResponse};
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

pub(super) fn map_request(
    state: CupratedRpcHandler,
    request: BinRequest,
) -> Result<BinResponse, RpcError> {
    use BinRequest as Req;
    use BinResponse as Resp;

    Ok(match request {
        Req::GetBlocks(r) => Resp::GetBlocks(get_blocks(state, r)?),
        Req::GetBlocksByHeight(r) => Resp::GetBlocksByHeight(get_blocks_by_height(state, r)?),
        Req::GetHashes(r) => Resp::GetHashes(get_hashes(state, r)?),
        Req::GetOutputIndexes(r) => Resp::GetOutputIndexes(get_output_indexes(state, r)?),
        Req::GetOuts(r) => Resp::GetOuts(get_outs(state, r)?),
        Req::GetTransactionPoolHashes(r) => {
            Resp::GetTransactionPoolHashes(get_transaction_pool_hashes(state, r)?)
        }
        Req::GetOutputDistribution(r) => {
            Resp::GetOutputDistribution(get_output_distribution(state, r)?)
        }
    })
}

fn get_blocks(
    state: CupratedRpcHandler,
    request: GetBlocksRequest,
) -> Result<GetBlocksResponse, RpcError> {
    todo!()
}

fn get_blocks_by_height(
    state: CupratedRpcHandler,
    request: GetBlocksByHeightRequest,
) -> Result<GetBlocksByHeightResponse, RpcError> {
    todo!()
}

fn get_hashes(
    state: CupratedRpcHandler,
    request: GetHashesRequest,
) -> Result<GetHashesResponse, RpcError> {
    todo!()
}

fn get_output_indexes(
    state: CupratedRpcHandler,
    request: GetOutputIndexesRequest,
) -> Result<GetOutputIndexesResponse, RpcError> {
    todo!()
}

fn get_outs(
    state: CupratedRpcHandler,
    request: GetOutsRequest,
) -> Result<GetOutsResponse, RpcError> {
    todo!()
}

fn get_transaction_pool_hashes(
    state: CupratedRpcHandler,
    request: GetTransactionPoolHashesRequest,
) -> Result<GetTransactionPoolHashesResponse, RpcError> {
    todo!()
}

fn get_output_distribution(
    state: CupratedRpcHandler,
    request: GetOutputDistributionRequest,
) -> Result<GetOutputDistributionResponse, RpcError> {
    todo!()
}
