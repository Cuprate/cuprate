# Targets
The target is described in the `tracing `docs as:

> a string that categorizes part of the system where the span or event occurred.

By default, the `tracing` crate will use the module path as the target, we override this to make it easier for users
to filter logs.

`tracing-subscriber` allows filtering logs based on target prefixes: [Targets](https://docs.rs/tracing-subscriber/0.3.18/tracing_subscriber/filter/targets/struct.Targets.html#).
To reduce potential friction by using custom targets we mimic what module paths would look like by using `::`. 

``

### P2P

| Target                        | Description                                             |
|-------------------------------|---------------------------------------------------------|
| `p2p`                         | Anything to do with the peer to peer network.           |
| `p2p::address_book`           | The address book of P2P peers.                          |
| `p2p::connection`             | Anything to do with P2P connections.                    |
| `p2p::connection::levin`      | The levin protocol parser.                              |
| `p2p::connection::handshaker` | The handshaker handles doing handshakes with new peers. |
| `p2p::connection::task`       | The task that maintains the peer connection.            |
| `p2p::peer_set`               | Contains connected peers.                               |
| `p2p::outbound_maintainer`    | Maintains the outbound connection count.                |
| `p2p::inbound_server`         | Handles incoming P2P connections.                       |
| `p2p::block_downloader`       | Downloads blocks when we fall behind.                   |

### Dandelion

| Target                    | Description                     |
|---------------------------|---------------------------------|
| `dandelion`               | Anything to do with dandelion++ |
| `dandelion::pool_manager` | The dandelion pool manager.     |
| `dandelion::router`       | The dandelion router.           |

### Consensus

| Target                      | Description                    |
|-----------------------------|--------------------------------|
| `consensus`                 | Anything to do with consensus. |
| `consensus::block_verifier` | Block verification.            |
| `consensus::tx_verifier`    | Transaction verification.      |

### Storage

| Target                | Description                      |
|-----------------------|----------------------------------|
| `storage`             | Anything to do with storage.     |
| `storage::service`    | The storage service abstraction. |
| `storage::blockchain` | Blockchain storage.              |
| `storage::txpool`     | Txpool storage.                  |

### cuprated

| Target       | Description                      |
|--------------|----------------------------------|
| `blockchain` |      |
| `txpool`     |  |
|              |  |
