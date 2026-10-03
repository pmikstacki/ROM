# Resource migration implementation plan

**Goal:** Run versioned offline Resource transformations through shared maintenance code.
**Architecture:** Typed conversions transform bounded snapshots. Adapters own native reads and publication. Exact receipt codec versions preserve retry meaning.
**Spec:** `docs/superpowers/specs/2026-10-03-resource-migrations-design.md` and release lifecycle requirements.

## Constraints

Keep `lib.rs` and `mod.rs` as facades. Preserve one Resource contract and stable
identities. Do not rewrite frozen external payloads or silently drop work. Use Rust
1.99.0 and the existing lockfile. Preserve the source and reject destination overwrite.

## Tasks

- [x] Add `#[resource(version = N)]`, default 1, with positive-u32 diagnostics and compile fixtures.
- [x] Add `Receipt::replay_version` and exact `Definition::replay_from::<Old>()` routing after current authorization; test ambiguous codecs and fresh-request rejection.
- [x] Add controlled Resource-value visitors to Row, StorageState and WorkLedger; preserve lifecycle fields and reject budget overflow without truncation.
- [x] Add `ResourceMigration`, `MigrationPlan` and shared `migrate_snapshot` in dedicated backup modules. Test versions, historical copies, errors, panics, bounds and work validation.
- [x] Add native `migrate_from` and test-support publication hooks to both adapters. Reuse readers and restore/publication code.
- [x] Run two-backend recovery/replay and interruption tests, backup/restore, independent review and the full local verifier.
- [x] Record concrete evidence and remaining stage requirements before integration.

## Review focus

An old normalized input must not be retried under the new codec. A tombstone's
protected value needs conversion too. Notification payloads with Unknown outcomes
must remain exact. Enlarged work must fit reserved lifecycle capacity. Reapplying a
step must fail before publication. Tests for each condition belong to the owning task.

Final evidence is in [Resource migration results](../../research/resource-migration-results.md).
Native format 5 and archive version 3 were added after review found that earlier
maintenance tools could silently discard receipt replay versions. Explicit upgrades
cover native 3/4 and archive 1/2. Full `./scripts/check` passed.
