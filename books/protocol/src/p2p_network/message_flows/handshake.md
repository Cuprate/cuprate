# Handshakes

Handshakes are used to establish connections to peers.

## Flow

The default handshake flow is made up of the connecting peer sending a [handshake request](../levin/admin.md#handshake-request) and the
receiving peer responding with a [handshake response](../levin/admin.md#handshake-response).

It should be noted that not all other messages are banned during handshakes, for example, support flag requests and even some protocol
requests can be sent.

### Handshake Request Checks

The receiving peer will check:

- The `network_id` is network ID expected.[^network-id]
- The connection is an incoming connection.[^req-incoming-only]
- The peer hasn't already completed a handshake.[^double-handshake]
- If the network zone is public, then the `peer_id` must not be the same as ours.[^same-peer-id]
- The core sync data is not malformed.[^core-sync-data-checks]

### Handshake Response Checks

The initiating peer will check:

- The `network_id` is network ID expected.[^res-network-id]
- The number of peers in the peer list is less than `250`.[^max-peer-list-res]
- All peers in the peer list are in the same zone.[^peers-all-in-same-zone]
- The core sync data is not malformed.[^core-sync-data-checks]
- If the network zone is public, then the `peer_id` must not be the same as ours.[^same-peer-id-res]

---

[^network-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2510>

[^req-incoming-only]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2519>

[^double-handshake]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2527>

[^same-peer-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2539>

[^core-sync-data-checks]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/cryptonote_protocol/cryptonote_protocol_handler.inl#L341>

[^res-network-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L1164>

[^max-peer-list-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2170>

[^peers-all-in-same-zone]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2182>

[^same-peer-id-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L1195>
