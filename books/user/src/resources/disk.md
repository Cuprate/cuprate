# Disk
`cuprated` requires at least ~300 GB of disk storage for operation although 500+ GB is recommended.

## Cache
The directory used for cache files is:

| OS      | Directory                              |
|---------|----------------------------------------|
| Windows | `C:\Users\User\AppData\Local\Cuprate\` |
| macOS   | `/Users/User/Library/Caches/Cuprate/`  |
| Linux   | `/home/user/.cache/cuprate/`           |

Although not recommended, this directory can be deleted without major disruption to `cuprated`.

The files in this directory are:

| File                   | Purpose |
|------------------------|---------|
| `addressbook/ClearNet` | P2P state for clear-net

## Configuration
The directory used for files related to configuration is:

| OS      | Directory                                          |
|---------|----------------------------------------------------|
| Windows | `C:\Users\User\AppData\Roaming\Cuprate\`           |
| macOS   | `/Users/User/Library/Application Support/Cuprate/` |
| Linux   | `/home/user/.config/cuprate/`                      |

The files in this directory are:

| File            | Purpose |
|-----------------|---------|
| `Cuprated.toml` | `cuprated` configuration file

## Data
The directory used for general data is:

| OS      | Directory                                          |
|---------|----------------------------------------------------|
| Windows | `C:\Users\User\AppData\Roaming\Cuprate\`           |
| macOS   | `/Users/User/Library/Application Support/Cuprate/` |
| Linux   | `/home/user/.local/share/cuprate/`                 |

The sub-directories are:

| Sub-directory         | Purpose |
|-----------------------|---------|
| `fjall/`              | Blockchain and transaction pool data
| `tapes/`              | Blockchain data
| `logs/{YYYY-MM-DD}`   | Log files for each day