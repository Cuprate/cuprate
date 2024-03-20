# Epee Encoding

This crate implements the epee binary format found in Monero; unlike other crates, 
this one does not use serde, this is not because serde is bad but its to reduce the 
load on maintainers as all the traits in this lib are specific to epee instead of 
general purpose.
