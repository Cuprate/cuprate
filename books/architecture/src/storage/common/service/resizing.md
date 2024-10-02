# Resizing
As noted in the [`cuprate_database` resizing section](../../db/resizing.md),
builders on-top of `cuprate_database` are responsible for resizing the database.

In `cuprate_{blockchain,txpool}`'s case, that means the `tower::Service` must know
how to resize. This logic is shared between both crates, defined in `cuprate_database_service`:
<https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/service/src/service/write.rs#L107-L171>.

By default, this uses a _similar_ algorithm as `monerod`'s:

- [If there's not enough space to fit a write request's data](https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/service/src/service/write.rs#L130), start a resize
- Each resize adds around [`1,073,745,920`](https://github.com/Cuprate/cuprate/blob/2ac90420c658663564a71b7ecb52d74f3c2c9d0f/database/src/resize.rs#L104-L160) bytes to the current map size
- A resize will be [attempted `3` times](https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/service/src/service/write.rs#L110) before failing

There are other [resizing algorithms](https://doc.cuprate.org/cuprate_database/resize/enum.ResizeAlgorithm.html) that define how the database's memory map grows, although currently the behavior of `monerod` is closely followed (for no particular reason).