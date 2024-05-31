use cuprate_blockchain::service::DatabaseReadHandle;

pub struct P2PRequestHandler {
    database: DatabaseReadHandle,

    txpool: (),
}
