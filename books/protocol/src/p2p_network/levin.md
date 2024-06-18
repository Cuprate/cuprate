# Levin Protocol

This chapter describes the levin protocol.

## Buckets

A Bucket is a single piece of data that the levin protocol parser can decode, it will contain a p2p message or it will be part of a chain
of buckets that will be combined into a single message.

### Bucket Format

| Field  | Type                          | Size (bytes) |
| ------ | ----------------------------- | ------------ |
| Header | [BucketHeader](#bucketheader) | 33           |
| Body   | bytes                         | dynamic      |

### BucketHeader

Format:

| Field            | Type   | Size (bytes) |
| ---------------- | ------ | ------------ |
| Signature        | LE u64 | 8            |
| Size             | LE u64 | 8            |
| Expect Response  | bool   | 1            |
| Command          | LE u32 | 4            |
| Return Code      | LE i32 | 4            |
| Flags            | LE u32 | 4            |
| Protocol Version | LE u32 | 4            |

#### Signature

The signature field is fixed for every bucket and is used to tell apart peers running different protocols.

It's value should be `0x0101010101012101`

#### Size

This field represents the size of the buckets body.

#### Expect Response

Messages with the expect response field set must be responded to in order, other messages are still allowed in between responses.

#### Command

This field is an identifier for what specific message the bucket's body contains.

#### Return Code

This field represents the status of the response from the peer, requests and notifications should set this to `0` and successful
responses should be `1`.

#### Flags

This is a bit-flag field that determines what type of bucket this is:

| Type           | Bits set    |
| -------------- | ----------- |
| Request        | `0000_0001` |
| Response       | `0000_0010` |
| Start Fragment | `0000_0100` |
| End Fragment   | `0000_1000` |
| Dummy          | `0000_1100` |

#### Protocol Version

This is a fixed value of 1.
