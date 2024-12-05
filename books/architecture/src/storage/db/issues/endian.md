# Endianness
`cuprate_database`'s (de)serialization and storage of bytes are native-endian, as in, byte storage order will depend on the machine it is running on.

As Cuprate's build-targets are all little-endian ([big-endian by default machines barely exist](https://en.wikipedia.org/wiki/Endianness#Hardware)), this doesn't matter much and the byte ordering can be seen as a constant.

Practically, this means `cuprated`'s database files can be transferred across computers, as can `monerod`'s.