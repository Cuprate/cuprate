# Configuration
`cuprated` reads its configuration file `Cuprated.toml` on startup - this is in the [TOML](https://toml.io) file format.

`cuprated` will try to look for `Cuprated.toml` in the follow places, in order:
- PATH specified in `--config-file`
- Current directory: `./Cuprated.toml`
- [OS specific directory](./resources/disk.md)

## `Cuprated.toml`
This is the default configuration file `cuprated` creates and uses, sourced from [here](https://github.com/Cuprate/cuprate/blob/main/binaries/cuprated/config/Cuprated.toml).

If `cuprated` is started with no [`--options`](./cli.md), then the configuration will be equivalent to this config file.

```toml
{{#include ../../../binaries/cuprated/config/Cuprated.toml}}
```
