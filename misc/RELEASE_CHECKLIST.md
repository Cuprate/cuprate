# Cuprate release check-list
This is a template checklist used to track releases.

---

- Changelog
  - [ ] Relevant changes added to `misc/changelogs/cuprated/${VERSION}.md`
- Fast sync
  - [ ] Update hashes, see `misc/FAST_SYNC_HASHES.md`
- User Book
  - [ ] Update necessary documentation
  - [ ] Book title reflects `cuprated`'s version
- `cuprated`
  - [ ] Killswitch timestamp updated
- Repository
  - [ ] Decide specific commit
  - [ ] Create draft release
  - [ ] Create version tag
  - [ ] Build CI binaries
  - [ ] Collect binary hashes and PGP signatures
- `cuprated` testing
  - Full-sync from scratch
    - [ ] x64 Windows
    - [ ] x64 Linux
    - [ ] ARM64 macOS
    - [ ] ARM64 Linux
- Release
    - [ ] Add binaries to release
    - [ ] Verify hashes
    - [ ] Verify PGP signatures
    - [ ] Publish `Cuprate/user-book`
    - [ ] Release
- Release announcements
  - [ ] Reddit
  - [ ] Matrix
