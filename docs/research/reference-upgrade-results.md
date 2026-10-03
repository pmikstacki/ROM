# Reference application upgrade results

Date: 2026-10-03. Scope: release stage 2.4. Final verification passed.
Source baseline: `9ff10f7`, with the reference upgrade changes in this worktree.

## Application contract

Checkout version 2 replaces a text stock ID with `ResourceRef<Stock>`. The wire
value stays a string. The descriptor now requires a live target and restricts
target deletion. The old application profile keeps the version-1 descriptor and
codec. Both profiles share business rules and the other application definitions.

The host uses public maintenance methods to migrate, back up and restore the
database. It starts the current application only after those operations succeed.
The restored runtime completes pending compensation and reads external folder
attachments. Database archives contain attachment metadata, not attachment bytes.

See the [operator guide](../reference-upgrade.md) and
[design](../superpowers/specs/2026-10-03-reference-upgrade-design.md).

## Acceptance coverage

| Scenario | Required result |
| --- | --- |
| Clean application stop | Both native adapters complete the finite `upgrade` command. |
| Actual process exit | The child exits with code 86 after rejection commits, before shutdown or worker drain. Recovery releases only that reservation. |
| Reference rebuild | A pending checkout prevents deletion of empty stock. Deleting the checkout permits stock deletion. |
| Old receipt | Replay returns the original revision through the retained codec, with no new mutation, event, work or effect. |
| Missing old codec | The current definition without its replay codec rejects the old receipt. |
| Revoked authority | The original receipt owner receives `Denied` after its current host grant is removed. |
| Rejected migration | Invalid conversion, a missing target and incompatible or unvalidated work prevent publication. |
| Existing destination | Maintenance preserves an occupied destination. A later attempt with a fresh path succeeds. |
| Source preservation | The original database and existing SQLite WAL retain their exact bytes. Maintenance does not add a source WAL. |

The child test uses the actual demo profile and reference scenario. It does not
replace the adapter with an in-memory model. SQLite and redb use the same public
application operations. Separate unit tests preserve the old descriptor and
validate callback identity, version and payload compatibility.

## Defect found by application acceptance

The first coordinator run passed three parent tests and failed two; one child
fixture was ignored by default and explicitly invoked by its parent. Clean
SQLite maintenance created a new source WAL. The actual process-exit scenario
already had a WAL and passed. The stored bytes did not change in the clean case,
but the file inventory did. The source-preservation assertion correctly failed.

Initial log: `rom-dev:/var/tmp/rom-reference-upgrade-root-first.log`.
The correction canonicalizes the source path before sidecar inspection. Without
a WAL, it uses an encoded immutable file URI. With a WAL, it retains an ordinary
read-only transaction. The host must keep both sources offline.
SQLite documents read-only WAL sidecar creation and the immutable alternative in
its [WAL documentation](https://www.sqlite.org/wal.html#read_only_databases).
Immutable mode disables locking and change detection, as described in the
[URI documentation](https://www.sqlite.org/uri.html).

A reviewer also reproduced creation of a WAL when a cold rollback journal exists.
Maintenance now rejects any rollback journal before opening SQLite. Host-controlled
recovery on a separate copy is a prerequisite for that source state. This avoids
ignoring a potentially hot journal. The source-preservation contract stays intact.

Five focused regression cases passed, including successful and rejected migration,
missing source, encoded filename characters, Unix non-UTF8 paths and a symlink to
committed WAL data. The WAL case invokes a child that exits with code 86. A bounded
wait prevents that fixture from hanging the verifier. The rollback journal case
also checks that rejection leaves the complete source file inventory unchanged.
Logs: `rom-dev:/var/tmp/rom-sqlite-offline-red.log`,
`/var/tmp/rom-sqlite-journal-red.log` and `/var/tmp/rom-sqlite-offline-green.log`.

The dependency change reuses pinned `url` 2.5.8. Its `std` feature is explicit,
because the path conversion must compile outside workspace feature unification.
No third-party package version changes. The lockfile adds only dependency edges
from `rom-sqlite` to `url` and from `rom-demo` to `rom-backup`.

## Final verification and review

The coordinator ran these checks in the native `rom-dev` container with Rust
1.99.0. All commands completed with exit code zero on the combined source changes.

| Command | Evidence |
| --- | --- |
| `./demo/verify` | `/var/tmp/rom-reference-upgrade-demo-verify.log`: demo formatting, Clippy, tests, SQLite/redb TCP smokes, server reopen/SIGINT and rustdoc. |
| `cargo check --locked -p rom-sqlite` | `/var/tmp/rom-reference-upgrade-sqlite-standalone.log`: adapter build without demo feature unification. |
| `./scripts/check` | `/var/tmp/rom-reference-upgrade-full-check.log`: strict OpenSpec, workspace formatting, Clippy, tests, docs, core dependency isolation, consumer, compile fixtures, auth and identity verifiers. |

The full run includes five demo upgrade parent tests and five SQLite offline-read
parent tests. Each process-exit parent invokes its ignored child fixture explicitly.
The application upgrade parent covers both SQLite and redb. The SQLite path tests
ran on Linux; this report does not claim executed Windows path coverage.

The shared target was `/var/tmp/rom-release-relations-tests-target` in the container.
Both `CARGO_PROFILE_DEV_DEBUG` and `CARGO_PROFILE_TEST_DEBUG` were zero.
The source was baseline `9ff10f7` plus the changes committed with this report.

Independent review covered application composition, shared transition rules,
callback compatibility, receipt codec/authority and source-file handling. No
blocking finding remained after the SQLite and explicit dependency-feature fixes.
Root module facades remain intact. The application has no separate repositories
or transport controllers for its two Checkout schema versions.

Stage 2.4 is complete. The next stage integrates the measured selector and native
index lifecycle. The [integration review](maintained-selector-integration-review.md)
records proposed boundaries and test requirements; it is not an implementation.

## Limits

These tests exercise offline Resource schema migration in the maintained native
format. They do not run an older ROM binary. Older native and archive formats
have separate conformance tests. Process exit does not prove power-loss recovery.
The demo uses synthetic local identity; production identity remains in stage 4.
This journey does not provide online cutover or back up external blob contents.
