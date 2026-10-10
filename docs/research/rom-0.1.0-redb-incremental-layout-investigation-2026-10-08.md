# redb incremental retained-work layout investigation

Date: 2026-10-08. Status: source-only seam proposal.
No product edits, native commands, speedup measurements, or final implementation review occurred.
The [shared contract review](rom-0.1.0-incremental-work-contract-independent-review-2026-10-08.md) supplies the semantic constraints.

## Current layout and integration cost

The current native format is 9. `format.rs` takes its marker from `rom_backup::STORAGE_FORMAT`.
Nine ordinary tables contain Resources, receipts, events, effects, metadata, state, schemas, outgoing references, and incoming references.
The `rom_state` table contains exactly one `state` JSON value.
That value includes the entire WorkLedger, OperatorLedger, bounded journal, retry epochs, generation, limits, and native dependency counts.

`storage.rs::retry_epochs` currently decodes that whole value to obtain its epoch header.
Reaction updates decode, clone, validate, serialize, and replace the complete value inside one Immediate write transaction.
`commit.rs::commit_bundle` does the same before writing Resources, references, receipt, event, and effects.
`operator.rs` reads or writes the complete metadata for projection and control.
This source establishes retained-history amplification. It does not quantify internal codec or synchronization shares.

All writer paths use `commit_gate` and `available()`. A failed native commit sets the uncertain flag and returns Unknown.
This gate covers the interval after redb releases its internal writer lock.
Keep the gate and uncertainty fence across every new table write and native commit outcome.
Do not permit another writer to treat a failed acknowledgement as a clean rollback.

## Proposed concrete split

The following table names and Rust support names are proposals, not implemented interfaces.

| Native table | Proposed key/value | Responsibility |
| --- | --- | --- |
| `rom_state` | `state` / core-defined slim header JSON | Storage limits, epochs, generation, receipt/effect counts, bounded journal and operator state; no WorkLedger record map |
| `rom_work_header` | `header` / core-defined header JSON | Frozen optional ReactionLimits, record/root counts, exact current/reserved canonical WorkLedger byte totals |
| `rom_work_records` | work ID / complete WorkRecord JSON | Frozen payload and current lifecycle state, attempts, generation, revision, due and delivery |
| `rom_work_roots` | root ID / core-defined RootAccount JSON | Used attempts, immutable epoch, member count and incomplete-member count |
| `rom_work_active` | work ID / core-derived event time | Pending or Leased records only; no terminal or held-reconciliation records |
| `rom_work_members` | `(root ID, work ID)` / fixed marker | Whole-root retention and reconstruction consistency |

Use existing redb `TableDefinition` key forms. Existing reference tables already demonstrate tuple-key native tables.
Add a handle-to-work-ID index only when the operator targeted-read contract is defined.
Do not let a hash collision become unique authority. Preserve ambiguous-handle rejection.
An epoch-to-root index is useful for retention but is not required for ordinary single-record updates.

The exact small header encoding must be defined by shared core support, not ad hoc JSON surgery in redb.
StorageState's private fields currently prevent a safe independent adapter implementation of that split.
Add an opaque split/reassemble support contract while preserving existing public StorageState paths and canonical archive encoding.
Represent absence of ReactionLimits exactly. Do not reconstruct arbitrary defaults during import.

Keeping OperatorLedger and bounded journal in the slim header is the smallest retained-work correction.
They remain possible separate costs. Do not claim this split makes every operation independent of operator receipts or journal size.
Follow measured attribution before broadening that change.

## Transaction-local read and write seam

Add `work_records.rs` for native table access and a named transaction adapter for the shared WorkRead interface.
It borrows one redb read or write transaction. It must not start a second transaction during a transition.
Copy an accessed value before releasing its AccessGuard. Do not retain native table guards across mutation or callbacks.

Within the existing writer gate:

1. Check availability and begin the write transaction with Immediate durability.
2. Read the coherent slim state and work header. Perform receipt arbitration before a fresh causal-claim check.
3. Use the shared overlay to validate affected records, roots, accounting, and candidate selection.
4. Write complete affected records, roots, header and indexes in the same native transaction as the existing bundle writes.
5. Release table guards, run the existing before-commit checkpoint, and commit once.
6. On native commit failure, set uncertain and return Unknown. Preserve the existing post-commit lost-ACK checkpoint.

Pre-commit errors discard all table writes. Incremental writes must join the existing fault-injection ordinals.
Standalone reaction updates need equivalent per-write injection coverage, not only Resource-bundle coverage.
A returned WorkClaim must use the finalized revision from the shared delta.
An exact replay receipt must not rewrite a WorkRecord or increment its generation.

`retry_epochs()` should read only the slim authoritative header.
`reaction_records()` intentionally reads all WorkRecords in ID order for its full public export.
Ordinary Claim must not call that full export. Operator snapshots remain bounded full observations until a distinct targeted contract exists.

## Minimum Claim index and its limits

An ID-keyed active table immediately excludes retained Done, Stopped, and AwaitingReconciliation history.
Seeking ascending active IDs and rechecking event time preserves the current Claim order and atomic prefix.
Its cost is proportional to examined active entries, including unrelated future Pending entries.
It is a useful first implementation for the selected retained-Done workload, not a constant-time eligibility index.

Measure active entries examined, records decoded, and records changed for each Claim.
If future Pending history dominates, add an ordered augmented index or another measured native selection strategy.
A `(time, ID)` index alone does not cheaply guarantee the smallest eligible ID.
Do not change fairness to due order or truncate preceding expiry/reconciliation effects to obtain a cheaper query.

Core derives the event time and membership from exact policy. Adapters persist those derived values atomically.
Pending event time includes earlier age expiry of delayed work; Leased event time is lease expiry.
At equal age/due boundaries, preserve ordinary eligible-claim behavior.

## Format 9 conversion and archive reconstruction

Introduce an explicit new native marker, proposed format 10, only with coordinated backup and SQLite ownership.
The current storage-format constant is shared. A redb-only bump must not silently change another adapter's accepted marker.
Archive version 7 and native format are distinct. Unchanged canonical serialization does not alone settle manifest compatibility.
Update the manifest reader's supported storage-format rules explicitly and retain historical archive readers.

Current `NativeFormat::Upgrade` accepts 3 through 8; it must also recognize offline format 9 for this conversion.
Format 9 continues through its complete canonical state decoder. New format reconstruction combines the slim header and native work tables.
Current maintenance assumes exactly nine tables and one state value. Replace that assumption with explicit version-specific layouts and exact table counts.
Reject missing, unexpected, malformed, or inconsistent native tables, accounting, root summaries, and indexes.

Use the existing offline NativeOwnership, read-only inspection, private recovery-copy fallback, Stage, and fresh destination publication.
Never rewrite the source marker or source records in place.
Validate the canonical predecessor snapshot before splitting it. Rebuild summaries/indexes from validated records, then validate the actual destination.
Preserve original source bytes, inode/hash evidence, and interrupted-publication evidence.

Current `restore_snapshot` calls `prepare_restore` before publication.
That changes the storage generation and recovers leases; migration using this path is not identical to preserving live native state.
Document and test that existing upgrade behavior. Compare expected prepared canonical state rather than demanding unfenced generation equality.
The source archive and original database remain unchanged.

Export builds canonical StorageState and ID-ordered WorkLedger from one coherent read transaction.
Account for work records before accumulating unbounded vectors; retain BackupLimits and Collector validation.
Full archive scans are acceptable at this maintenance boundary, not during ordinary updates.
Restore, migration, retention, and schema conversion must all write the split layout through one shared native reconstruction helper.

An old format-9 binary must refuse the new marker before writable open.
The current preflight already inspects the marker read-only, with recovery only on a private copy for dirty allocators.
Prove actual old-binary refusal and unchanged source bytes. Editing a current marker and testing the new reader is weaker evidence.

## Required checks before adoption

- Differential canonical tests preserve all five updates, enqueue, bundle composition, operator receipts, and one revision increment per changed record.
- Exact current/reserved accounting matches canonical serialization, including escaped keys, map boundaries, and zero-used roots with retained members.
- Native corruption tests reject orphan roots, incorrect counters, absent active entries, extra active entries, and wrong event-time entries.
- Claim tests preserve ascending ID, preceding expiry/reconciliation changes, and complete rollback on a later overflow.
- Real redb faults cover every new native write, before commit, committed lost acknowledgement, reopen, and the uncertain writer fence.
- Format-9 conversion and older supported conversion preserve actual Runtime/custom-codec receipt replay under current authorization.
- Archive reconstruction, restore fencing, whole-root retention, interrupted staging, and old-writer refusal retain historical evidence.
- Repeat the unchanged optimized retained-work seed and mixed load. Re-run diagnostics, public consumers, extracted packages, and full verification after source changes.

This proposal is an implementation investigation. It is not an independent approval of future code written from it.
