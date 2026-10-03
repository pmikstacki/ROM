# Maintained persistence adapters

## Current maintained profile

Both adapters currently use **format 6**, with no implicit migration from earlier
experimental formats. The core-owned bundle includes state/revision, durable
receipt, journal and effect/work records in one native transaction. Journal
retention and receipt/effect/work capacities are bounded; cursor gaps and
unsupported formats fail explicitly. Runtime owns asynchronous scheduling over
these synchronous ports. Exactly one Runtime owns writes and invalidation per
deployment. Independent runtimes must not share a Storage handle. This is
unsupported host misuse; registration does not automatically prevent it.

See the current [journal](research/maintained-http-journal.md),
[reactions](research/maintained-reaction-results.md),
[channels](research/maintained-channel-results.md), and
[backup/recovery](research/maintained-backup-results.md) reports. Both adapters
provide a bounded, checksummed native backup and fresh-destination restore.
External blob bytes and original-host cutover fencing remain operator duties.
Normal deletion writes a logical tombstone. Explicit offline [retention](retention.md)
can purge an eligible tombstone after its dependencies expire.

Runtime binds canonical descriptors through `Storage::register` before intake.
New native commits require a stored descriptor. Matching receipt replay precedes
schema and reference checks. Registration retains omitted kinds and rejects
changed descriptors until an explicit migration occurs.
Custom persistence adapters must implement this method. Its default returns
`Unsupported`. Transparent wrappers must forward registration to their backing store.

Query selection now uses the [owned adapter query contract](query-adapters.md).
Its default delegates to one bounded snapshot, so both maintained adapters retain their current execution path.
Native candidate support requires the separate profile and integrity obligations in that contract.

Declared references now enforce target existence and restrict deletion inside
the native commit transaction. See [reference integrity](references.md).
Existing stores receive a complete bounded validation at open. The default budget
is 128 MiB and 400,000 archive records. Use `open_with_validation_limits` to supply
a larger host budget. Exceeded limits fail; validation never truncates success.

For a format-3, format-4 or format-5 source, use the explicit [native upgrade](native-upgrade.md) into a
fresh destination. redb also checks the format before writable open. If its header
requires recovery, that check uses a private temporary copy first. This preserves
unsupported sources and requires temporary disk space approximately equal to their file size.

For field changes, use a typed [Resource migration](resource-migrations.md).
Format 6 protects receipt codec versions and retry epoch boundaries from older
maintenance tools. New commits bind an explicit replay version. Migrated receipts keep
their original version and use a registered legacy codec for retry.
Transparent Storage wrappers must also forward `retry_epochs` when their backing
store supports nonzero epochs. Its default describes an epoch-zero-only store.

Run `cargo test -p rom-storage-conformance --locked` from the repository root.
The sections below preserve the original format-one milestone and its test
results. The current profile and linked reports supersede that milestone's
limitations for journal/worker/backup features. Machine power-loss and multiple-writer
certification remain outside the evidence.

## Historical foundation milestone

The maintained `rom-sqlite` and `rom-redb` crates implement the same core-owned `Storage` contract. Resource remains the only application domain entity. A `Bundle` contains an expected revision and a receipt with the canonical action fingerprint and resulting row. It also contains a changed flag and zero or more effect intentions. Driver types stay outside `rom`.

## Public boundary

- `Sqlite::open(path)` / `Redb::open(path)` open or initialize the versioned adapter format. Share one handle using `Arc`. The verified runtime deployment profile is one ROM runtime owning writes.
- `Storage::load(&Key)` and `receipt(identity)` return authoritative stored values.
- `Storage::snapshot(kind, max_rows, max_bytes)` returns rows in stable ID order, including tombstones. Limits charge the UTF-8 JSON encoding of each **complete Row**, including key and revision. Overflow returns `Error::TooLarge`; it never returns truncated success. Before adapters deserialize or append each row, they validate row count and cumulative byte size with checked arithmetic. Empty kinds return an empty vector even with zero limits. This is a serialized-size budget, not a precise heap/driver-cache memory limit.
- `Storage::commit(&Bundle)` atomically arbitrates revision and action identity, then commits row, event, receipt and every effect intention. A same-identity, same-fingerprint retry returns the previous receipt; a changed fingerprint returns `IdentityMismatch`. A stale expected revision returns `Conflict`. Core creates the fingerprint. A trusted native caller cannot substitute a different canonical meaning under a fraudulent fingerprint and expect the adapter to reconstruct action semantics.
- A no-op must exactly match the existing row/revision and contain no effects. It stores only a receipt. An invalid revision/no-op bundle returns `NotCommitted`. The shared revision profile is `0..=i64::MAX`; additions use checked arithmetic.
- `counts()` and `intentions()` are administrative inspection methods, not authenticated application APIs. `intentions()` is presently a full scan; a bounded durable worker API is separate work. Event inspection used by tests is not a cursor-based, ordered-history API.

These are synchronous adapters. The host must run storage work through ROM's bounded blocking-I/O execution boundary. Creating an adapter does not create a runtime or per-Resource executor.

## Format and failure policy

Format one stores JSON `Row`, `Receipt` and `Intent` values. SQLite records its version in `PRAGMA user_version`; redb uses `rom_metadata["format"]`. New empty databases initialize format one. Before any ROM application schema/data write, adapters reject unknown versions and nonempty databases without the marker as unsupported. No implicit migration from disposable prototypes or older unversioned maintained snapshots occurs. Raw engine open/recovery may still perform engine housekeeping.

SQLite uses WAL with `synchronous=FULL`. redb 4.3.0 transactions explicitly use `Durability::Immediate`. redb compound keys separate `(kind,id)` and `(identity,ordinal)` without ambiguous string concatenation. Changed bundles emit one event containing the resulting row; effects are stored in input order. This preserves the current foundation contract and does not introduce journal cursor/retention semantics.

Both adapters report `Unknown` when commit acknowledgement is uncertain. After an actual engine commit error, redb conservatively marks the handle unavailable. Dispose of all shared handles. Reopen the database. Before you retry effects, resolve the stable identity. Its commit gate also covers the interval between the engine releasing its writer lock and ROM recording uncertainty. Injected lost acknowledgements occur after a successful engine commit and return `Unknown` without pretending rollback.

redb's own commit documentation says errors other than a poisoned transaction can already have durable effects and need reopening. ROM conservatively classifies every engine commit error as uncertain. The selected crate supports Rust 1.90 and is MIT OR Apache-2.0 licensed; ROM tests it on its declared 1.99 floor. Sources: [redb Database](https://docs.rs/redb/4.3.0/redb/struct.Database.html), [redb transaction source](https://docs.rs/redb/4.3.0/src/redb/transactions.rs.html), and the retained workspace lockfile. Dependency advisory review remains a separate release gate.

## Test support

Both adapters expose `on_commit(Option<Arc<dyn Fn(usize) -> Result<()> + Send + Sync>>)` only with the explicit `test-support` feature. Checkpoints are `1..` after each actual write, `0` immediately before commit, and `usize::MAX` after the engine commit returns. Returning an error before commit aborts the transaction; an error after commit becomes `Unknown`. The callback can terminate a dedicated subprocess without Rust destructors. Thus, restart tests exercise real files rather than an in-memory clone.

SQLite's original `inject_fault(1..=5)` helper is also gated by `test-support`; the external consumer enables that feature only as a dev dependency. Ordinary adapter builds expose no fault injection API. Callbacks are test instrumentation and must not be enabled as a host extension or arbitrary production callback mechanism.

## Executed acceptance evidence

The shared source is `tests/persistence/tests/shared.rs`; format/redb-initial regressions are in `tests/persistence/tests/conformance.rs`. The same fixtures and assertions run on actual SQLite and redb files:

| Test | Boundary exercised |
| --- | --- |
| `shared_atomic_bundle_and_noop_identity_validation` | Exact state/revision/receipt/event/two-effect values, repeat identity, mismatched identity, no-op and invalid no-op. |
| `shared_rollback_after_every_write_and_before_commit` | Failure after state, event, receipt, first effect, second effect and before commit leaves the entire update absent; retry succeeds. |
| `shared_lost_commit_acknowledgement_reopen_resolves_original_identity` | Real commit succeeds, acknowledgement reports unknown, handle closes, reopened same identity produces no extra transition/effect. |
| `shared_revision_race_arbitrates_one_transition` | Concurrent threads with different identities and the same expected revision produce one winner and one conflict. |
| `shared_same_identity_race_returns_one_receipt` | Concurrent identical requests return the same receipt with one complete bundle. |
| `shared_bounded_snapshot_exact_limit_overflow_and_kind_isolation` | Exactly-at-limit success, row/byte overflow rejection, stable order and separate kinds. |
| `shared_snapshot_rejects_oversized_encoded_row_before_deserializing` | An oversized malformed stored row returns `TooLarge` under the small budget and `Storage` only when its bytes fit. |
| `shared_delete_tombstone_and_invalid_revision_preserve_contract` | Invalid revision leaves no update; deletion stores the tombstone consistently. |
| `shared_core_typed_actions_work_without_application_repositories` | The same derived Resource and ordinary core Runtime create/replay/replace actions use either adapter. |
| `shared_actual_subprocess_exit_at_every_write_and_commit_boundary` | Fourteen real child processes exit without destructors: seven checkpoints on each adapter. Parent reopens and verifies all-before-commit absent / after-commit complete, then stable-key retry. |
| `shared_missing_format_marker_on_nonempty_database_is_not_implicitly_migrated` | Reject unversioned nonempty files in both engines. |
| `redb_unknown_format_rejected_without_application_table_writes` / `sqlite_rejects_future_format_before_schema_writes` | Future format rejected without new ROM tables or overwriting the marker. |
| `redb_atomic_bundle_available_through_storage` | New redb adapter satisfies the existing core Storage boundary. |

During development, the future-SQLite-format test first failed against the maintained baseline. The initial redb contract test failed against the unimplemented adapter. Both subsequently passed. The complete `./scripts/check` run passed these checks:

- Four OpenSpec changes and formatting.
- Workspace Clippy with warnings denied.
- 32 integration tests plus one rustdoc test.
- Warning-free docs.
- No-default-feature core build and driver/transport dependency exclusion.
- Five expected compiler failures.
- The renamed external consumer.

The subprocess fixture is marked ignored during normal discovery but is explicitly executed fourteen times by its passing parent test.

Rerun from the host:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/transport-trials && CARGO_TARGET_DIR=$PWD/target-redb ./scripts/check'
```

Focused conformance: `cargo test -p rom-storage-conformance --locked` from the workspace. These tests do not prove behavior under power-loss/storage-device faults, multi-process writers or disk-full engine commit failure. They do not prove backup/restore, bounded global receipt storage, authenticated journal or worker claim/checkpoint protocol. Process exit tests preserve the operating-system/filesystem process environment and therefore are weaker than machine failure tests.

## Combined main-branch verification

The coordinator integrated bounded asynchronous core `6a7a19f` with the
adapters and reran the complete verifier at main `d228c13` on 2026-10-02.
All 54 integration tests and one doctest passed, alongside compiler fixtures,
Clippy, docs and no-default-feature checks. The 17 persistence tests include
three additional format regressions: SQLite containing only a view, a legitimate
`sqliteXlegacy` table, and redb containing only a multimap. Each must be rejected
without writing a ROM marker or tables. They failed against the preceding
adapter source and pass after `e60160e`; independent review confirmed the fixes.

The maintained rerun command is now from the repository root, not a reused
experimental worktree:

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM && CARGO_NET_OFFLINE=true ./scripts/check'
```
