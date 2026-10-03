# Upgrade a native database

`Sqlite::upgrade_from` and `Redb::upgrade_from` convert native formats 3 through 6 into format 7.
The operation requires a new destination and an explicit Resource descriptor catalog.
It does not transform Resource values or infer schemas from stored JSON.

## Procedure

1. Stop the application that writes to the source database.
2. Supply descriptors that match every stored Resource kind, including kinds with tombstones.
3. Select a new destination path and explicit `BackupLimits`.
4. Call the adapter's `upgrade_from(source, destination, descriptors, limits)` method.
5. Start the application against the returned format-7 store.
6. Preserve the original source until the deployment passes its acceptance checks.

The default maintenance budget is 128 MiB and 400,000 collected records.
Rows, receipts, events, effects, work, descriptors and reference edges consume that budget.
SQLite derived query records share the same record and encoded-byte budget.
The byte limit bounds collected data; it does not bound the database's physical file size.
For formats 4 through 6, the supplied catalog must match the persisted catalog exactly.
For formats 3 through 5, receipts without a codec version receive their source catalog version.
Format 6 preserves retry boundaries and receipt origins, including absent origins.
Older maintenance tools must not process format-7 stores or format-5 archives.
Legacy epochless data starts in retry epoch zero. See [retention](retention.md)
before advancing retry boundaries.

## Preserved guarantees

The source is read through one coherent read-only native transaction. SQLite includes
committed WAL contents. Its read-only connection can use shared-memory locks; the
upgrade does not issue SQL writes against source data.

If redb requires physical recovery after a process exit, ROM recovers a private
temporary copy. It reads the recovered snapshot from that copy and preserves the
original source. This path requires temporary disk space approximately equal to
the source file size. Clean sources do not require this copy.

Rows, revisions, receipt identities, fingerprints, events, effect intentions and work
payloads retain their values. The new database adds canonical descriptors and live
reference indexes. Missing targets and incompatible values prevent publication.

The destination uses restore fencing. Its journal has a new generation. Active work
claims become pending with a newer claim generation. Attempt budgets and uncertain
delivery outcomes retain their meaning. Old cursors and claims cannot control the destination.

ROM validates the rebuilt native store before publishing it. Existing destinations
and symlinks are not overwritten. Failure before publication leaves the source
available. A process exit can leave a private staging directory. Remove it only
after confirming that its maintenance process has stopped.

Publication can succeed before its final acknowledgement fails. An `Unknown` result
requires inspection of the destination; do not overwrite it or assume rollback.

## Scope

This operation upgrades the storage format and introduces declared reference
integrity. It is not a general migration of fields or action payloads. Cross-backend
conversion, online cutover and concurrent writers are outside this operation's contract.
Use [Resource migrations](resource-migrations.md) for typed field transformations.
