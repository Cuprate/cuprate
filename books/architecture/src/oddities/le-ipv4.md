# Little-endian IPv4 addresses

## What
Monero encodes IPv4 addresses in [little-endian](https://en.wikipedia.org/wiki/Endianness) byte order.

## Expected
In general, [networking-related protocols/code use _networking order_ (big-endian)](https://en.wikipedia.org/wiki/Endianness#Networking).

## Why
TODO

- <https://github.com/monero-project/monero/issues/3826>
- <https://github.com/monero-project/monero/pull/5544>

## Affects
Any representation and (de)serialization of IPv4 addresses must keep little
endian in-mind, e.g. the P2P wire format or `int` encoded IPv4 addresses in RPC.

For example, [the `ip` field in `set_bans`](https://www.getmonero.org/resources/developer-guides/daemon-rpc.html#set_bans).

For Cuprate, this means Rust's [`Ipv4Addr::from_bits/from`](https://doc.rust-lang.org/1.82.0/src/core/net/ip_addr.rs.html#1182) cannot be used in these cases as [it assumes big-endian encoding](https://doc.rust-lang.org/1.82.0/src/core/net/ip_addr.rs.html#540).

## Source
- <https://github.com/monero-project/monero/blob/893916ad091a92e765ce3241b94e706ad012b62a/contrib/epee/include/net/net_utils_base.h#L97>
