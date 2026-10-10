# ROM database maintenance

`rom-backup` implements the private logical archive used by `rom_sqlite::Sqlite` and `rom_redb::Redb`. It adds no driver dependency to `rom`. Native hosts call these synchronous methods on their bounded blocking executor:

```rust,ignore
let manifest = database.backup_to("/private/backup.rom", rom_backup::BackupLimits::default())?;
let restored = rom_sqlite::Sqlite::restore_from(
    "/private/backup.rom", "/private/restored.db", rom_backup::BackupLimits::default(),
)?;
```

The redb methods have the same signatures. The destination must be new. Restore refuses existing paths and symlinks.
New candidate archives identify archive format 7 / database format 10. Archives remain backend-specific.
The reader also accepts archive-7/database-9 through the same canonical validator and retains the original manifest marker.
This is not a cross-backend migration or automatic native format upgrade.
Archives include canonical descriptors, retry epoch boundaries, operator receipts, delivery profiles and the validated live reference graph.

To upgrade an older archive, call `rom_backup::upgrade_v1_archive(source, destination, backend, descriptors, limits)`. Supply explicit descriptors for every stored kind. For catalogued archives, use `upgrade_v2_archive(source, destination, backend, limits)`, `upgrade_v3_archive`, `upgrade_v4_archive`, `upgrade_v5_archive`, or `upgrade_v6_archive` for source archive versions 2, 3, 4, 5, or 6 respectively. These operations preserve the source.
Native adapters also provide `upgrade_from` for formats 3 through 9 into a fresh format-10 destination.
Archive-4/5/6 and native-format-6/7/8/9 upgrades preserve retry epochs and receipt origins unchanged.
Native format 10 separates retained Work records from bounded metadata. Canonical archives preserve the logical state representation.
The 0.1.0 candidate adds frozen delayed-work eligibility. Accepted 0.0.3 remains archive format 6 / database format 8.
Full release acceptance remains open. Earlier sources require epoch-zero metadata.

Use [offline retention](../../docs/retention.md) to reclaim expired history. It shares
dependency checks across adapters and requires explicit host retry boundaries.
Keep a trusted retry fence outside rollback backups before activating new boundaries.

Use a typed `MigrationPlan` and native `migrate_from` to change Resource fields.
See [Resource migrations](../../docs/resource-migrations.md). Receipts preserve
their original request codec version. Older maintenance tools reject the new format
instead of silently dropping that metadata. Unfinished work needs explicit compatibility
validation; action and delivery payloads remain frozen.

## What is included

One coherent native read transaction captures complete Resource rows, including private deletion authorization and configuration provenance. It also captures retained journal events, all retained idempotency receipts, effect intentions, work records, causal counters, delivery uncertainty and persisted limits. Values are preserved without application decoding or normalization. The payload-free manifest contains counts, backend/format and explicit exclusions.

SQLite reads the database and its WAL through SQL within the transaction. Restore writes one fresh transaction, then checkpoints and truncates its WAL. It switches the staging database to DELETE journaling and closes it before publication. redb exports a native ReadTransaction and restores an immediate-durability transaction, closing before publication. Both use a private sibling staging directory and a no-overwrite hard link; no live database files are copied.

New archive/database files use Unix mode 0600. Staging directories use mode 0700. Use trusted parent directories on a local filesystem supporting hard links and directory fsync. Non-Unix private archive creation/reading is unsupported. Protected values appear in archive contents and must remain private. SHA-256 detects accidental corruption; it is neither encryption nor an authenticity signature. Protect archive access and distribution separately.

The default archive limits are 128 MiB serialized bytes and 400,000 aggregate records, including work. Native text is charged before decoding/appending; output serialization and input size are bounded. The implementation buffers the logical snapshot and encoded archive. Thus, peak heap use exceeds the byte limit by decoded-object/serialization overhead. Hosts must budget executor and memory capacity; this is not a streaming multi-terabyte backup API.

## Recovery procedure and completeness

1. Choose a trusted archive. Retain the original source and archive unchanged. Restore into a new path with the matching adapter. Before publication, restore validates the input checksum, sizes, counts, keys, receipts, event/state consistency and work counters. Invalid archives never replace existing data.
2. Restore the application's matching descriptors, actions, reaction/channel functions, authority configuration and secrets separately. Those native functions and credentials are not archive contents.
3. Restore **external blob objects and their versions** with the blob backend's separate backup process. Verify those objects and versions. Database Blob Resources and references alone do not contain those bytes. Neither archive manifest nor API reports external objects complete.
4. Before the switch to the restored host, quiesce the original host. Stop its workers. Fence external clients. An online snapshot is coherent but can precede later original-host commits or external effects. If both hosts run, their resource and work histories diverge. Generation rotation does not fence another process or a remote receiver.
5. Resume using the archived storage/work limits. Supply a clock consistent with persisted work timestamps. Age budgets persist, so old work can stop rather than execute. Reconcile notification outcomes. Use receiver-side deduplication keyed by stable delivery identity. Restore cannot undo external effects or provide exactly-once delivery.
6. Explicitly resynchronize journal consumers. Pre-restore cursors fail `HistoryGap`. Obtain an authorized new journal head. Rebuild the snapshot with the normal overlap procedure.

Restore resets leased records to Pending and increases their claim generation. Attempts, root work budgets, due times of already-pending work, frozen inputs, stable identities and Unknown delivery outcomes persist. Done/Stopped remain terminal. Old lease claims fail even before their original expiry. A recovered exhausted record can perform receipt resolution but cannot regain an execution budget.

Files are synced before publication and the destination parent is synced afterward. A post-publication fsync failure returns `Unknown`. A complete destination can already exist. Never remove it automatically. Do not retry into it without inspection. Inspect/reopen that path. Abrupt process termination can leave a private `.rom-maintenance-*` directory. After confirmation that no maintenance process owns the staging directory, an operator can remove it. Source data is unchanged. No automatic retention, encryption, incremental backups, external-object orchestration, or destructive replacement is provided.

## Experimental ledger compatibility

Work admission reserves space for maximum-width mutable counters/timestamps, lease/resolution states and delivery outcomes. Thus, accepted work can progress within its persisted byte cap. Older experimental ledgers remain compatible only when they have this headroom. Insufficient headroom produces `Unsupported` during native open, work update or archive validation/restore; ROM does not silently raise persisted limits. There is currently **no supported automatic migration or in-place capacity reconfiguration tool**. Preserve the source/archive. Before you use such a store with this version, arrange a separately reviewed migration. Do not edit private JSON or drop pending obligations as a recovery shortcut. Terminal records retain their reserved capacity until an explicit future retention mechanism exists.
