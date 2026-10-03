# Resource reference integrity

Declare a relation with `ResourceRef<T>`. The same contract applies inside
nullable, optional, list and map fields. Application code needs no relation repository.

## Commit behavior

Each new commit validates its value against the persisted descriptor. Every
distinct target must have a live current row. A missing or deleted target produces
`Conflict`. No row, event, receipt, effect or work item commits for that rejection.

Deletion uses `restrict`: a live reference from another Resource blocks deletion.
Remove that reference with an authorized action, patch, replace or source deletion first.
A self-reference does not block deletion of its own Resource. Cycles between
different Resources require an explicit unlink.

Checks and index changes share the row transaction. SQLite maintains a target-first
index on its edge table. redb maintains source-first and target-first tables.
Normal commits do not scan every Resource to find incoming references.

Integrity checks do not filter by caller visibility. A hidden source can block
deletion, but the error does not disclose its identity. Authorization still applies
to the requested operation and its result.

## Registration and history

Runtime registers the complete active declaration graph before it accepts work.
Storage retains descriptors for omitted kinds. Their persisted references still
block target deletion when a later Runtime omits those source kinds.

Existing descriptors must match their stored canonical definitions. Changing a
reference into a string is a schema change, even if its JSON encoding stays the same.
Registration does not perform such a migration.

A matching receipt replays its original result without adding edges or checking
historical targets. Current Runtime authorization controls disclosure. Receipts,
events and protected tombstone values do not create live references.

## Maintenance

Native format 7 and archive version 5 store descriptors and reference edges.
Startup and backup validation compare the stored graph with current values.
Missing, extra or dangling edges fail validation. Restore validates the archive
and rebuilds both native access paths before publishing a new destination.

`rom_backup::upgrade_v1_archive` upgrades an existing format-1 archive into a new
format-5 archive. Supply explicit descriptors for all stored kinds. It preserves
rows, receipt identities, events and work records. It rejects incompatible values
or dangling references. It does not infer schemas or transform Resource values.

`upgrade_v2_archive` upgrades a format-2 archive using its persisted catalog.
`upgrade_v3_archive` and `upgrade_v4_archive` support archive versions 3 and 4.
Normal native open rejects formats 3 through 6. Both adapters provide `upgrade_from` to read
those formats and publish a new format-7 database. Supply the complete descriptor
catalog and stop source writers before cutover. See [native upgrade](native-upgrade.md).
Use [Resource migrations](resource-migrations.md) to change fields and rebuild references.

The supported deployment still has one Runtime writer. Transaction race tests
prove reference integrity; they do not establish cross-Runtime live invalidation.
