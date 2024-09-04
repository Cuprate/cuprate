# Initialization
A database service is started simply by calling: [`init()`](https://doc.cuprate.org/cuprate_blockchain/service/fn.init.html).

This function initializes the database, spawns threads, and returns a:
- Read handle to the database (cloneable)
- Write handle to the database (not cloneable)
- The database itself

These handles implement the `tower::Service` trait, which allows sending requests and receiving responses `async`hronously.