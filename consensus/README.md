# Consensus Rules

This folder contains 2 crates: `cuprate-consensus-rules` (rules) and `cuprate-consensus`. `cuprate-consensus-rules`
contains the raw-rules
and is built to be a more flexible library which requires the user to give the correct data and do minimal
calculations, `cuprate-consensus`
on the other hand contains multiple tower::Services that handle tx/ block verification as a whole with a `context`
service that
keeps track of blockchain state. `cuprate-consensus` uses `cuprate-consensus-rules` internally.

If you are looking to use monero consensus rules it's recommended you try to integrate `cuprate-consensus` and fall back
to
`cuprate-consensus-rules` if you need more flexibility.
