# ROM database maintenance

`rom-backup` implements the private logical archive used by `rom_sqlite::Sqlite` and `rom_redb::Redb`. It adds no driver dependency to `rom`. Native hosts call these synchronous methods on their bounded blocking executor:

```rust,ignore
let manifest = database.backup_to("/private/backup.rom", rom_backup::BackupLimits::default())?;
let restored = rom_sqlite::Sqlite::restore_from(
    "/private/backup.rom", "/private/restored.db", rom_backup::BackupLimits::default(),
)?;
```

The redb methods have the same signatures. A fresh destination is mandatory; existing paths and symlinks are refused. Archives are backend-specific, archive format 1 / database format 3. This is not a cross-backend migration or automatic format upgrade.

## What is included

One coherent native read transaction captures complete Resource rows (including private deletion authorization and configuration provenance), retained journal events, all retained idempotency receipts, effect intentions, work records, causal counters, delivery uncertainty and persisted limits. Values are preserved without application decoding or normalization. The payload-free manifest contains counts, backend/format and explicit exclusions.

SQLite reads the database and its WAL through SQL within the transaction. Restore writes one fresh transaction, checkpoints/truncates its WAL, switches the staging database to DELETE journaling and closes it before publication. redb exports a native ReadTransaction and restores an immediate-durability transaction, closing before publication. Both use a private sibling staging directory and a no-overwrite hard link; no live database files are copied.

New archive/database files use Unix mode 0600 and staging directories mode 0700. Use trusted parent directories on a local filesystem supporting hard links and directory fsync. Non-Unix private archive creation/reading is unsupported. Protected values appear in archive contents and must remain private. SHA-256 detects accidental corruption; it is neither encryption nor an authenticity signature. Protect archive access and distribution separately.

The default archive limits are 128 MiB serialized bytes and 400,000 aggregate records, including work. Native text is charged before decoding/appending; output serialization and input size are bounded. The implementation buffers the logical snapshot and encoded archive, so peak heap use exceeds the byte limit by decoded-object/serialization overhead. Hosts must budget executor and memory capacity; this is not a streaming multi-terabyte backup API.

## Recovery procedure and completeness

1. Choose a trusted archive; retain the original source and archive unchanged. Restore into a new path with the matching adapter. Input checksum, sizes, counts, keys, receipts, event/state consistency and work counters are checked before any destination is published. Invalid archives never replace existing data.
2. Restore the application's matching descriptors, actions, reaction/channel functions, authority configuration and secrets separately. Those native functions and credentials are not archive contents.
3. Restore and verify **external blob objects and their versions** using the blob backend's separate backup process. Database Blob Resources and references alone do not contain those bytes. Neither archive manifest nor API reports external objects complete.
4. Quiesce the original host, stop its workers and fence external clients before switching to the restored host. An online snapshot is coherent but may precede later original-host commits or external effects. Running both hosts would fork resource and work histories; generation rotation does not fence another process or a remote receiver.
5. Resume using the archived storage/work limits. Supply a clock consistent with persisted work timestamps; age budgets are preserved, so old work may stop rather than execute. Reconcile notification outcomes and use receiver-side deduplication keyed by stable delivery identity. Restore cannot undo external effects or provide exactly-once delivery.
6. Explicitly resynchronize journal consumers. Pre-restore cursors fail `HistoryGap`. Obtain an authorized new journal head and rebuild the snapshot with the normal overlap procedure.

Restore resets leased records to Pending and increases their claim generation. Attempts, root work budgets, due times of already-pending work, frozen inputs, stable identities and Unknown delivery outcomes persist; Done/Stopped remain terminal. Old lease claims fail even before their original expiry. A recovered exhausted record can perform receipt resolution but cannot regain an execution budget.

Files are synced before publication and the destination parent is synced afterward. A post-publication fsync failure returns `Unknown`: a complete destination may already exist. Never remove it automatically or blindly retry into it; inspect/reopen that path. Abrupt process termination can leave a private `.rom-maintenance-*` directory. After proving no maintenance process owns it, an operator may remove that staging directory; source data is unchanged. No automatic retention, encryption, incremental backups, external-object orchestration, or destructive replacement is provided.
