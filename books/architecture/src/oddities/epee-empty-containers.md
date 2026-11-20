# Epee empty containers

## What
Monero's serialization in the `epee` library, responsible for both JSON and binary encoding, will not serialize containers that are empty.

This causes some issues for the binary format:
- <https://github.com/monero-rs/monero-epee-bin-serde/issues/49>
- <https://github.com/monero-project/monero/pull/8940>

For JSON, fields with empty containers will cause the field itself to not be present in the JSON output.

## Expected
Serialization of the key and an empty field, for example, this is expected:
```json
{
  "empty": [],
  "non_empty": [1]
}
```

However `monerod` will write:
```json
{
  "non_empty": [1]
}
```

## Why
TODO

## Affects
TODO

## Source
- TODO