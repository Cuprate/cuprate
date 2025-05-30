#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(unreachable_pub, reason = "This is a binary, everything `pub` is ok")]

mod cli;
mod rpc;

use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() {
    let now = Instant::now();

    // Parse CLI args.
    let cli::Args {
        rpc_url,
        update,
        rpc_tasks,
        buffer_limit,
        threads,
    } = cli::Args::get();

    // Set-up RPC client.
    let client = rpc::RpcClient::new(rpc_url, rpc_tasks).await;
    let top_height = client.top_height;
    println!("top_height: {top_height}");
    println!();

    todo!()
}

// some draft code for `monerod` <-> `cuprated` RPC compat testing

// /// represents a `monerod/cuprated` RPC request type.
// trait RpcRequest {
//     /// the expected response type, potentially only being a subset of the fields.
//     type SubsetOfResponse: PartialEq;

//     /// create a 'base' request.
//     fn base() -> Self;

//     /// permutate the base request into all (or practically) possible requests.
//     // e.g. `{"height":0}`, `{"height":1}`, etc
//     fn all_possible_inputs_for_rpc_request(self) -> Vec<Self>;

//     /// send the request, get the response.
//     ///
//     /// `monerod` and `cuprated` are both expected to be fully synced.
//     fn get(self, node: Node) -> Self::SubsetOfResponse;
// }

// enum Node {
//     Monerod,
//     Cuprated,
// }

// // all RPC requests.
// let all_rpc_requests: Vec<dyn RpcRequest> = todo!();

// // for each request...
// for base in all_rpc_requests {
//     // create all possible inputs...
//     let requests = all_possible_inputs_for_rpc_request(base);

//     // for each input permutation...
//     for r in requests {
//         // assert (a potential subset of) `monerod` and `cuprated`'s response fields match in value.
//         let monerod_response = r.get(Node::Monerod);
//         let cuprated_response = r.get(Node::Cuprated);
//         assert_eq!(
//             monerod_response.subset_of_response(),
//             cuprated_response.subset_of_response(),
//         );
//     }
// }
