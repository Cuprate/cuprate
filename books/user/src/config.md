# Configuration
`cuprated` reads its configuration file `Cuprated.toml` on startup - this is in the [TOML](https://toml.io) file format.

`cuprated` will try to look for `Cuprated.toml` in the follow places, in order:
- PATH specified in `--config-file`
- Current directory: `./Cuprated.toml`
- [OS specific directory](./resources/disk.md)

## `Cuprated.toml`
This is the default configuration file `cuprated` creates and uses.

If `cuprated` is started with no [`--options`](./cli.md), then the configuration used will be equivalent to this config file.

> Some values may be different for your exact system, generate the config with `cuprated --generate-config` to see the defaults for your system. 

```toml
{{#include ../Cuprated.toml}}
```
