# Reference upgrade implementation plan

> **For agentic workers:** Use superpowers:executing-plans for the assigned task.

**Goal:** Prove the reference application can migrate and restore before recovering pending work.
**Architecture:** Versioned Resource declarations share application composition.
A host module calls public native maintenance APIs. Tests use actual process exit.
**Tech Stack:** Existing Rust 1.99.0 workspace, SQLite, redb and folder blobs.
**Spec:** `docs/superpowers/specs/2026-10-03-reference-upgrade-design.md`.

## Constraints

Keep `lib.rs` and `mod.rs` as facades. Preserve public app paths through exports.
Use public ROM operations for all data mutation. Maintenance requires fresh
destinations. Keep source/evidence and active build caches. No remote credentials.

## Tasks

- [x] Evolve Checkout to version 2 and keep a frozen version-1 codec/profile.
  Files: `demo/src/compensation.rs`, its named helper modules, `application.rs` and
  `upgrade/legacy.rs`. Test default restrict, old-codec replay and shared transition rules.
- [x] Add `demo/src/upgrade.rs` with `prepare(redb, database, objects)`,
  `recover(redb, source, migrated, archive, restored, objects)` and `run(redb)`.
  Add CLI dispatch and a direct `rom-backup` dependency. Reuse reference prepare/recover.
- [x] Add `demo/tests/upgrade.rs`: both command paths, actual child exit before
  maintenance, source preservation, exact receipt replay, attachments and negative
  migration publication. Test invalid command arguments without side effects.
- [x] Independently review the old/new profile and maintenance boundary. Run
  `./demo/verify` and `./scripts/check`. Record evidence and update release stage 2.4.

## Review focus

An unchanged wire string must not hide a missing reference index. Use empty-stock
deletion to prove restrict. A pending old callback must retain its service identity
and version. Do not run bootstrap with changed seed payloads to fake migration.
Do not restore blob bytes from the database archive. Distinguish clean shutdown
from actual process exit. Check authorization after restore before exposing old data.
