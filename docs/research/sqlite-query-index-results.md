# SQLite query index integration results

Date: 3 October 2026. Base: `bedff7a`, branch `codex/release-query-index`.
This report covers maintained native execution and recovery. It does not establish measured cost coefficients or a universal speedup.
This is the original integration checkpoint. Its remaining-work list records that checkpoint, not the latest release status.
The [cost measurement report](maintained-query-cost-results.md) records later experiments and selector changes.

## Delivered behavior

SQLite now implements `Storage::query_read` through a coherent read transaction.
Whole-kind row, persisted-text byte and canonical Row-byte limits apply before planning.
Core grants the native mode only for explicit uniform read and field permissions.
Core retains residual predicates, moving anchors, ordering, page limits and final disclosure.

The adapter encodes scalar values as exact BLOB keys. Missing, null and present values remain distinct.
The encoder preserves unsigned and signed limits, finite-float equality, both zeros and UTF-8 prefix order.
One supported predicate supplies a complete candidate set. The database receives no result LIMIT.

The SQLite planner chooses the physical access path. ROM supplies no forced index hint.
Plan inspection and execution use the same statement and parameters.
The first recognizer accepts specific output from bundled SQLite 3.53.2. Unknown output selects the reference path.
Once native execution starts, errors propagate without a second read.

The shared selector compares checked relative costs with exact request and snapshot bindings.
Its current adapter weights and selectivity fractions are uncalibrated heuristics. They do not describe observed latency.
Release measurements remain necessary for broad, skewed and write-heavy workloads.

## Atomic maintenance and integrity

Format 7 has one key for each scalar field of each live Resource, including optional missing fields.
The primary key is `(kind,id,field)`; the value index is `(kind,field,encoded,id)`.
This primary order permits a direct per-Resource membership check during commit.
The earlier proposed `(kind,field,id)` order could scan a whole kind for that check.
This is a structural access-path improvement, not a benchmark result.

The same transaction commits authoritative state, reference edges, changed index keys, exact counters, generation, receipt, journal and pending work.
Unchanged field keys remain in place. Replay, conflict and rollback do not advance index generation.
Core and adapter share scalar classification through `Shape::is_scalar`; they do not duplicate wrapper rules.

Startup and backup compare all derived memberships and counters with authoritative rows and the complete persisted catalog.
Validation includes the native columns, primary order, index order, collation and partial-index status.
Physical index records share the complete native byte and record budget with logical records.
Membership validation visits one key at a time; it does not allocate a second complete key map.

Logical archives contain no physical keys. Restore, migration, retention and native upgrade reconstruct them before publication.
The public `Sqlite::rebuild_indexes_from` method also reconstructs corrupt derived contents into a fresh destination.
The source stays offline and unchanged. Known table inventory and valid authoritative records are prerequisites.
A replacement receives a fresh index identity. Publication fences old journal cursors and work claims through the existing maintenance protocol.

## Compatibility

Both maintained adapters use native format 7; the shared archive version is 5.
redb retains reference query execution. Ordinary open rejects previous formats without silently upgrading them.
Explicit upgrade accepts native format 6 and archive version 4 while preserving nonzero retry boundaries and receipt origins.
Older sources retain their existing epoch-zero checks.
Historical fixture builders now remove the new SQLite tables when they construct an actual older native inventory.

## Verification

| Evidence | Result |
| --- | --- |
| SQLite unit tests | 17 pass: scalar encodings, candidate selection, key deltas, counters, rollback, bounded derivation and physical corruption |
| Runtime query-read conformance | 11 pass, including three cases that observe actual native responses from 128-row SQLite fixtures |
| Native integration | Four pass: commit/replay/rollback/delete/reopen, cross-store query agreement and live updates, backup/rebuild, epoch-aware native upgrade |
| Recovery integration | Five pass, including a separately invoked child that exits before publication |
| Full persistence conformance | Passes, including per-write rollback and process-exit boundaries on real databases |
| Backup and redb packages | 27 tests pass, including epoch-aware archive and native upgrades |
| Shared scalar classification | The new public method test first failed on the absent method, then passed |
| Independent review | No remaining blockers after primary-key and layout-validation corrections |
| Full local verifier | `./scripts/check` passes |
| Reference application | `./demo/verify` passes on SQLite and redb, including TCP, reopen and SIGINT drain |

The actual native authorization tests distinguish explicit uniform policy from opaque callbacks.
An excluded custom decoder can be skipped only under the explicit contract; a selected invalid value still fails.
An actor-only denial occurs before storage admission. Opaque sort-field disclosure still rejects inaccessible data.
Revocation after native materialization still prevents disclosure.

Recovery tests include noncanonical native Row text and separate raw/canonical counters.
They verify correct byte subtraction on a later mutation, omitted kinds, authoritative corruption rejection and combined publication limits.
The process-exit test retains source main-file and WAL contents, then retries publication into the absent destination.
These are process failure tests, not machine power-loss certification.

The physical-layout regression first accepted a changed NOCASE index and failed.
After layout validation, it rejects changed collation, a partial secondary index and an incompatible table layout.
The initial coordinator integration attempt failed to compile while parallel native modules were still absent; it was not a behavioral failure.
A later format-6 test exposed an invalid fixture receipt count. Correcting that fixture preserved all 128 existing receipts.

## Reproduction and evidence

Run in the `rom-dev` container from `/workspace/ROM/.worktrees/release-query-index`:

```sh
export CARGO_TARGET_DIR=/var/tmp/rom-release-relations-tests-target
export CARGO_PROFILE_DEV_DEBUG=0
export CARGO_PROFILE_TEST_DEBUG=0
cargo test -p rom-sqlite --lib --locked
cargo test -p rom-storage-conformance --locked
./scripts/check
./demo/verify
```

The compiler and Cargo are 1.99.0. No dependency or lockfile change was necessary.
`Cargo.lock` SHA-256 is `cdd9d553545253494b7fb3a7e26c8de6334a454865e621a71b62a5437b702d22`.
Changes are in the SQLite index/lifecycle modules, backup/redb compatibility, shared scalar classification, conformance tests and linked documentation.

Container logs:

- `/var/tmp/rom-index-full-check-final.log`: complete local verifier.
- `/var/tmp/rom-index-demo-verify.log`: application verification.
- `/var/tmp/rom-sqlite-index-conformance.log`: coordinator persistence suite.
- `/var/tmp/rom-native-query-auth.log`: actual native authorization tests.
- `/var/tmp/rom-index-query-final.log`: final SQLite unit suite.
- `/var/tmp/rom-index-catalog-red.log` and `/var/tmp/rom-index-catalog-green.log`: catalog implementation cycle.
- `/var/tmp/rom-sqlite-index-recovery-first.log`: public recovery tests.
- `/var/tmp/rom-format7-focused.log`: archive and redb compatibility tests.

## Remaining work

The index profile is fixed and indexes every scalar field. Its write and space costs need measurement before a production recommendation.
Selection uses heuristics, not database cardinality statistics or measured ROM costs.
Stage 3.3 still requires independent and skewed workloads, allocations, memory, decoded bytes and write amplification.
Stage 3.4 retains the final review of that measured integration.
Single-writer enforcement, operational release tasks and package preparation also remain open.
The full release goal is not complete.
