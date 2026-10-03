# Maintained backup and restore implementation plan

> Execute inline using superpowers:executing-plans; coordinator owns the independent integration review.

**Goal:** Consistent complete database snapshots and explicit fresh-destination recovery for both maintained adapters.
**Architecture:** `rom-backup` provides private bounded checksummed logical archives and structural validation, without either database driver. Each adapter exports all native tables from one coherent read transaction and restores one fresh staged database transaction. The generic application Storage path remains unchanged.
**Spec:** integrated-MVP task4.2 and coordinator instructions. Baseline3057e03 includes protected deletion/configuration metadata. Rust1.99, existing pinned serde/sha2; no real external effects.

## API and constraints

`Sqlite::backup_to(path, BackupLimits)` / `Redb::backup_to` return a payload-free Manifest. Before it makes a new database, `restore_from(archive, fresh_destination, limits)` validates the source format, backend, checksum, counts, and structural invariants. It returns the adapter with the archived storage limits. It refuses an existing destination. No cross-backend migration or automatic production-format upgrade.

Unix0600 archives and staged database files, owned0700 staging directories, host-trusted parent directories, no protected values in errors/logs. Publish completed files with no-overwrite hard-link semantics and fsync. SQLite exports native transactional table reads including committed WAL content; it closes/checkpoints the restored staging database before publication. No copying a live SQLite database file.

Restore rotates only the journal generation. It fences and resets leased work to recoverable Pending. It preserves attempts, delivery uncertainty, frozen inputs, and idempotency identities. Existing Done/Stopped outcomes persist. Source database and archive remain unchanged. External blob bodies, remote notifications, application functions/configuration and credentials are not database archive contents; Manifest/README explicitly say so.

## Task1: archive contract

- [x] Write failure tests for private complete roundtrip, corrupt/wrong-format archive and destination overwrite controls.
- [x] Implement bounded collector, checksummed format, private staging/publication and structural invariants in new maintenance crate.
- [x] Add minimal StorageState/WorkLedger maintenance helpers for validation, restored limits and generation/lease reset.

## Task2: native adapters

- [x] Implement coherent SQLite native read transaction and redb ReadTransaction exports.
- [x] Restore all rows/receipts/retained events/effects/state in one native transaction; fsync/checkpoint/close before fresh publication.
- [x] Exercise actual reopen, protected metadata, lost-reply receipt replay, frozen pending work and old-cursor HistoryGap against both adapters.
- [x] Cover concurrent writers and corrupt/oversized/private-mode/overwrite/wrong-backend negatives.

## Task3: evidence and review

- [x] Document API, archive completeness, offline cutover requirements, retained limits, memory buffering and external-side-effect ambiguity.
- [x] Run full verifier, package metadata check, commit and hand off exact limitations for independent review.
