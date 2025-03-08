# Cuprate release check-list
This is a template checklist used to track releases.

---

- [ ] Changelog
  - [ ] Relevent changes are added to `misc/changelogs/v${VERSION}.md`
- [ ] User Book
  - [ ] Update nesscessary documentation
  - [ ] Book title reflects `cuprated`'s version
- [ ] `cuprated`
  - [ ] `--help` output to `binaries/cuprated/help.txt`
  - [ ] Killswitch timestamp updated
- [ ] Repository
  - [ ] Decide specific commit
  - [ ] Create draft release
  - [ ] Create version tag
  - [ ] Build CI binaries
  - [ ] Collect binary hashes and PGP signatures
- [ ] `cuprated` testing
  - [ ] Full-sync from scratch
    - [ ] x64 Windows
    - [ ] x64 Linux
    - [ ] ARM64 macOS
    - [ ] ARM64 Linux
- [ ] Release
    - [ ] Add binaries to release
    - [ ] Verify hashes
    - [ ] Verify PGP signatures
    - [ ] Publish `Cuprate/user-book`
    - [ ] Release
- [ ] Release announcements
  - [ ] Reddit
  - [ ] Matrix
