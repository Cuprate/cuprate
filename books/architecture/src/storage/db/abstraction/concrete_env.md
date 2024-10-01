# `ConcreteEnv`
After a backend is selected, the main database environment struct is "abstracted" by putting it in the non-generic, concrete [`struct ConcreteEnv`](https://doc.cuprate.org/cuprate_database/struct.ConcreteEnv.html).

This is the main object used when handling the database directly.

This struct contains all the data necessary to operate the database.
The actual database backend `ConcreteEnv` will use internally [depends on which backend feature is used](https://github.com/Cuprate/cuprate/blob/0941f68efcd7dfe66124ad0c1934277f47da9090/storage/database/src/backend/mod.rs#L3-L13).

`ConcreteEnv` itself is not too important, what is important is that:
1. It allows callers to not directly reference any particular backend environment
1. It implements [`trait Env`](https://doc.cuprate.org/cuprate_database/trait.Env.html) which opens the door to all the other database traits

The equivalent "database environment" objects in the backends themselves are:
- [`heed::Env`](https://docs.rs/heed/0.20.0/heed/struct.Env.html)
- [`redb::Database`](https://docs.rs/redb/2.1.0/redb/struct.Database.html)