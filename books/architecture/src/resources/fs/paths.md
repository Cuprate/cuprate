# Index of PATHs
This is an index of all of the filesystem PATHs Cuprate actively uses.

The [`cuprate_helper::fs`](https://doc.cuprate.org/cuprate_helper/fs/index.html)
module defines the general locations used throughout Cuprate.

[`dirs`](https://docs.rs/dirs) is used internally, which follows
the PATH standards/conventions on each OS Cuprate supports, i.e.:
- the [XDG base directory](https://standards.freedesktop.org/basedir-spec/basedir-spec-latest.html) and the [XDG user directory](https://www.freedesktop.org/wiki/Software/xdg-user-dirs/) specifications on Linux
- the [Known Folder](https://msdn.microsoft.com/en-us/library/windows/desktop/bb776911(v=vs.85).aspx) system on Windows
- the [Standard Directories](https://developer.apple.com/library/content/documentation/FileManagement/Conceptual/FileSystemProgrammingGuide/FileSystemOverview/FileSystemOverview.html#//apple_ref/doc/uid/TP40010672-CH2-SW6) on macOS

## Cache
Cuprate's cache directory.

| OS      | PATH                                    |
|---------|-----------------------------------------|
| Windows | `C:\Users\Alice\AppData\Local\Cuprate\` |
| macOS   | `/Users/Alice/Library/Caches/Cuprate/`  |
| Linux   | `/home/alice/.cache/cuprate/`           |

## Config
Cuprate's config directory.

| OS      | PATH                                                |
|---------|-----------------------------------------------------|
| Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\`           |
| macOS   | `/Users/Alice/Library/Application Support/Cuprate/` |
| Linux   | `/home/alice/.config/cuprate/`                      |

## Data
Cuprate's data directory.

| OS      | PATH                                                |
|---------|-----------------------------------------------------|
| Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\`           |
| macOS   | `/Users/Alice/Library/Application Support/Cuprate/` |
| Linux   | `/home/alice/.local/share/cuprate/`                 |

## Blockchain
Cuprate's blockchain directory.

| OS      | PATH                                                           |
|---------|----------------------------------------------------------------|
| Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\blockchain\`           |
| macOS   | `/Users/Alice/Library/Application Support/Cuprate/blockchain/` |
| Linux   | `/home/alice/.local/share/cuprate/blockchain/`                 |

## Transaction pool
Cuprate's transaction pool directory.

| OS      | PATH                                                       |
|---------|------------------------------------------------------------|
| Windows | `C:\Users\Alice\AppData\Roaming\Cuprate\txpool\`           |
| macOS   | `/Users/Alice/Library/Application Support/Cuprate/txpool/` |
| Linux   | `/home/alice/.local/share/cuprate/txpool/`                 |

## Database
Cuprate's database location/filenames depend on:

- Which database it is
- Which backend is being used

---

`cuprate_blockchain` files are in the above mentioned `blockchain` folder.

`cuprate_txpool` files are in the above mentioned `txpool` folder.

---

If the `heed` backend is being used, these files will be created:

| Filename   | Purpose            |
|------------|--------------------|
| `data.mdb` | Main data file     |
| `lock.mdb` | Database lock file |

For example: `/home/alice/.local/share/cuprate/blockchain/lock.mdb`.

If the `redb` backend is being used, these files will be created:

| Filename    | Purpose            |
|-------------|--------------------|
| `data.redb` | Main data file     |

For example: `/home/alice/.local/share/cuprate/txpool/data.redb`.