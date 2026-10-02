# Maintained database backup and restore

Implemented on `codex/maintained-backup` from baseline `3057e03` in `.worktrees/mvp-library`. This is a maintained host maintenance capability for the integrated library, not a second prototype database or a completed production operations system.

## Public surface and ownership

Both adapters expose `backup_to(path, rom_backup::BackupLimits) -> rom::Result<rom_backup::Manifest>` and `restore_from(archive, fresh_destination, limits) -> rom::Result<Self>`. Calls are synchronous and must use a bounded host maintenance executor. No change to generic `Storage`, Resource declarations, application authorization, query semantics, HTTP, configuration ingestion or reaction callbacks was needed.

`crates/rom-backup` owns the logical format, bounded collector, checksum, structural validation and private file publication. `crates/rom-{sqlite,redb}/src/maintenance.rs` own native transactional export/import. Core additions are narrow `StorageState` validation/persisted-limits/restore preparation helpers and internal WorkLedger validation/fencing. The core remains driver-free and has no dependency on rom-backup. The new crate reuses the already locked SHA-256 version `0.10.9`; no new algorithm, cloud driver or notification SDK was selected.

See [the maintenance README](../../crates/rom-backup/README.md) for the executable API shape, supported filesystem contract and cutover procedure.

## Guarantees exercised

An archive captures all five SQLite logical tables / all six redb tables in a single native read transaction. It includes complete Resource values and protected deletion/configuration metadata, retained journal rows, receipts, effect intentions, pending/terminal work, causal counters and storage/work limits. Structural checks verify table keys, unique identities, revision ranges, current authoritative receipts, effect ordinals, journal order/content/floor/head and exact work attempt totals. Payloads are never passed through application codecs or public projections. The manifest is payload-free; the archive is private data.

SQLite transactions retain a coherent snapshot while concurrent writers commit; a deferred transaction starts its snapshot when it accesses the database. The implementation uses SQL reads including WAL contents, not a copy of a live main file. Its private restored database is checkpointed, changed to DELETE journaling and closed before publication. These choices follow [SQLite transaction semantics](https://sqlite.org/lang_transaction.html) and [WAL checkpoint behavior](https://sqlite.org/pragma.html#pragma_wal_checkpoint).

redb exports through `ReadTransaction`, restores with Immediate durability, then drops the database before publication. The locked implementation's read transaction captures committed state, and database drop flushes/closes the file. Sources: [redb ReadableDatabase](https://docs.rs/redb/latest/redb/trait.ReadableDatabase.html) and [redb Database lifecycle](https://docs.rs/redb/latest/redb/struct.Database.html), checked alongside the installed 4.3.0 source.

Restore validates before constructing the destination. Both drivers rebuild a private fresh database, sync it, publish via a no-overwrite hard link and sync the destination parent. Archive and database files are 0600; staging directories are 0700. Existing destinations and symlinks are refused. Hard-link availability and same-filesystem placement are explicit constraints from the [Rust filesystem API](https://doc.rust-lang.org/std/fs/fn.hard_link.html). The archive is format 1, storage format 3, backend-specific; another backend or format requires an explicit future migration.

A restored journal has a fresh generation, so a pre-restore cursor returns HistoryGap. Leased work becomes Pending with a new claim generation and immediate eligibility. Attempts, root budgets, original timestamps, frozen input, stable work/delivery identities and Unknown delivery outcomes remain intact; Done/Stopped remain terminal. Restore does not give exhausted work another execution budget. Old claim keys cannot acknowledge restored work, even before their previous lease expiry. Idempotent replay of a committed request returns its original receipt without creating another event/effect/work obligation.

## Executed evidence

Rust `1.99.0` in native `rom-dev`, host checkout mounted at `/workspace/ROM/.worktrees/mvp-library`, `CARGO_BUILD_JOBS=2`, isolated persistent target directory. No compiler compatibility below the declared floor is claimed.

The initial conformance test was executed before implementation and failed on the absent BackupLimits/adapter methods. A later negative control constructed a Resource with no authoritative receipt and consistent zero metadata: validation incorrectly accepted it. That test was observed failing, the validator was tightened, and the full verifier was rerun successfully.

Five shared tests in `tests/persistence/tests/backup.rs`, each exercising both real adapters:

- Committed-but-lost acknowledgment, actual source reopen, protected live/tombstone/provenance values, complete backup, fresh restore, exact receipt replay, old cursor/claim fencing, Unknown delivery recovery with preserved attempts, terminal outcomes, and actual restored reopen.
- Existing archive/destination protection, wrong backend, byte/record limits, bit corruption, truncation and non-private archive mode. Rejections preserve the source and avoid destination publication.
- Invalid logical table relationships rejected before archive publication, including a current row without its authoritative receipt.
- Concurrent writer/export snapshots retain complete Resource/event/receipt/effect bundles.
- Non-default persisted limits and journal retention survive restore; receipt exhaustion and existing history gaps remain enforced.

Two archive unit tests verify valid-checksum wrong format/count/metadata rejection, late destination creation racing publication, and symlink refusal. The existing full workspace additionally exercises notification receiver duplication/deduplication and restart; backup tests deliberately preserve Unknown outcome rather than claiming that external acknowledgment became transactional.

Final `./scripts/check` exited 0: strict OpenSpec validation, formatting, Clippy warnings denied, workspace tests/doctests, rustdoc warnings denied, core without derive, external consumer, driver-free core dependency check, five intended compile-failure fixtures, authentication and identity feature profiles. The full run includes the new 2 archive unit and 5 shared backup tests. Logs: native `/var/tmp/rom-backup-final-check.log`.

`TMPDIR=/var/tmp node scripts/check-packages.mjs` exited 0 with nine maintained archives, packaged licenses/metadata, an unpacked external consumer and unchanged Cargo.lock. Evidence: native `/var/tmp/rom-packaged-consumer-WMZLOz`. That packaging run preceded the last internal current-receipt validation tightening; package layout, public API and dependencies are unchanged, and the final full source verifier includes that tightening. Coordinator can repeat packaging on the combined integration tree.

## Limits and operational follow-up

The database archive is **not a complete application or blob backup**. External blob bytes/versions, application native functions, authority/provider configuration and credentials require separate restoration and verification. External notifications performed after the snapshot can be repeated after restore; a saved Unknown outcome remains uncertain. Stable receiver deduplication is still required, and exactly-once external delivery is not claimed.

Before cutover, stop/fence the original host and workers. Journal/claim generation changes only affect the restored database; they cannot stop an old process from mutating its original database or contacting a receiver. Persisted time/age policies are retained, so the recovery host needs a compatible clock. This package provides primitives and an explicit procedure, not an automated cluster fencing service.

Archive input/output and collection have byte/record bounds (default 128 MiB / 400,000 records). Logical decoded objects and encoded bytes coexist, so peak heap exceeds the serialized byte cap. Native read transactions temporarily retain their snapshot; the current same-adapter maintenance gate also delays its writers during collection. No throughput/latency or large-database performance claim was inferred from test duration. Streaming/incremental archives and storage-aware operators remain future work.

Only the Unix private-file/local-filesystem profile is implemented. SHA-256 detects corruption, not malicious forgery; there is no built-in archive encryption/signature, key management, remote upload or retention policy. A crash can leave a private staging directory for an operator to inspect and remove after proving no maintenance process owns it. A publication/reopen failure can return Unknown with a complete destination already present; never delete or overwrite automatically. Filesystem/power-loss testing beyond the adapter's existing transaction conformance has not been claimed.

The accepted core premise is unchanged. Multi-resource transactions, destructive restore, migration, external blob snapshot orchestration and workflow administration are not silently added. Independent combined-tree review and an actual deployment recovery drill remain necessary before deployment-specific readiness claims.
