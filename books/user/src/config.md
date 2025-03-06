# Configuration
`cupratedd` reads its configuration file `Cuprated.toml` on startup - this is in the [TOML](https://toml.io) file format. Where this file is depends on the OS, details can be found in the [`Disk`](./resources/disk.md) section.

[`Command line`](command-line/command-line.md) flags will override any overlapping config values.

## `Cuprated.toml`
This is the default configuration file `cuprated` creates and uses, sourced from [here](https://github.com/Cuprate/cuprate/blob/main/binaries/cuprated/config/Cuprated.toml).

If `cuprated` is started with no `--flags`, then the configuration will be equivalent to this config file.

```toml
{{#include ../../../binaries/cuprated/config/Cuprated.toml}}
```
