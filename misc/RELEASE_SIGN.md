# Cuprate release hashes and signatures
This is a template used to collect PGP signatures on `cuprated` releases.

---

The purpose of this issue is to collect signatures confirming the hashes of `cuprated` release files.

## Steps
1. Copy the below `SHA256SUMS` text to a file
2. Run `gpg --clearsign --detach SHA256SUMS`
3. Upload the following `SHA256SUMS.asc` here
4. (Optional) submit a PR to add the signing key to `misc/gpg_keys`

## `SHA256SUMS`
```
Version    | cuprated 0.0.1 NAME_OF_METAL
Repository | https://github.com/Cuprate/cuprate
Commit     | <...>

Hashes:
```