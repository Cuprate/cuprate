# Verifying
Verification of release files is optional but highly recommended. This ensures that you have not downloaded a tampered version of `cuprated`.

To verify release files of `cuprated`, follow these instructions:

### Download verification files for latest release
- Latest release: <https://github.com/Cuprate/cuprate/releases/latest>
- Hashes: <https://github.com/Cuprate/cuprate/releases/download/v0.0.1/SHA256SUMS>
- Hash signatures: <https://github.com/Cuprate/cuprate/releases/download/v0.0.1/SHA256SUMS.asc>

### Verify the hashes
After downloading the release files, compare their hashes with the `SHA256SUMS` file.

```bash
sha256sum --ignore-missing --check SHA256SUMS
```

You should see something like: `cuprate-0.0.1-linux-x64.tar.gz: OK`.

### Verify the hash signatures
Cuprate releases are signed by multiple individuals.

First, import the PGP keys for all individuals:
```bash
# Clone the Cuprate repository.
git clone https://github.com/Cuprate/cuprate

# Import all PGP keys.
gpg --import cuprate/misc/gpg_keys/*.asc
```

Then, confirm all signatures:
```bash
gpg --verify SHA256SUMS.asc
```

You should see `gpg: Good signature` for all keys.