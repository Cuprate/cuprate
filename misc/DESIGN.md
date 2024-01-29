![Cuprate](logo/wordmark/CuprateWordmark.svg)

---

## Index

1. [Introduction](#introduction)
2. [P2P](#p2p)
   1. [levin-cuprate](#levin-cuprate)
   2. [monero-wire](#monero-wire)
   3. [cuprate-p2p](#cuprate-p2p)
3. [Verifier](#verifier)
   1. [block](#block)
   2. [transaction](#transaction)
4. [Syncer](#syncer)
   1. [Block downloader](#the-block-downloader)
5. [Database](#database)



### Introduction

This document outlines the initial plan for Cuprate, a Rust Monero node. Currently, Monero only
has one node implementation, which many would class as an issue.

This document isn't supposed to outline everything, but it is meant to give a good overview of the
plan.

Cuprate won't build everything from scratch and aims to use crates already in existence
when they're a good fit, an example is monero-serai for our transactions and blocks.

Cuprate makes heavy use of [tower](https://docs.rs/tower/latest/tower/index.html) to modularize its
parts. Using tower across the node will provide us with a consistent API and will allow us to use towers
extensive middleware, for tasks such as routing requests and timeouts.

---

### P2P

Cuprates P2P takes heavy inspiration from Zebra. The P2P crate will abstract the network into one endpoint,  
meaning other parts of the node will have no P2P code except from sending requests to this one endpoint.
This endpoint will be made of a few different tower::Services for the different routing methods, the most
simple method is to use a load balancing algorithm to send a request to one peer.

The peer to peer part of Cuprate will be split into multiple crates:

| Name                | Short Description                                                                                              |
|---------------------|----------------------------------------------------------------------------------------------------------------|
| levin-cuprate       | A library containing the levin header format.                                                                  |
| monero-wire         | A library containing all Monero P2P messages built on-top of `levin-cuprate`.                                  |
| monero-p2p          | A library defining the network zone abstraction and individual peer logic (handshakes etc).                    |
| monero-address-book | Contains the P2P address book, handles storing peers, getting peers etc.                                       |
| cuprate-p2p         | Defines the PeerSet and the different routing methods (excluding d++), has the logic for starting the network. |
| dandelion-pp        | Defines the D++ routing method.                                                                                |

#### levin-cuprate

This library will have the [levin header format](https://github.com/monero-project/monero/blob/master/docs/LEVIN_PROTOCOL.md#header),
with a [tokio-codec](https://docs.rs/tokio-util/0.7.8/tokio_util/codec/index.html) for encoding and
decoding p2p messages. To do this a trait `LevinMessage` will be used so users can define their own
P2P messages. This will allow other Rust projects to use the levin header format with different messages.

#### monero-wire

This will be a library built on top of [levin-cuprate](#levin-cuprate), It will contain every P2P
message with decoding/ encoding capability. This library will implement the `LevinMessage` trait.

The serialization format used for P2P messages has already been implemented in Rust, multiple times :). I have decided to
use monero-epee-bin-serde.

The monero-wire crate can be used in other Rust projects which need Monero's p2p network messages.

#### monero-p2p

This library will contain the network zone abstraction, which abstracts over clear-net, Tor, I2P and any future network.

This will also contain a `Client` and `Connection`. The `Connection` will be an async task that gives requests from
the peer to the inbound request handler and sends requests from Cuprate to the peer. The `Client` will implement
tower::Service and will simply pass requests from our node to the `Connection` task.

This will also contain a `Handshaker` which is responsible for taking a peer connection doing a handshake with it
and creating a `Client` and `Connection`.

This library is intended to be a more flexible monero p2p library than what cuprate-p2p is, allowing wider use in applications that need to
interact with Monero's p2p network but don't want/ or need Cuprates whole p2p stack.

To be compatible with tower::Service the Monero P2P protocol needs to be split into requests and responses.
Levin admin messages are already in the request/ response format, but notifications are not. For some
notifications it's easy: `GetObjectsRequest` but for others it's harder.
Here is a table of the Monero P2P messages put in either requests or responses:

```
/// Admin (These are already in request/ response format):
///     Handshake,
///     TimedSync,
///     Ping,
///     SupportFlags
/// Protocol:
///     Request: GetObjectsRequest,                 Response: GetObjectsResponse,
///     Request: ChainRequest,                      Response: ChainEntryResponse,
///     Request: FluffyMissingTransactionsRequest,  Response: NewFluffyBlock,  <- these 2 could be requests or responses
///     Request: GetTxPoolCompliment,               Response: NewTransactions, <-
///     Request: NewBlock,                          Response: None,
///     Request: NewFluffyBlock,                    Response: None,
///     Request: NewTransactions,                   Response: None
```

To split messages that can be requests or responses we will need to keep track of sent
requests.

#### monero-address-book

This implements Monero's p2p address book, this is a separate crate to monero-p2p to allow developers to create their own address book implementation
if `monero-address-book` is not suitable for them. `monero-address-book` will implement an `AddressBook` trait defined in `monero-p2p`.

#### cuprate-p2p

This library will abstract the P2P network away into one endpoint. Sadly, this endpoint will have to be made
up of different tower::Services for the different routing methods. For example, new blocks need to be sent to every
peer but a request for a block may only need to go to a single peer.

To allow splitting the endpoint into multiple tower::Services a `PeerSet` will be defined that will be shared between the services and is the structure
that holds on the currently connected peer on a certain network. The tower::Services will use this `PeerSet` to get peers to route requests to.

`cuprate-p2p` will also have a block downloader which will be a `futures::Stream`, it will use the `PeerSet` to find the chain with the highest cumulative 
difficulty and download that chain, when it gets a block it will pass it back through the `Stream`. 

#### dandelion-pp

This crate is separate from the other routing methods to allow wider usage, to do this it will be generic over the requests/ responses allowing users
to define them.

This crate won't be able to handle all of dandelion++ as that requires knowledge of the tx-pool but it will handle all of the routing side, deciding the current
state, getting the peers to route to etc.

Each request will have to include an origin, e.g self, fluff, so the d++ can route it correctly.

---

### Verifier

The verifier will be split into 2 different tower::Services: block and transaction. All checks will
be explicit and won't be scattered around the codebase, if for some reason we do have to scatter checks
(some are preformed at de-serialisation for example) they will be referred to in to in the non-scattered
location.

The verifiers tower::Services will be optional and behind a feature flags so projects that need Monero's consensus
rules won't have to use the tower interface.

#### Block

Responsible for performing block validation, able to handle multiple blocks at once.

Block verification will need Random-X. Cuprate, at the moment will use Rust bindings and not the Rust Monero
Miner, although in the future we would like to move to a Rust Random-X. We will also use Rust bindings to the
old CryptoNight POW(s).

#### Transaction

Responsible for validating transactions. This is able to handle one or more transactions at a time to
benefit from batching verification where we can, currently only bulletproofs(+) is able to be batched.
monero-serai already has the API to allow batch verification of bulletproofs(+). Also accepting multiple
transactions will also allow us to use a thread-pool like `rayon` to parallelize verification that can't
be batched.

---

### Database

The database interface will abstract away the underlying database to allow us to easily swap out the database for a different one,
this makes it possible to performance test different databases for our workload, which we plan to do. Initially we plan to go with
MDBX, a database similar to LMDB which is used in monerod.

We plan to investigate the database schema for optimisations as well, so our schema will more than likely be different than monerods.

Cuprate will interact with the database though a tower::Service providing another layer of abstraction, the service will make use of
the database interface abstraction. This allows us to make use of towers middleware for the database and makes the database conform to
the API of the rest of the node.
