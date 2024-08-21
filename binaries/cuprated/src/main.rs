use clap::Parser;

mod blockchain;
mod config;
mod p2p;
mod rpc;
mod txpool;

#[derive(Parser)]
struct Args {}
fn main() {
    let _args = Args::parse();

    let (bc_read_handle, bc_write_handle, _) =
        cuprate_blockchain::service::init(cuprate_blockchain::config::Config::default()).unwrap();

    let async_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    async_rt.block_on(async move {
        let (block_verifier, tx_verifier, context_svc) = blockchain::init_consensus(
            bc_read_handle,
            cuprate_consensus::ContextConfig::main_net(),
        )
        .await
        .unwrap();

        //blockchain::init_blockchain_manager()
    });
}
