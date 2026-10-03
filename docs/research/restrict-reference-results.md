# Transactional references and module cleanup

Status: implemented and verified for the reference-integrity slice. The full release goal remains open.

## Changed behavior

Both native adapters now bind canonical descriptors before new commits. References
are extracted from declared fields, including nullable, optional, list and map values.
Target checks and restrict deletion share the existing native transaction.

SQLite uses one edge table and a target-first index. redb uses two ordered edge
tables. The core has no driver dependency. Persisted descriptors retain references
from kinds omitted from a later Runtime.

Native format 4 and archive version 2 include schema and edge metadata. Existing
stores receive bounded startup validation. Restore verifies the graph before
publication. `upgrade_v1_archive` provides an explicit archive upgrade with supplied
descriptors; direct native format upgrade and value-transforming migrations remain open.

## Evidence

The initial reference suite failed 12 cases on the previous implementation. Two
self-reference controls passed. After implementation, 22 reference cases passed
across SQLite and redb. Cases include missing targets, unlink, source deletion,
schema mismatch, omitted kinds after restart, historical replay and transaction races.
An additional SQLite regression brings the reference suite to 23 cases. A payload-only
update now has four native write checkpoints instead of seven. The adapter writes
only changed edges. This measures write operations, not end-to-end latency.

Independent review added five maintenance cases. They exercise every observed edge
write checkpoint, rollback, lost acknowledgement, backup/restore and missing indexes.
The review reproduced two defects and verified their corrections:

- Archive validation now rejects noncanonical persisted descriptors before publication.
- The shared collector counts pending work against the startup record budget.

The independent conformance and consumer run passed 193 tests. Its one ignored
child-process fixture is executed by its parent test. Log in `rom-dev`:
`/var/tmp/rom-review-final.log`.

Core extraction has five tests. Backup has six tests, including an explicit
legacy archive upgrade that preserves source bytes and refuses overwrite.
These results do not prove general schema transformation or physical power-loss durability.

Final `./scripts/check` passed with Rust/Cargo 1.99.0 and the unchanged lockfile.
It ran OpenSpec strict validation (9 changes), formatting, Clippy, workspace tests,
doctests, Rustdoc, compile fixtures, consumer execution and isolated auth/identity checks.
The log is `rom-dev:/var/tmp/rom-relations-full-check.log`. A first run found a
SQLite import that needed a `test-support` condition; the corrected full run passed.
The existing real-MinIO tests remain opt-in and were not run in this pass.

## Module boundaries

The owner required facade-only `lib.rs` and `mod.rs` files. All 14 maintained
workspace files were inspected. Implementation moved into modules for storage,
commits, references, maintenance, archives, publication, identity, transport and application behavior.

Public paths remain available through exports. Rust requires two procedural macro
entry functions at the crate root; both delegate to the expansion module.
Historical prototypes were preserved as research evidence.

Runtime and persistence now share canonical value validation. The backup collector
owns common record accounting. Native transaction operations remain adapter-specific.

## Remaining release work

Stage 2 still requires direct upgrade workflows, schema transformations, dependency-aware
retention and the reference application's complete upgrade acceptance journey.
Stage 3 optimization and stage 4 operations/package gates remain required.
The release goal is not complete.
