# Genesis

Monero has a hardcoded genesis block that gets added to the blockchain on the first run of the daemon[^first-run]. The contents of this block
are different depending on the network.

For all networks the timestamp is set to 0, the major and minor version of the block are set to `CURRENT_BLOCK_MAJOR_VERSION` and
`CURRENT_BLOCK_MINOR_VERSION`[^version-set]. These two constants are set to 1 and 0 respectively[^version-defined]. The transaction
field is empty, and the previous block hash is not set so that field is zeroed.

## Mainnet

The nonce is set to 10,000 and the miner transaction is set to:
`013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1`
[^mainnet-params]

The mainnet genesis block will hash to: `418015bb9ae982a1975da7d79277c2705727a56894ba0fb246adaabb1f4632e3`.

The final block:

```json
{
    header: {
        major_version: 1,
        minor_version: 0,
        timestamp: 0,
        previous: [0; 32],
        nonce: 10000
        },
    miner_tx: "013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1",
    txs: [],
}
```

## Testnet

The nonce is set to 10,001 and the miner transaction is set to the same as mainnet[^testnet-params]

The testnet genesis block will hash to `48ca7cd3c8de5b6a4d53d2861fbdaedca141553559f9be9520068053cda8430b`.

The final block:

```json
{
    header: {
        major_version: 1,
        minor_version: 0,
        timestamp: 0,
        previous: [0; 32],
        nonce: 10001
        },
    miner_tx: "013c01ff0001ffffffffffff03029b2e4c0281c0b02e7c53291a94d1d0cbff8883f8024f5142ee494ffbbd08807121017767aafcde9be00dcfd098715ebcf7f410daebc582fda69d24a28e9d0bc890d1",
    txs: [],
}
```

## Stagenet

The nonce is set to 10,002 and the miner transaction is set to:
`013c01ff0001ffffffffffff0302df5d56da0c7d643ddd1ce61901c7bdc5fb1738bfe39fbe69c28a3a7032729c0f2101168d0c4ca86fb55a4cf6a36d31431be1c53a3bd7411bb24e8832410289fa6f3b`
[^stagenet-params].

The stagenet genesis block will hash to `76ee3cc98646292206cd3e86f74d88b4dcc1d937088645e9b0cbca84b7ce74eb`.

The final block:

```json
{
    header: {
        major_version: 1,
        minor_version: 0,
        timestamp: 0,
        previous: [0; 32],
        nonce: 10002
        },
    miner_tx: "013c01ff0001ffffffffffff0302df5d56da0c7d643ddd1ce61901c7bdc5fb1738bfe39fbe69c28a3a7032729c0f2101168d0c4ca86fb55a4cf6a36d31431be1c53a3bd7411bb24e8832410289fa6f3b",
    txs: [],
}
```

---

[^first-run]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/blockchain.cpp#L340>

[^version-set]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_core/cryptonote_tx_utils.cpp#L663-L665>

[^version-defined]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_config.h#L45-L46>

[^mainnet-params]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_config.h#L231-L232>

[^testnet-params]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_config.h#L272-L273>

[^stagenet-params]: <https://github.com/monero-project/monero/blob/eac1b86bb2818ac552457380c9dd421fb8935e5b/src/cryptonote_config.h#L287-L288>
