# Maintained blob package verification

2026-10-02. Base commit `6782e54bec2fd1607253eb43338a989e62ee91a8`;
the implementation is the package commit introducing this report, on
`codex/mvp-blobs`. Core Row/Bundle and query/configuration/reaction code are unchanged.

`rom-blob` supplies an ordinary protected attachment Resource, named store builder,
provider-neutral storage port and bounded supervised lifecycle. The separate
`rom-blob-object-store` crate supplies trusted-folder and explicit S3 adapters.
The [design](../superpowers/specs/2026-10-02-maintained-blobs-design.md) and
[plan](../superpowers/plans/2026-10-02-maintained-blobs.md) record the approved scope.
Package READMEs state the accepted-worker authority, immutable publication,
current read authorization, uncertainty and explicit maintenance contracts.

## Executed evidence

Native `rom-dev`, Rust 1.99.0, two Cargo build jobs. Final build target:
`/var/tmp/rom-blob-review-target`. The earlier tmpfs target ran out of space during
linking. A move of these disposable artifacts to persistent scratch storage resolved
that environment failure. No source/test failure was suppressed.

- The initial three lifecycle tests failed against the service stub (`Unsupported`),
  then passed after implementation. Eleven final lifecycle tests cover complete/private
  attachment, generic Ready forgery rejection, failed/conflicting input, current
  revocation during a blocked read, permit ownership after caller cancellation,
  shutdown draining, input byte/chunk/count/time limits, backend panic supervision,
  lost provider and metadata acknowledgments, and an explicit orphan after a
  concurrent reservation deletion. Repeated Pending detachment preserves one cleanup
  identity and prevents later publication. Detachment leaves bytes for deliberate cleanup.
- Three default adapter tests cover shared create/get/head/delete semantics,
  conflicting and racing create-only publication, exact and exceeded byte limits,
  empty objects, invalid physical keys/endpoints, external oversized files,
  filesystem reopen, and full folder-plus-SQLite attachment restart/detachment.
- Negative control: temporarily replacing `PutMode::Create` with `Overwrite`
  made `folder_atomic_create_bounds_and_reopen` fail (`Ok(())` versus expected
  `Conflict`). The original source was restored before final verification.
- The real disposable MinIO suite ran all five adapter tests, including the same
  folder/S3 contract and a real signed PUT whose successful MinIO response was
  deliberately dropped. The adapter reported `Unknown`. An independent GET
  returned the complete accepted bytes. MinIO version:
  `RELEASE.2024-09-22T00-33-43Z`. No real cloud endpoint was contacted.
- Complete `scripts/check-rust` passed: formatting, workspace Clippy with warnings
  denied, all workspace tests/doctests, warning-free docs, external consumer,
  compiler fixtures, no-default core, auth feature/profile verification and native
  identity verification. Separately, all four strict OpenSpec validations passed.
  The normal `rom-blob` dependency tree contains no object_store/HTTP/database driver.
  A final additional Pending-detachment regression passed with the full eleven-test
  blob suite, its doctest and package Clippy after the complete workspace run;
  product code was unchanged by that final test addition.

Reproduce from the isolated worktree (or the integrated root after merge):

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-blob-review-target ./scripts/check
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/var/tmp/rom-blob-review-target \
  MINIO_BIN=/nix/store/7yy1v6cln7calpqkqax9bilszlr25wra-minio-2024-09-22T00-33-43Z/bin/minio \
  ./crates/rom-blob-object-store/verify-s3
```

Default tests visibly ignore the two real S3 cases. Only the explicit MinIO run
above establishes their result. Scratch logs were `/tmp/rom-blobs-check.log`,
`/tmp/rom-blobs-tests.log`, `/tmp/rom-blobs-minio.log` and
`/tmp/rom-blobs-overwrite-negative.log` inside `rom-dev`.

## Dependency evidence

Pinned Cargo.lock SHA-256:
`97d64740260f659f5719d17037c2fd2b465dda45817bc0342b3684c3d8a97275`.
The delta adds packages without upgrading existing registry package versions.
`object_store = 0.14.2` uses its `fs` default and explicit `aws` feature. Registry
metadata declares `MIT/Apache-2.0` and Rust 1.85; this package is tested and declared
only at the workspace's Rust 1.99 floor. Its upstream Apache NOTICE and license
are included in the adapter crate. Direct Tokio/futures/SHA-256/Serde/core packages
remain permissively licensed. All resolved metadata has a license declaration;
the graph includes the existing ISC/Apache/MIT/BSD combinations, Unicode data terms,
and the additional CDLA-Permissive-2.0 certificate bundle. The LGPL alternative in
one multiple-license declaration does not require selecting LGPL.

Executed `cargo-audit 0.22.2 audit --db /tmp/rom-dependency-audit/advisory-db
--no-fetch --json` against this lock: **256 dependencies, zero vulnerabilities,
zero warnings**, with RustSec database commit
`117edb3bed98e9be112f277b7615eea3252e7c43` (1280 advisories). This is an executed
cached-database audit, not a live claim about all native/system libraries or future
advisories. Full `cargo metadata --all-features --locked` license/MSRV metadata was
inspected; no declaration exceeded Rust 1.99. Integration with other dependency
packages requires a fresh combined lockfile delta/audit.

Official upstream contract/source references: [object_store](https://docs.rs/object_store/0.14.2/object_store/),
[atomic put modes](https://docs.rs/object_store/0.14.2/object_store/enum.PutMode.html),
[local filesystem](https://docs.rs/object_store/0.14.2/object_store/local/struct.LocalFileSystem.html).
The exact locally cached 0.14.2 sources were inspected for local staged hard-link
publication, fsync and S3 ETagMatch create-only handling; web retrieval of these
versioned documentation pages was unavailable during this run.

## Limits of the result

This is a bounded whole-object native Rust API, not multipart, resumable upload or
a new public HTTP upload route. Folder roots are trusted, not symlink sandboxes.
No power-loss, disk-full, permission-revocation, TLS, AWS or arbitrary S3 endpoint
certification is claimed. Detachment is not physical erasure. Automatic orphan
collection is not implemented. Cleanup needs host grace, reference checks and
quiescence. First, reconcile unknown metadata acknowledgment. A successful
publication followed by failed attachment can leave an orphan. Current reads receive
authorization again before disclosure. Accepted finalization can run under the explicitly
trusted worker as documented. It does not claim an atomic SQL/S3 transaction.

### Final drain review

Independent review found that the active-work counter could reach zero before the
supervised task released its last adapter/runtime owner. The lifecycle guard now
owns only accounting state. Task-owned services are dropped before the drain signal.
A 32-iteration multithreaded regression checks adapter weak ownership after caller
cancellation and shutdown. A temporary post-signal scheduling delay reproduced the
old failure. The same test passed with the fix. The delay is absent from production code.
The coordinator reran all 12 lifecycle tests, the doctest and warning-free Clippy.
