# Epee Encoding

- [What](#what)
- [Features](#features)
- [Usage](#usage)
- [Derive Attributes](#derive-attributes)
- [No std](#no-std)
- [Options](#options)

## What
This crate implements the epee binary format found in Monero; unlike other crates, 
this one does not use serde, this is not because serde is bad but its to reduce the 
load on maintainers as all the traits in this lib are specific to epee instead of 
general purpose.

## Features

### Default

The default feature enables the [derive](#derive) feature.

### Derive

This feature enables the derive macro for creating epee objects for example:

```rust
use epee_encoding::EpeeObject;
#[derive(EpeeObject)]
struct Test {
    val: u8
}
```

## Usage

### example without derive:
```rust
use epee_encoding::{EpeeObject, EpeeObjectBuilder, read_epee_value, write_field, to_bytes, from_bytes};
use epee_encoding::io::{Read, Write};

pub struct Test {
    val: u64
}

#[derive(Default)]
pub struct __TestEpeeBuilder {
    val: Option<u64>,
}

impl EpeeObjectBuilder<Test> for __TestEpeeBuilder {
    fn add_field<R: Read>(&mut self, name: &str, r: &mut R) -> epee_encoding::error::Result<bool> {
        match name {
            "val" => {self.val = Some(read_epee_value(r)?);}
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn finish(self) -> epee_encoding::error::Result<Test> {
        Ok(
            Test {
                val: self.val.ok_or_else(|| epee_encoding::error::Error::Format("Required field was not found!"))?
            }
        )
    }
}

impl EpeeObject for Test {
    type Builder = __TestEpeeBuilder;
    
    fn number_of_fields(&self) -> u64 {
        1
    }

    fn write_fields<W: Write>(&self, w: &mut W) -> epee_encoding::error::Result<()> {
       // write the fields
       write_field(&self.val, "val", w)
   }
}


let data = [1, 17, 1, 1, 1, 1, 2, 1, 1, 4, 3, 118, 97, 108, 5, 4, 0, 0, 0, 0, 0, 0, 0]; // the data to decode;
let val: Test = from_bytes(&data).unwrap();
let data = to_bytes(&val).unwrap();


```

### example with derive:
```rust
use epee_encoding::{EpeeObject, from_bytes, to_bytes};

#[derive(EpeeObject)]
struct Test {
    val: u64
}


let data = [1, 17, 1, 1, 1, 1, 2, 1, 1, 4, 3, 118, 97, 108, 5, 4, 0, 0, 0, 0, 0, 0, 0]; // the data to decode;
let val: Test = from_bytes(&data).unwrap();
let data = to_bytes(&val).unwrap();

```

## Derive Attributes

The `EpeeObject` derive macro has a few attributes which correspond to specific C/C++ macro fields.

- [epee_flatten](#epeeflatten)
- [epee_alt_name](#epeealtname)
- [epee_default](#epeedefault)

### epee_flatten

This is equivalent to `KV_SERIALIZE_PARENT`, it flattens all the fields in the object into the parent object. 

so this in C/C++:
```cpp
struct request_t: public rpc_request_base
    {
      uint8_t major_version;

      BEGIN_KV_SERIALIZE_MAP()
        KV_SERIALIZE_PARENT(rpc_request_base)
        KV_SERIALIZE(major_version)
      END_KV_SERIALIZE_MAP()
    };
```
Would look like this in Rust:
```rust
#[derive(EpeeObject)]
struct RequestT {
    #[epee_flatten]
    rpc_request_base: RequestBase,
    major_version: u8,
}
```

### epee_alt_name

This allows you to re-name a field for when its encoded, although this isn't related to a specific macro in 
C/C++ this was included because Monero has [some odd names](https://github.com/monero-project/monero/blob/0a1eaf26f9dd6b762c2582ee12603b2a4671c735/src/cryptonote_protocol/cryptonote_protocol_defs.h#L199).

example:
```rust
#[derive(EpeeObject)]
pub struct HandshakeR {
    #[epee_alt_name("node_data")]
    pub node_daa: BasicNodeData,
}
```

### epee_default

This is equivalent to `KV_SERIALIZE_OPT` and allows you to specify a default value for a field, when a default value
is specified the value will be used if it is not contained in the data and the field will not be encoded if the value is 
the default value.

so this in C/C++:
```cpp
struct request_t
{
  std::vector<blobdata>   txs;
  std::string _; // padding
  bool dandelionpp_fluff; //zero initialization defaults to stem mode

  BEGIN_KV_SERIALIZE_MAP()
    KV_SERIALIZE(txs)
    KV_SERIALIZE(_)
    KV_SERIALIZE_OPT(dandelionpp_fluff, true) // backwards compatible mode is fluff
  END_KV_SERIALIZE_MAP()
};
```

would look like this in Rust:
```rust
#[derive(EpeeObject)]
struct RequestT {
    txs: Vec<Vec<u8>>,
    #[epee_alt_name("_")]
    padding: Vec<u8>,
    #[epee_default(true)]
    dandelionpp_fluff: bool,
}
```

## No std

This crate is no-std.

## Options

To have an optional field, you should wrap the type in `Option` and use the `epee_default` attribute.
So it would look like this: 

```rust
#[derive(EpeeObject)]
struct T {
    #[epee_default(None)]
    val: Option<u8>,
}
```