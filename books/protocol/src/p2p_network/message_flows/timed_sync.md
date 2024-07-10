# Timed Syncs

A timed sync request is sent every 60 seconds to make sure the connection is still live.

## Flow

First the timed sync initiator will send a [timed sync request](../levin/admin.md#timed-sync-request), the receiver will then
respond with a [timed sync response](../levin/admin.md#timed-sync-response)

### Timed Sync Request Checks

- The core sync data is not malformed.[^core-sync-data-checks]


### Timed Sync Response Checks

- The core sync data is not malformed.[^core-sync-data-checks]
- The number of peers in the peer list is less than `250`.[^max-peer-list-res]
- All peers in the peer list are in the same zone.[^peers-all-in-same-zone]

---

[^core-sync-data-checks]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2464>

[^max-peer-list-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2170>

[^peers-all-in-same-zone]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/net_node.inl#L2182>

