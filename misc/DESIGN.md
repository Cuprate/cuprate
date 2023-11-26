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

The peer to peer part of Cuprate will be split into 3 crates:

| Name          | Short Description                                                                             |
|---------------|-----------------------------------------------------------------------------------------------|
| levin-cuprate | A library containing the levin header format.                                                 |
| monero-wire   | A library containing all Monero P2P messages built on-top of `levin-cuprate`.                 |
| cuprate-p2p   | A library abstracting the P2P network away, with logic for handshakes, the address book, etc. |

#### levin-cuprate

This library will have the [levin header format](https://github.com/monero-project/monero/blob/master/docs/LEVIN_PROTOCOL.md#header),
with a [tokio-codec](https://docs.rs/tokio-util/0.7.8/tokio_util/codec/index.html) for encoding and 
decoding p2p messages. To do this a trait `LevinMessage` will be used so users can define their own 
P2P messages. This will allow other Rust projects to use the levin header format with different messages.

#### monero-wire

This will be a library built on top of [levin-cuprate](#levin-cuprate), It will contain every P2P 
message with decoding/ encoding capability. This library will implement the `LevinMessage` trait.

The serialization format used for P2P messages has already been implemented in Rust, multiple times :). I have decided to 
implement it yet again in the crate: `epee-encoding`. This crate was created specifically for use in Cuprate.

The monero-wire crate will be able to be used in other Rust projects who want to interact with Monero's P2P network. 

#### cuprate-p2p

This library will abstract the P2P network away into one endpoint. Sadly, this endpoint will have to be made 
up of different tower::Services for the different routing methods. For example, new blocks need to be sent to every 
peer but a request may only need to go to a single peer.

The library will be split into many modules:

##### protocol

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

##### peer

This will contain a `Client` and `Connection`. The `Connection` will be an async task that gives requests from
the peer to the inbound request handler and sends requests from Cuprate to the peer. The `Client` will implement
tower::Service and will simply pass requests from our node to the `Connection` task.

This module will also contain a `Handshaker` which is responsible for taking a peer connection doing a handshake with it 
and creating a `Client` and `Connection`.

##### address book

The address book will use the same overall idea as monerod's address book. It will contain a White, Grey and Anchor
list. Under the hood we will have 3 separate address books for each network (clear, i2p, Tor) and will route requests 
using a tower::Steer. 

White: Peers we have connected to at some point.

Gray: Peers we have heard about but haven't attempted to connect to.

Anchor: A list of currently connected peers so, if we were to re-start, we can choose a couple peers from this list to 
reduce our chance of being isolated.

The address book will be an async task which we will be able to interact with through a tower::Service.

##### peer set

This is the part of the P2P crate that holds all currently connected peers. The rest of Cuprate will interact with this 
structure to send requests to the network. There will be multiple tower::Service interfaces to interact with the network
for the different routing methods:

- broadcast: send a message to all ready `Clients`
- single: use a load balancing algorithm to route a message to a single `Client`
- multiple: sends a request to an amount of peers chosen by the requester, this might be joined with broadcast.

*There may be more routing methods in the future*

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

Transaction verification will be split into 2 sections: hard and soft.

##### Hard: 
If a transaction fails this, the node will reject the transaction completely including in blocks. 

##### Soft:
If a transaction fails this, the node won't broadcast the transaction but will allow it in blocks.

This is to make it easy to do things like stopping transaction with too large extra fields and making transactions
follow a standard decoy selection algorithm (this isn't planned) without the need for a hard fork.

---

### Syncer

The syncer will be responsible for syncing the blockchain after falling behind. It will utilize many of the components 
we have discussed, a new tower::Service is needed though `The block downloader`.

#### The block downloader

This will be responsible for finding the chain tip and getting blocks from peers, it does no verification* and simply gets 
the next block.

(*) some verification may be done here just to see if the block we got is the one we asked for but TBD.

The syncer will call the block downloader to get the chain-tip then it will call for the next batch of blocks, when it has this batch 
it will send it to the block verifier, which will return if the blocks are valid, if they are we add them to our blockchain.

---

### Database

The database interface will abstract away the underlying database to allow us to easily swap out the database for a different one,
this makes it possible to performance test different databases for our workload, which we plan to do. Initially we plan to go with 
MDBX, a database similar to LMDB which is used in monerod.

We plan to investigate the database schema for optimisations as well, so our schema will more than likely be different than monerods. 

Cuprate will interact with the database though a tower::Service providing another layer of abstraction, the service will make use of 
the database interface abstraction. This allows us to make use of towers middleware for the database and makes the database conform to 
the API of the rest of the node.
