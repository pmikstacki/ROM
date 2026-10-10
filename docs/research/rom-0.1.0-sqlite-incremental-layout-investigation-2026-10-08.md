# SQLite incremental work layout investigation

Date: 2026-10-08. Status: source investigation and proposed patch boundaries.

This report does not record an implemented SQLite layout change. No native command or product edit was performed for this investigation.
The root agent owns the shared `StorageState` split and reconstruction bridge. The accounting agent owns the core accounting helper.
SQLite implementation must wait for the shared engine contract and source freeze.

## Problem and evidence boundary

The actual optimized R10 profile did not admit 10,000 Resources and 10,000 retained reaction records within its unchanged 120-second runtime.
The instrumented SQLite copy contained 1,200 Resources, 1,164 Done records and 36 Pending records.
The redb copy contained 1,485 Resources, 1,400 Done records and 85 Pending records.
These are failed seed admissions, not production throughput limits or a completed mixed-load test.

The fixture measures complete `Storage` calls, including waiting and native I/O. It does not measure live JSON encoding bytes or exclusive codec CPU time.
The maintained evidence and qualifiers are in [the R10 report](rom-0.1.0-application-load-2026-10-08.md).

Source inspection identifies a repeated operation: SQLite reads and writes the complete work ledger through the singleton `rom_state` JSON.
This occurs during Resource commits and work transitions. A table split alone is insufficient if each call still reconstructs every work record.
No performance improvement is claimed before the same retained-work profile passes with new measurements.

## Current physical and logical formats

`rom_backup::STORAGE_FORMAT` is 9. SQLite uses this constant for `PRAGMA user_version` and snapshot collection.
The archive model also uses this constant. Native layout and logical archive version are currently coupled.

| Current object | Stored content | Proposed disposition |
| --- | --- | --- |
| `resources` | Resource rows and native revision projection | Preserve. |
| `receipts` | Durable mutation receipts by identity | Preserve. |
| `events` | Committed Resource rows by event identity | Preserve and retire in the existing bundle transaction. |
| `effects` | Intent records by receipt identity and ordinal | Preserve. |
| `rom_state` | Complete `StorageState`, including all work and roots | Keep the validated metadata header; move work records and root usage out. |
| `schemas`, `reference_edges` | Accepted descriptors and stored references | Preserve validation and transactional updates. |
| `query_keys`, `query_kinds`, `query_profile` | Query projections | Preserve existing rebuild and inventory checks. |

`store.rs::open_connection` checks `user_version` before WAL configuration or schema writes.
It accepts format 9 or an empty format-0 database. Other existing formats require explicit maintenance.
Open then validates the complete native snapshot and configured storage limits.

`snapshot.rs::validate_inventory` compares every non-internal SQLite object with an exact inventory.
It also requires one `rom_state` row. Additional work tables therefore require a new inventory branch.
The collector cannot treat a header with missing work as a complete canonical state.

## Minimal native split

The following names are proposed private adapter objects. They do not define new application Resources or public authoring APIs.

| Proposed object | Required meaning |
| --- | --- |
| `rom_state` | Singleton metadata header supplied by the shared typed bridge. Keep generation, retry epochs, limits, receipt/effect counts, journal state and operator metadata intact. |
| `rom_work_header` | Singleton work policy and validated accounting metadata required by the shared work engine. |
| `rom_work` | One authoritative serialized `WorkRecord` per immutable work ID. |
| `rom_work_roots` | One authoritative root-attempt usage entry per root ID. |

Keep all objects in the same SQLite database and the same transaction.
SQLite WAL does not provide cross-database atomicity for a transaction involving attached databases. See [SQLite WAL guarantees](https://www.sqlite.org/wal.html).

Use the shared typed bridge to split and reconstruct state. Do not modify serialized JSON keys in the adapter.
Today, `WorkLedger.work`, `roots` and `limits` are private outside the core module.
`records()` clones all records; `root_usage()` is crate-private. These methods are not a sufficient incremental persistence interface.

The bridge must retain exact records, policy, roots and accounting. It must reject duplicate IDs, mismatched identities and invalid root totals.
Persist accounting with the records that it describes. Do not repair inconsistent accounting silently during open.

Avoid new SQL scheduling projections in the first patch unless the shared engine requires them.
If a later projection stores `u64` deadlines, generations or positions, use checked conversion or an exact encoding.
SQLite signed integers cannot represent every Rust `u64`; an unchecked cast changes the contract.

The metadata header still contains a bounded journal and operator receipts.
Writing that header can remain expensive. Measure the first correction before proposing a second journal or operator-table optimization.

## Exact patch surfaces

| Existing module | Required narrow change |
| --- | --- |
| `store.rs` | Introduce the explicit native-layout marker and new tables. Validate complete reconstructed state before accepting the connection. Preserve native ownership lifetime and current open refusal. |
| `persistence.rs::state/save_state` | Replace complete work JSON reads and writes with the typed header/work/root boundary. Centralize serialization and native error mapping. |
| `persistence.rs::commit` | Persist the shared engine's bounded work/root changes and header in the existing Resource bundle transaction. Preserve every receipt-first and revision check. |
| `persistence.rs::reaction_update` | Apply the same work engine transition and persist only its authoritative changes within one IMMEDIATE transaction. |
| `persistence.rs::retry_epochs/reaction_records` | Read the header directly for epochs; reconstruct bounded work output only when the contract requires all records. |
| `operator.rs` | Preserve receipt replay and current operator authorization boundary. Persist operator changes, work changes and metadata atomically. |
| `snapshot.rs` | Read the new inventory coherently, charge physical bytes, reconstruct canonical state, and run existing snapshot validation. Keep a separate format-9 reader. |
| `maintenance.rs::restore_snapshot` | Write the split state into the fresh stage through the same bridge. Recollect and validate before publication. |
| `upgrade.rs` | Add an explicit format-9 source conversion into the new native destination. Keep the source read-only and unchanged. |
| `migration.rs`, `retention.rs`, index rebuild | Route canonical reads and fresh publication through the new collector and split writer. Preserve reference and query validation. |

Use private named modules for schema, reads and transactional writes. Keep `lib.rs` as a facade and retain public `Sqlite` paths.
Do not create separate controllers for commit, reaction and operator persistence that duplicate work transition rules.

## Transaction and error boundaries to preserve

`commit` currently starts an IMMEDIATE transaction, loads metadata and checks the existing receipt before revision arbitration.
It checks both the requested retry epoch and any completed-work claim.
A replay checks the stored receipt epoch and fingerprint, then returns without new state, work, events or effects.

For a new receipt, the adapter checks the expected revision and unchanged-mutation constraints before applying metadata changes.
`StorageState::bundle` validates the completed claim, finishes that work, checks receipt/effect limits, enqueues reactions, and retains journal events.
It validates the resulting work epochs before publishing the candidate state. Preserve this sequence and its errors.

Keep Resource rows, references, query projections, receipts, effects, events, work, root usage and metadata in one transaction.
An IMMEDIATE transaction obtains the write transaction at its start and can return `SQLITE_BUSY`. See [SQLite transactions](https://www.sqlite.org/lang_transaction.html).

Keep test checkpoints before commit and after commit acknowledgement.
A rejected native write is `NotCommitted`; commit uncertainty or a lost acknowledgement remains `Unknown`.
Do not publish an in-memory candidate after rollback. After an uncertain native commit, do not use an unverified candidate for later arbitration.
Any engine cache must share connection synchronization and have an explicit recovery rule before another operation.

`NativeOwnership` excludes cooperating ROM processes on trusted local Linux storage.
It does not authorize arbitrary external writers, live replacement, aliases or network filesystems.
A proposed cache cannot widen this deployment contract or substitute for durable receipt checks.

## Canonical reconstruction and conversion

Reconstruct full canonical `StorageState` during open validation, backup collection and fresh restore validation.
Do not reconstruct it for every incremental transition unless required for correctness; that would retain the original full-ledger read cost.

`Collector::new` currently accepts complete state JSON. Reuse its validation after typed reconstruction, or add one shared typed collector entry point.
The collector must retain all work and operator counts, native byte budgets, event identities, journal order and reference validation.
Do not export an incomplete header as a valid archive.

Prefer a SQLite-native layout version 10 while retaining logical archive format 9 if canonical serialization is unchanged.
This requires separating the adapter's native marker from the shared archive constant at each SQLite call site.
If the logical serialized model changes, make that a separate shared archive-version decision with redb compatibility tests.
Do not bump a shared constant implicitly or continue to label the new native tables as format 9.

Add a format-9 collector that uses the existing physical inventory and complete `rom_state` representation.
Convert its validated canonical snapshot into a fresh format-10 stage through `restore_snapshot`.
Keep the current formats 3–8 upgrade routes. Format 9 needs no legacy field approximation.

Preserve `prepare_restore`: fence old cursor generations and in-flight claims while retaining work identities and receipts.
Preserve stage validation, WAL checkpoint, engine close, publication and directory synchronization.
Failure before publication leaves the fresh destination absent. Publication uncertainty retains its existing classification.
SQLite has specific schema-change procedures; this proposal uses ROM's existing fresh-stage maintenance path rather than an in-place rewrite.
See [SQLite schema changes](https://www.sqlite.org/lang_altertable.html).

## Smallest safe implementation order

1. Freeze the shared typed header/work/root bridge and incremental transition contract.
2. Add the private native layout, full reconstruction and validation tests.
3. Add incremental transactional writes for commit, work transitions and operator controls.
4. Add canonical backup/restore and explicit format-9 conversion.
5. Run maintained conformance, failure checkpoints and predecessor refusal tests.
6. Rebuild a source-fenced optimized fixture and repeat the unchanged 10,000-row retained-work admission profile.
7. Admit mixed HTTP traffic only after complete seed and drain admission passes.

Each intermediate stage must preserve compatibility evidence. A table-only intermediate implementation is not a performance acceptance result.

## Required acceptance cases

| Area | Maintained proof required |
| --- | --- |
| Reconstruction | Header, policy, every work record, roots and accounting reconstruct the canonical old-model state. Missing, extra and inconsistent entries fail validation. |
| Bundle arbitration | Replay, conflicting fingerprint, stale revision, stale epoch, completed claim and unchanged mutation retain the same results and zero rejected side effects. |
| Work lifecycle | Claim, materialization, retry, delivery, reconciliation, compensation and terminal records retain IDs, attempts, generations and root usage. |
| Transaction failure | Inject each work/root/header write failure. Reopen and verify no partial Resource, receipt, event, effect or work state. |
| Unknown acknowledgement | Recover the durable receipt after post-commit failure without another mutation or lost work completion. |
| Operator controls | Replay is unchanged; rejected controls do not consume budget, replace claims or append receipts. |
| Archive | Compare canonical format-9 exports before and after native conversion. Validate byte/count limits and restore fencing. |
| Predecessor writer | An actual preserved format-9 binary refuses format-10 open before writes. Verify source hash, inode and sidecars separately. |
| Maintenance | Upgrade, migration, retention and index rebuild preserve source bytes and publish only a validated fresh destination. |
| Load | Repeat the same 10,000 Resources and 10,000 retained records, limits, optimized profile, finite runtime and cgroup. Record call timing and allocation without claiming isolated codec cost. |
| Whole application | Repeat the relevant current-source R8 restore and actual identity/receipt/work journeys after layout changes. |

This investigation proposes SQLite surfaces only. It does not grant product ownership, change the engine contract or establish release readiness.

## Source references

- `crates/rom-sqlite/src/store.rs`, `persistence.rs`, `operator.rs`, `snapshot.rs`, `maintenance.rs`, `upgrade.rs`, `migration.rs`, `retention.rs`.
- `crates/rom/src/storage_state.rs`, `storage_state/work.rs`, `storage_state/retention.rs`, `storage_state/operator.rs`.
- `crates/rom/src/reaction_work/ledger.rs`, `control.rs`, `model.rs`; `crates/rom/src/operator/receipts.rs`.
- `crates/rom-backup/src/model.rs`, `collector.rs`, `native_ownership.rs`.

These source findings describe the inspected pre-integration layout. Later accounting or bridge edits need a fresh source review and witness.
The report follows the repository's STE writing guide. It does not claim certified ASD-STE100 compliance.
