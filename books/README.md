## Books
This directory contains the source files for Cuprate's various books.

The source files are edited here, and published in other repositories, see:
- [Cuprate's architecture book](https://github.com/Cuprate/architecture-book)
- [Cuprate's protocol book](https://github.com/Cuprate/monero-book)

## Build tools
Building the book(s) requires [Rust's cargo tool](https://doc.rust-lang.org/cargo/getting-started/installation.html) and [mdBook](https://github.com/rust-lang/mdBook).

After installing `cargo`, install `mdbook` with:
```bash
cargo install mdbook
```

## Building
To build a book, go into a book's directory and build:

```bash
# This build Cuprate's architecture book.
cd architecture/
mdbook build
```

The output will be in the `book` subdirectory (`architecture/book` for the above example). To open the book, you can open it in your web browser like so:
```bash
mdbook build --open
```
