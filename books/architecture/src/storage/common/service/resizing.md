# Resizing
Database backends that require manually resizing will, by default, use a similar algorithm as `monerod`'s.

Note that this only relates to the [`Service`](../common/service/intro.md) section, where the database is handled by `cuprate_database_service` itself, not the user. In the case of a user directly using `cuprate_database`, it is up to them on how to resize. The database will return [`RuntimeError::ResizeNeeded`](https://doc.cuprate.org/cuprate_database/enum.RuntimeError.html#variant.ResizeNeeded) when it needs resizing.

Within `service`, the resizing logic defined [here](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/service/write.rs#L139-L201) does the following:

- If there's not enough space to fit a write request's data, start a resize
- Each resize adds around [`1_073_745_920`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L104-L160) bytes to the current map size
- A resize will be attempted `3` times before failing

There are other [resizing algorithms](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L38-L47) that define how the database's memory map grows, although currently the behavior of [`monerod`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L104-L160) is closely followed.