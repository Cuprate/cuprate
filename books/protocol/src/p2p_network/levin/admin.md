# Admin Messages

This chapter describes admin messages, and documents the current admin messages. Admin messages are a subset of messages that handle connection
creation, making sure connections are still alive, and sharing peer lists.

## Levin

All admin messages are in the request/response levin format. This means requests will set the [expect response bit](./levin.md#expect-response) and
responses will set the return code to [`1`](./levin.md#return-code).

## Messages

### Handshake

ID: `1001`[^handshake-id]

#### Request [^handshake-req] { #handshake-request }

| Fields         | Type                                                  | Description                          |
|----------------|-------------------------------------------------------|--------------------------------------|
| `node_data`    | [basic node data](../common_types.md#basic-node-data) | Static information about our node    |
| `payload_data` | [core sync data](../common_types.md#core-sync-data)   | Information on the node's sync state |

#### Response [^handshake-res] { #handshake-response }

| Fields               | Type                                                                     | Description                             |
|----------------------|--------------------------------------------------------------------------|-----------------------------------------|
| `node_data`          | [basic node data](../common_types.md#basic-node-data)                    | Static information about our node       |
| `payload_data`       | [core sync data](../common_types.md#core-sync-data)                      | Information on the node's sync state    |
| `local_peerlist_new` | A Vec of [peer list entry base](../common_types.md#peer-list-entry-base) | A list of peers in the node's peer list |

### Timed Sync

ID: `1002`[^timed-sync-id]

#### Request [^timed-sync-req] { #timed-sync-request }

| Fields         | Type                                                | Description                          |
| -------------- | --------------------------------------------------- | ------------------------------------ |
| `payload_data` | [core sync data](../common_types.md#core-sync-data) | Information on the node's sync state |

#### Response [^timed-sync-res] { #timed-sync-response }

| Fields               | Type                                                                     | Description                             |
|----------------------|--------------------------------------------------------------------------|-----------------------------------------|
| `payload_data`       | [core sync data](../common_types.md#core-sync-data)                      | Information on the node's sync state    |
| `local_peerlist_new` | A Vec of [peer list entry base](../common_types.md#peer-list-entry-base) | A list of peers in the node's peer list |

### Ping

ID: `1003`[^ping-id]

#### Request [^ping-req] { #ping-request }

No data is serialized for a ping request.

#### Response [^ping-res] { #ping-response }

| Fields    | Type   | Description                       |
| --------- | ------ | --------------------------------- |
| `status`  | string | Will be `OK` for successful pings |
| `peer_id` | u64    | The self assigned id of the peer  |

### Request Support Flags

ID: `1007`[^support-flags]

#### Request [^sf-req] { #support-flags-request }

No data is serialized for a ping request.

#### Response [^sf-res] { #support-flags-response }

| Fields          | Type | Description                                                  |
| --------------- | ---- | ------------------------------------------------------------ |
| `support_flags` | u32  | The peer's [support flags](../common_types.md#support-flags) |

---

[^handshake-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L213>

[^handshake-req]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L215>

[^handshake-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L227>

[^timed-sync-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L249>

[^timed-sync-req]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L251>

[^timed-sync-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L260>

[^ping-id]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L284>

[^ping-req]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L288>

[^ping-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L297>

[^support-flags]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L316>

[^sf-req]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L318>

[^sf-res]: <https://github.com/monero-project/monero/blob/cc73fe71162d564ffda8e549b79a350bca53c454/src/p2p/p2p_protocol_defs.h#L325>
