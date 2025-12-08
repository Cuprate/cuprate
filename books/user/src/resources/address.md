# Address
IP addresses and ports used by `cuprated`.

Depending on the network used, the 1st number of the port used will change:
- Mainnet: `1` (e.g. `18080`)
- Testnet: `2` (e.g. `28080`)
- Stagenet: `3` (e.g. `38080`)

### P2P
`cuprated` can bind to a [IPv4](https://en.wikipedia.org/wiki/IPv4) or [IPv6](https://en.wikipedia.org/wiki/IPv6) address for P2P connections.

By default, this address is `0.0.0.0:18080`, which will bind to all available interfaces.

See the [`listen_on` and `p2p_port` option in the config file](../config.md) to manually set this address.

Setting the port to `0` will disable incoming P2P connections.

### RPC
By default, the:

- unrestricted RPC server is enabled and binds to `127.0.0.1:18081`
- restricted RPC server is disabled and binds to `0.0.0.0:18089`

See the [`address` option in the config file](../config.md) to manually set the addresses.
