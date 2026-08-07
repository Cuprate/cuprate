(list
 (channel
  (name 'guix)
  (url "https://git.savannah.gnu.org/git/guix.git")
  (branch "master")
  ;; Commit pin policy: pin to a tagged Guix release whenever the rust
  ;; available there is >= the maximum rustc version pulled in by
  ;; cuprate's workspace deps. Today (May 2026) v1.5.0 (230aa373f3) only
  ;; ships rust up to 1.88, but several deps (fjall, lsm-tree,
  ;; typed-index-collections, monero-daemon-rpc, ...) require up to 1.91,
  ;; so we pin to a recent master commit that ships rust-1.93. Repin to
  ;; the next stable Guix tag as soon as one carrying rust >= 1.91 lands.
  (commit "7041be9c117cbae2a5238bb22a0ff93ef11ca91a")
  ;; Guix v1.5.0 requires every channel to carry an `introduction` with the
  ;; commit + OpenPGP fingerprint that started the chain of trust; without
  ;; this `guix time-machine` aborts with "channel 'guix' lacks an
  ;; introduction and cannot be authenticated". These values are the
  ;; canonical introduction for the official Guix channel.
  (introduction
   (make-channel-introduction
    "9edb3f66fd807b096b48283debdcddccfea34bad"
    (openpgp-fingerprint
     "BBB0 2DDF 2CEA F6A8 0D1D  E643 A2A0 6DF2 A33A 54FA")))))
