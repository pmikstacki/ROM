# Native journal write amplification

## Status and recommendation

This is a read-only source investigation for ROM 0.1.0.
No implementation, Cargo command, database probe, or workload change was performed for this report.
The current format-10 source and running parent-owned trial remain unchanged.
Source inspection used HEAD `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470` plus the current working tree.

Recommend a keyed native journal with a scalar header and one shared, opaque append/retention delta.
Keep Resource, receipt, event, effects, references, query indexes, Work changes, and journal changes in the existing transaction.
Keep `synchronous=FULL`, original deadlines, ordering, callback execution, and retained history.
Do not implement speculative Claim prefetch or deferred Materialize.
The separate [group-commit report](rom-0.1.0-durable-work-group-commit-2026-10-08.md) explains those semantic hazards.

First measure actual page amplification under a bounded, separately admitted diagnostic trial.
A keyed journal has a concrete source-level advantage, but no measured throughput gain yet.
The largest uncertainty is whether journal pages materially contribute to native commit time on this storage device.
WorkUpdate does not rewrite journal metadata, yet its native commit interval dominates the supplied timings.

## Current source findings

[`StorageMetadata`](../../crates/rom/src/storage_state/metadata.rs) stores scalar accounting and a complete `Vec<JournalEvent>`.
Its `from_state` clones that vector.
[`prepare_bundle`](../../crates/rom/src/storage_state/metadata/bundle.rs) clones metadata into a canonical candidate and retains another before-image.
This includes every retained event Row.

[`journal_bundle`](../../crates/rom/src/storage_state/bundle.rs) appends an event for a changed Resource.
It serializes the incoming event to enforce the per-event byte limit.
It repeatedly serializes retained events to calculate the retention byte total.
Each evicted prefix item advances `floor` and returns an identity for native deletion.
The retained prefix remains bounded by configured journal rows and bytes; this is amplification within that bound.

SQLite [`save_metadata`](../../crates/rom-sqlite/src/native_work/write.rs) rewrites the complete metadata JSON in `rom_state`.
The same transaction also writes the Row to `events(identity,data)`.
The event table has no persisted canonical position or canonical event-byte accounting.
The metadata vector supplies those facts today.

[`retry_epochs`](../../crates/rom-sqlite/src/persistence.rs) reads and decodes complete metadata before returning scalar epochs.
The same occurs for `journal_head`.
Journal paging clones metadata and runs the canonical paging implementation.
Work Claim and Materialize use the keyed Work reader and do not call `save_metadata`.
Therefore, eliminating journal rewrite cannot directly eliminate their durable transaction cost.

Canonical [`validate_archive`](../../crates/rom/src/storage_state.rs) requires exact event/native Row agreement.
It verifies contiguous positions, unique identities, `head - floor == event_count`, receipt/effect counts, and canonical event bytes.
The proposed layout must preserve these checks during open, import, archive, migration, and explicit maintenance.

## Measurement limits

The parent supplied these stage totals for terminal session 20442:

| Stage | Resource Commit | WorkUpdate |
| --- | ---: | ---: |
| Native commit | 43.248034681 seconds / 2200 samples | 62.402274084 seconds / 4306 samples |
| Metadata read | 1.429765674 seconds | No separate Work interval |
| Shared preparation | 2.588525454 seconds | 0.405251569 seconds |
| Native publication | 1.213283598 seconds | 0.227359415 seconds |

The supplied RetryEpochs metadata interval is 6.376582435 seconds.
These values are parent-observed evidence, not measurements reproduced by this investigator.
The parent owns raw trial logs and the fresh untraced HOST-stage run.
The older aggregate trace reported 6611 fsync calls over 89.080815 seconds in a separate trial.
Do not subtract that aggregate from these stage intervals or treat it as the same capture.

Native commit includes pager, WAL, synchronization, and possible checkpoint work.
It does not isolate fsync latency.
Removing the supplied Commit metadata and preparation intervals would not prove admission of the unchanged workload.
A future comparison must keep the seed, binary/source identity, profile, durability, device, and original deadlines explicit.

## SQLite mechanism

SQLite WAL appends changed database pages and a transaction commit marker.
In WAL mode, FULL synchronizes the WAL on each transaction commit.
Automatic checkpoints normally start at 1000 WAL pages and can run in the committing thread.
A reader keeps one end mark throughout its transaction.
These mechanisms support the amplification hypothesis but do not measure ROM's device latency.
See the official [WAL explanation](https://www.sqlite.org/wal.html).

WAL contains committed changes not yet copied into the database.
The shared-memory WAL index supports coordination and does not supply durable application state.
Do not use that index as the authoritative ROM journal or epoch store.
See the official [WAL file format](https://www.sqlite.org/walformat.html).

PASSIVE checkpointing does not wait for readers or writers to finish.
FULL, RESTART, and TRUNCATE have different waiting and completion contracts.
Moving checkpoint work can change latency distribution without reducing total required durable work.
No checkpoint policy change is recommended before page and checkpoint attribution.
See the official [checkpoint API](https://www.sqlite.org/c3ref/wal_checkpoint_v2.html).

## Minimal shared support seam

Introduce a trusted scalar `MetadataHeader` with existing epochs, limits, generation, head, floor, receipt/effect counts, and journal count/bytes.
Provide validated public parts, following the existing WorkHeader pattern.
Keep `StorageMetadata` available for canonical format-7 archive projection and old-format reads.
Public application Resource and Storage paths do not need a changed contract.

Introduce a `JournalRead` support trait with coherent header and keyed prefix-entry reads.
Its transaction fence must share the publication context used for Work preparation.
Preparation returns an opaque journal delta with the exact before-header, appended fact, evicted prefix facts, and after-header.
Include read-only facts in delta preconditions, not just overwritten rows.
Reject application of a delta from another transaction or after an earlier publication invalidates its fence.

Refactor shared preparation into two responsibilities: scalar Resource/receipt/effect accounting and journal append/retention accounting.
Both native adapters must call the same semantic implementation.
The existing canonical implementation should delegate to that implementation through an in-memory journal reader.
Use canonical oracle tests to establish identical error ordering, byte accounting, and retired identity order.
Do not implement a second retention algorithm only inside SQLite.

Preparation computes the incoming `JournalEvent` serialization size exactly once.
It uses persisted, validated totals and reads only the prefix necessary for eviction.
For each evicted entry, validate position, identity, payload agreement, and stored canonical byte size before subtraction.
A malformed touched entry fails before any publication.
A changed-false bundle changes receipt/effect accounting without appending a fact or advancing journal head.
Replay returns its existing receipt without a fresh append or accounting increment.

A scalar-only epoch read becomes possible after the header is authoritative.
Do not accept a cached duplicate epoch that has not been checked against the persisted Work header.
Preserve the pre-revision retry check and receipt-replay seam.
The header must reject invalid epoch ordering, overflow, malformed limits, or inconsistent count/head/floor before use.

## Proposed native layout and publication

Keep event payloads in the existing `events(identity,data)` table.
Add `journal_positions(position,identity,canonical_bytes)` with a unique identity constraint and deterministic position ordering.
Persist scalar metadata in `rom_state` or a named scalar-header singleton.
Do not store a second Row copy in the position index.
Use an integer mapping that preserves the public u64 position range or reject overflow consistently with the current contract.
A plain SQLite signed INTEGER cannot silently narrow accepted u64 positions.

For one changed bundle, insert one payload and one position entry, delete the exact retired prefix, and update the scalar header.
Publish these with all existing Resource and Work writes in the same IMMEDIATE transaction.
Verify before-header and touched index/payload facts before writes.
Place test-support failure checkpoints between each new write.
A commit acknowledgement loss must still return Unknown and recover through the existing durable receipt.
No cache becomes authoritative after that result.

Journal reads need a single coherent read transaction for header, positions, and payloads.
A connection mutex alone cannot establish a snapshot across other SQLite writers.
Advance cursors across inspected events of other kinds, as the canonical implementation does.
Do not replace this with a kind-filtered SQL LIMIT that changes the returned cursor.
Preserve HistoryGap for expired, future, wrong-kind, and wrong-generation cursors.
Preserve TooLarge when the first matching event exceeds the caller byte bound.
Stop before the matching event that exceeds a subsequent row or byte limit.

Canonical collection must bound raw header, index keys/values, and payload bytes before copying or decoding.
Count derived position rows independently for physical admission.
Logical archive and journal bounds charge each canonical JournalEvent once.
Do not charge the duplicate derived identity/index bytes against the returned logical page budget.
Reject missing, duplicate, orphaned, reordered, or mismatched payload/index entries during full canonical validation.
Do not normalize oversized whitespace or dishonest byte accounting before raw physical admission.

## Format and compatibility work

The layout changes exact native inventory and the meaning of `rom_state`.
It requires an actual new native storage format; propose 11 only after coordinated adapter agreement.
Do not relabel existing format-10 databases or accept both schemas under one marker.
Ordinary open must refuse unsupported old/new layouts rather than partially interpret them.

Implement accepted format-10 to new-format conversion into a fresh destination through bounded canonical export/import.
Preserve source files and committed WAL history.
Keep formats 3–9 upgrade paths where already supported.
Migration, schema mapping, retention, operator maintenance, backup restore, and fresh initialization must all populate the position index.
A full canonical projection is acceptable on these bounded maintenance paths.
It is not acceptable for ordinary retry-epoch reads or Resource commits.

The canonical archive body can remain version 7 if its logical schema is unchanged.
[`archive::read`](../../crates/rom-backup/src/archive.rs) currently accepts exactly archive/storage pairs (7,9) and (7,10).
A new writer requires explicit acceptance of (7,new-format), preserving original manifest markers on reads.
Update shared constants, exact inventory checks, old-native fixtures, packaged consumers, and cross-backend conformance together.
Do not bump an archive version merely to hide an incomplete native migration.

## Alternatives and hazards

| Option | Benefit | Cost and reason for ranking |
| --- | --- | --- |
| Scalar header plus keyed positions | Removes full metadata payload reads and rewrites; reuses existing event Rows. | Recommended structural path. Requires shared delta, accounting, real migration, and cursor-equivalence tests. |
| Whole-metadata compression | May reduce dirty overflow pages for repetitive history. | Still clones, serializes, compresses, and rewrites the full retained prefix. Adds codec and decompression-bomb limits. No gain is measured. |
| Persistent scalar sidecar beside unchanged metadata | Can speed epoch reads without journal migration. | Does not remove large writes. Duplicates authority and requires atomic consistency checks for every maintenance/reset path. |
| Process cache or retained decoded projection | Can avoid repeated JSON decode. | Unknown acknowledgement, reopen, retention, reset, and raw corruption can invalidate authority. Not a durable replacement. |
| JSON scalar extraction from existing metadata | May avoid materializing event Rows during epoch reads. | Still scans the existing JSON representation. Error and corruption behavior needs equivalence tests; it does not reduce WAL writes. |
| Larger or background checkpoints | May move commit-tail checkpoint time. | Does not remove FULL transaction sync. Requires reader/WAL bounds and new failure/lifecycle tests; measure attribution first. |

Compression requires both a raw encoded-byte bound and a strict decompressed-byte bound before allocation.
Caching requires a validated persisted generation/revision in each coherent transaction and invalidation after any Unknown.
Those checks must include restoration, retention, operator control, migration, schema mapping, and external raw modifications.
Neither option may silently skip corruption that the current authoritative path rejects.

## Regression and measurement plan

1. Build a shared in-memory journal oracle against the existing canonical implementation before native changes.
2. Compare changed/unchanged/replayed bundles, mixed kinds, exact byte thresholds, multiple prefix evictions, and receipt/effect overload error order.
3. Test maximum positions, total-byte/count overflow, empty history, advanced floor, and restore generation fences.
4. Compare every page event and cursor across row/byte limits and other-kind gaps; include first-match TooLarge.
5. Inject errors after every payload, position, header, Work, receipt, Resource, reference, and query-index write. Reopen and compare canonical state.
6. Lose acknowledgement after commit and retry with the same identity. Assert one Resource mutation, one fact, and unchanged Work completion semantics.
7. Corrupt header totals, position order, stored sizes, duplicate/orphan entries, JSON, and oversized raw whitespace. Assert bounded rejection before publication.
8. Change a read-only header or prefix fact in the same reader context; reject the prepared delta. Reject another transaction's fence.
9. Test full-history canonical export, fresh restore, retention, schema mapping, old format-10 source preservation, and unsupported marker/layout combinations.
10. Exercise both native adapters through the same shared conformance cases and preserve configured maintenance admission budgets.
11. Run a fixed sequence while measuring actual WAL frames/pages per Resource commit and Work update, checkpoint count, and native commit time.
12. Compare identical retention bounds before saturation, at saturation, with larger payloads, and with a long-lived reader. Do not change the acceptance workload.

The parent must admit any new probe with finite disk/time budgets after its current trial ends.
WAL file length alone is not cumulative page traffic because WAL resets and reuse occur.
Collect frame/dirty-page evidence without treating observation overhead as the uninstrumented acceptance result.
After the diagnostic comparison, rerun the unchanged application gate with the default uninstrumented binary.

No proposed case in this section has been executed by this investigation.
Source-level complexity reduction is established; admission and the gain in milliseconds remain unproved.
The report uses STE guidance and project technical vocabulary without a claim of certified compliance.

## Executed bounded numerical WAL probe

The parent subsequently authorized one fresh-store diagnostic without product changes or Cargo.
The investigator first read both terminal stage result files and their stage snapshots.
Subtracting each baseline independently reproduces the supplied loop timings above.
It also gives the following untraced HOST-stage intervals:

| Interval | Seed-only samples | Seed-only elapsed seconds |
| --- | ---: | ---: |
| Resource native commit | 4249 | 39.831457582 |
| WorkUpdate native commit | 8484 | 48.194176183 |
| Resource metadata read | 4249 | 3.688375814 |
| RetryEpochs metadata read | 12747 | 15.657736854 |

Both runs failed their original seed deadlines.
Both recorded source fences before and after execution and drained cgroups without OOM events.
The independently calculated differences and input hashes are in `prior-stage-comparison.json` under the new probe directory below.
These historical fences do not establish a match against later source edits.

The tracer control used an owned file and one positional 4096-byte write.
`strace -e raw=pwrite64` printed only the descriptor, pointer address, requested length, offset, and return value.
A second control verified exact path filtering.
Neither control printed a buffer's contents.
Control evidence and diagnostic scripts are in `/var/tmp/rom-wal-trace-control-20261009/`.

The first launch failed immediately because the fixture rejected a HOST path outside its permitted prefix.
Its drained result is `/var/tmp/rom-owned-wal-amplification-018417454210b8c46c69bc8d/result.json`.
This failed launch remains preserved.
The corrected launch bound a new HOST directory into the fixture's permitted path through a private mount namespace.
It used the same preserved stage binary and original prepare profile.
It installed no SQLite hook and invoked no checkpoint API.

The corrected probe directory is `/var/tmp/rom-owned-wal-amplification-50dc2af04abb8342a8acc136`.
Its private namespace projection was `/var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-50dc2af04abb8342a8acc136`.
The tracer selected only that new store's `database-wal` path.
It traced numeric `pwrite64` arguments and sync/truncate calls.
The configuration contains private synthetic credentials and remains mode 0600; do not publish it or its contents.

The launcher enforced a 130-second timeout, a one-second termination grace, and managed process-group drain.
Its cgroup limited memory to 4 GiB, tasks to 512, and CPU to four cores.
A file-size limit capped each producer file at 64 MiB.
A 100-millisecond monitor stopped the owned group for sampling failure, an unknown output entry, a symlink, or a capacity boundary.
The monitored boundaries were 16 MiB for trace output, 256 MiB aggregate directory bytes, and at least 1 GiB free filesystem space.
The aggregate monitor was not a hard quota and could overshoot between samples.
No other process was stopped.

The trace boundary ended the probe after approximately 44 seconds.
The observed trace size was 16,782,826 bytes, which exceeded the 16 MiB threshold by 5,610 bytes.
The maximum sampled aggregate directory size was 26,753,688 bytes across 441 samples.
The original 120-second workload deadline was not changed; the separate diagnostic guard ended execution first.
The result is `diagnostic-failed`, with an explicit `monitor-fail-closed` reason, drained group, and both historical source fences true.

### Numerical prefix evidence

The raw trace is `numeric-wal.trace` in that directory, protected by mode 0600.
The authoritative numerical summary is `numeric-wal-summary-v2.json`.
Version 1 remains preserved and undercounts the final sync attempt with an unknown return.
The second parser handles that interrupted return separately.
It also verifies a 4096-byte database page size from the fixed database header, without opening SQLite for writes.

| Observed syscall evidence | Value |
| --- | ---: |
| Successful WAL `pwrite64` calls | 264246 |
| Cumulative successful WAL write bytes | 544087176 |
| Aligned 24-byte frame-header writes | 132059 |
| Aligned 4096-byte page-payload writes | 132059 |
| Page-payload bytes | 540913664 |
| 32-byte WAL-header writes at offset zero | 128 |
| Failed or short positional writes | 0 |
| Successful WAL fsync calls | 4672 |
| Final interrupted fsync, unknown return | 1 |

The raw trace SHA256 is `4fe36a43204350a27a5c09af7d4976a51ddfff0e55d47eee2fc75e04de4cb711`.
Every observed positional-write return was parsed.
There were no pending partial syscall lines at the end.
Eight non-numeric terminal signal records remain explicitly counted.
The final sync returned `?` after SIGTERM and is not counted as success.
The summary marks the full trial incomplete and retains the terminal-signal indicator.

SQLite defines a 32-byte WAL header and a 24-byte header before each page payload.
WAL resets can reuse earlier offsets without truncating the file.
These facts justify the observed alignment classification and cumulative counting across repeated offsets.
See the official [WAL frame and reset format](https://www.sqlite.org/fileformat2.html#walformat).

The counts describe actual successful syscall traffic, not the final WAL file length.
They do not establish that every written frame committed or survived recovery.
They do not count block-device traffic below the filesystem.
They do not distinguish Resource commits, Work updates, event pages, metadata pages, or checkpoint causes.
No callback or table-page marker was present in this trace.
The terminal interruption prevented a final stage snapshot.
Do not divide these counters by the last progress line to claim a per-Resource or per-commit cost.

### Conclusion from the prefix

The bounded prefix confirms substantial physical WAL write traffic under the current layout.
It strengthens the case for investigating reduced dirty-page publication.
It does not prove that retained journal duplication caused those writes or that format 11 meets the deadline.
Do not increase trace limits merely to complete a 120-second trace.

A further diagnostic needs a separately reviewed numeric table/page attribution mechanism.
A SQLite WAL hook is not a passive observer: installing it replaces the existing callback and can remove automatic checkpoint behavior.
Its callback can also return an error after a successful commit.
See the official [WAL hook contract](https://www.sqlite.org/c3ref/wal_hook.html).
`PRAGMA wal_checkpoint(PASSIVE)` performs checkpoint work and is not a read-only measurement.
Neither mechanism was used in this probe.

The parent owns subsequent format approval, shared-support changes, and release verification.
No further probe was launched after this prefix.

## Task 0 numeric publication counters: implementation checkpoint

Root authorized only the numeric attribution task before a layout change.
The adapter now supplies `StageObservation::publication_snapshot()` with ten fixed entries.
The original 18 timing entries remain unchanged.
The two operation labels are `commit` and `work_update`.
Each operation has `metadata`, `event_payload`, `work_header`, `work_record`, and `work_root` categories.
Entries contain `samples`, `encoded_bytes`, `dropped_samples`, and `saturated`; the snapshot also reports recorder poisoning recovery.

A sample follows a successful SQL statement, before its unchanged failure checkpoint.
The byte count uses the exact serialized String supplied to that statement.
It excludes native keys, active indexes, references, receipt/effect Rows, database page headers, and filesystem traffic.
Metadata includes its retained journal payloads.
No additional serialization computes diagnostic byte counts.
Successful statements later rolled back remain counted.
The counter therefore does not describe durable committed bytes.
Replay and scalar epoch reads add no publication samples.
Maintenance import and initialization are outside these operation counters.

Without `test-support`, observer fields and recording calls are absent.
With the feature enabled, an unobserved instance does not update counters.
Overflow drops an entire sample, preserving earlier count/byte totals and reporting a dropped sample.
Poisoned recorder synchronization recovers without returning a storage error.
No payload, identifier, callback, hook, or per-request label is retained.

The first compile failed because a test requested unsupported SQLite u64 decoding.
The test now uses checked i64 conversion.
The genuine behavior RED then found zero Metadata samples after a real successful commit, where one was expected.
Wiring exact SQL-success observations made that test pass.
The subsequent complete test-support library checkpoint passed 54 tests.
It includes raw persisted byte comparisons, growing retained metadata, Claim/Materialize accounting, rollback, unknown acknowledgement, replay, disabled controls, overflow, and poisoning.
Final default-feature, all-target Clippy, consumer, and workload gates remain separate at this checkpoint.

CONTAINER raw logs use these paths in `rom-dev:/var/tmp/`:

- `/var/tmp/rom-010-sqlite-publication-bytes-red-20261008.log`: initial compile failure.
- `/var/tmp/rom-010-sqlite-publication-bytes-behavior-red-20261008.log`: genuine zero-counter behavior failure.
- `/var/tmp/rom-010-sqlite-publication-bytes-wired-20261008.log`: one targeted test passed.
- `/var/tmp/rom-010-sqlite-publication-bytes-lib-20261008.log`: 54 library tests passed.

Root owns batch collection and the diagnostic consumer.
A fresh binary/source capture is required before another attribution workload.
No layout marker changed; format 10 remains current.

### Final adapter counter gates

A real SQLite trigger now rejects event insertion before successful publication.
That test verifies zero publication samples and a fully rolled-back Resource mutation.
It distinguishes SQL-success counters from encoding-attempt counters.
A feature-only facade import warning found by the consumer build was corrected.
Direct adapter Clippy then passed with warnings denied.

| Final gate | Executed result | HOST raw log |
| --- | --- | --- |
| Test-support library | 55 passed | `/var/tmp/rom-010-sqlite-publication-bytes-final-lib-20261008.log` |
| Test-support all-target Clippy | Exit 0; warnings denied | `/var/tmp/rom-010-sqlite-publication-bytes-final-clippy-20261008.log` |
| Default library | 36 passed | `/var/tmp/rom-010-sqlite-publication-bytes-default-lib-20261008.log` |
| Default all-target Clippy | Exit 0; warnings denied | `/var/tmp/rom-010-sqlite-publication-bytes-default-clippy-20261008.log` |
| SQLite source formatting | Exit 0 | `/var/tmp/rom-010-sqlite-publication-bytes-fmt-20261008.log` |

All earlier counter logs listed above now also have preserved HOST copies in `/var/tmp/`.
Their original CONTAINER files remain unchanged.
`/var/tmp/rom-010-sqlite-publication-bytes-source-gate-20261008.json` records log hashes and after-command source identity.
It does not establish a pre-command source fence.
Cargo used two jobs, disabled incremental compilation and development debug information, offline locked dependencies, and a 180-second timeout per command.

The adapter Cargo lease was released and its sources frozen for a new parent-owned consumer capture.
Equal-completed-batch attribution, a full verifier after these changes, and the original uninstrumented workload remain separate gates.
No journal keying or new native marker has been implemented by this task.

### Task 0: Completed-batch publication attribution

The fresh HOST diagnostic finished with a seed deadline failure.
Its last fully drained batch contained 1500 Resources at 115293 milliseconds.
The original profile still required 10000 Resources within 120000 milliseconds.
This diagnostic is not an acceptance pass.

The independent validator checked all fifteen completed-batch pairs against their call snapshots.
It subtracted the pre-seed baseline before calculating cumulative publication bytes.
All fixed labels matched; counters reported no dropped samples, saturation, or poisoning recovery.
Claim plus Materialize call counts matched WorkUpdate header publication counts at each completed batch.

| Publication category | Samples at 1500 completed Resources | Encoded SQL payload bytes, after baseline |
| --- | ---: | ---: |
| Commit metadata | 1500 | 328670214 |
| Commit event payload | 1500 | 197250 |
| Commit Work header | 1500 | 644989 |
| Commit Work record | 1500 | 1217250 |
| Commit Work root | 1500 | 70500 |
| WorkUpdate metadata/event payload | 0 | 0 |
| WorkUpdate Work header | 3030 | 1304516 |
| WorkUpdate Work record | 3000 | 2520000 |
| WorkUpdate Work root | 3000 | 141000 |

Each equal batch contained 100 Resources.
The first batch published 1641631 metadata bytes; the last published 42213300 metadata bytes.
Each batch published 13150 event payload bytes.
Thus, metadata publication grew with retained history while the newly emitted event payload remained constant for this workload.
Source inspection identifies the retained journal inside the counted metadata string.
The measurement does not isolate each field within that string.

Terminal totals included the next, incompletely drained batch.
They recorded 1600 Commit operations, 3094 WorkUpdate operations, and 4800 retry epoch reads after baseline.
Terminal Commit metadata publication totaled 373788514 bytes.
These totals must not be divided by the 1500 fully drained Resources as a per-Resource estimate.
Native commit intervals totaled 56.289215393 seconds for Commit and 52.611657776 seconds for WorkUpdate.
Those intervals include native transaction completion costs; they do not measure fsync alone.

The binary capture included 393 source inputs.
Its SHA-256 was `4d6eec7143bc3c711840547bf7ca5ee62d666e10971f7fbb37701ee3d7942854`.
Both source fences passed, the managed process drained, and the cgroup reported zero remaining processes.
The binary exited with status 1 after the deadline failure.
The directory budget remained a plan, not a hard quota.

This trial drained fewer Resources than the earlier HOST timing trial, which reached 4200.
The captures and observation differ; observer overhead and host variation remain unresolved.
Do not use this comparison to claim equivalent throughput or attribute the difference to the observer.
These counters measure selected successful SQL payload writes, including writes later rolled back.
They do not measure all database bytes, physical WAL traffic, or durable committed bytes.

Root accepted this attribution as justification for the additive canonical header/journal prototype in Tasks 1–2.
That decision does not admit native format 11 or promise a performance improvement.
Both adapters, compatibility gates, and a fresh uninstrumented acceptance run remain required before a layout change can ship.

HOST evidence:

- `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-7b1a9997aa0760c97a5d58b3/result.json`: terminal lifecycle and source fences.
- `/var/tmp/rom-owned-host-seed-7b1a9997aa0760c97a5d58b3/source/load-storage-publication.json`: baseline, fifteen completed batches, and terminal publication counters.
- The same source directory contains `load-storage-calls.json` and `load-storage-stages.json`.
- `/var/tmp/rom-010-publication-attribution-validation-20261008.js`: independent numeric validation and fixed-label checks.
- `/var/tmp/rom-010-publication-attribution-validation-20261008.json`: passed validation, input hashes, equal-batch deltas, and separate terminal totals.

### Task 3 SQLite design: read-only preparation

The shared Task 2 API is now available.
This section specifies adapter work; it does not record implementation, tests, or format 11 admission.
The format 10 predecessor capture and parent verifier retain the sole Cargo lease.

#### File boundaries

| File | Proposed responsibility |
| --- | --- |
| `crates/rom-sqlite/src/native_journal.rs` | Facade exports only. |
| `native_journal/schema.rs` | Exact candidate schema, inventory, and unsigned position encoding. |
| `native_journal/read.rs` | Scalar header and keyed entry reads through the existing transaction-local Reader. |
| `native_journal/write.rs` | Journal delta publication after combined fact validation. |
| `native_journal/canonical.rs` | Bounded full projection using `JournalImage::from_native`; explicit maintenance only. |
| `native_work/read.rs` | One shared fence and physical read budget for both reader traits. |
| `native_work/write.rs` | Reuse `WorkDelta::validate`; separate private publication from validation. |
| `persistence.rs` | Preserve arbitration order; jointly validate candidates before the first Resource write. |
| `snapshot.rs`, `store.rs` | Exact native inventory and coherent public reads after coordinated activation. |
| `upgrade.rs`, `migration.rs`, `maintenance.rs` | Reuse canonical snapshot restore for genuine predecessor conversion. |
| `native_journal_tests.rs` | Separate real-SQLite behavior, corruption, budget, and fault tests. |

#### Schema and canonical representation

Use an eight-byte big-endian BLOB for each journal position.
SQLite INTEGER cannot represent the complete unsigned u64 range.
Require BLOB type and exact length in schema constraints and reader validation.
Use a `WITHOUT ROWID` position table with an explicit unique identity index.
Store identity and the canonical JournalEvent byte count beside the position.
Encode that byte count as an eight-byte unsigned BLOB, with checked conversion to usize on read.
Reject zero positions, malformed blobs, duplicate identities, and values outside canonical header bounds during maintenance validation.

Keep the existing `events(identity,data)` Row payload.
A keyed read joins the position entry to its event Row.
It reconstructs `JournalEvent { position, identity, row }` for the shared validator.
It does not persist a second full JournalEvent payload.
An index entry without its Row is corruption, not an absent event.
An absent position remains observable even beyond the declared head; do not hide corrupt append-slot entries with a header filter.

Replace full `rom_state` journal metadata with serialized `MetadataHeaderParts` only after coordinated layout activation.
The scalar DTO currently has no serde derives.
Root should add the shared serialization contract rather than require matching adapter-owned DTO copies.
Canonical maintenance reconstructs all events and calls `JournalImage::from_native` once.
It then reassembles the existing canonical StorageMetadata with the existing Work and operator projections.
Generation, floor, head, epochs, counters, storage limits, and every retained event remain authoritative canonical fields.

#### Coherence, bounds, and publication

Implement `JournalRead` and `WorkRead` on the same existing SQLite Reader.
Both implementations return its single ReadFence.
Both use one cumulative physical read budget, initialized from the opened store's configured validation limits.
Read borrowed SQLite values, charge key and raw value lengths, then allocate or decode.
Repeated fact reads consume the same cumulative budget; do not reset it between journal and Work validation.
Header, index, event Row, Work header, record, and root reads share this budget.
Candidate enumeration also charges examined active IDs and payloads.
Logical journal page limits remain separate from the physical admission budget.
Whitespace and oversized invalid JSON must fail admission before decoding.

Keep receipt replay and revision arbitration before candidate preparation.
Use `MetadataHeader::check_retry_epoch` for the existing pre-revision retry check.
Use `prepare_native_bundle` for the new journal and Work candidates.
Call `NativeBundleDelta::validate(&reader, &reader)` before the first Resource, reference, query index, receipt, event, or effect write.
This call must re-read persisted header and all recorded journal/Work facts.
Do not rely on a cached header or validate Work facts only after Resource writes.

Use a private validated-publication helper to write the Work delta without a second algorithm.
Write the journal append, retirements, scalar header, and Work changes in the original IMMEDIATE transaction.
Rotate the shared fence once after combined publication.
Work-only publication must also rotate that fence, invalidating outstanding combined candidates.
No fallible domain callback moves into the transaction.
Existing fault checkpoints remain test-only; new journal SQL boundaries receive rollback checkpoints.
Commit failure remains unknown; acknowledgement failure after durable commit remains unknown.

For `journal` reads, create one read transaction across header and entries.
Use shared `journal_page`, including cursor progress across other Resource kinds.
For `retry_epochs`, read only the validated scalar header.
Explicit archive/operator reconstruction uses the configured maintenance limits, with raw bounds before allocation and canonical limits after reconstruction.
Do not replace an explicit larger or smaller profile with default BackupLimits.

#### Candidate marker ruling and conversion

Stage candidate schema tests through private real-SQLite test helpers while public format 10 behavior remains unchanged.
Do not expose a physical format 11 store through the public format 10 open or backup path.
Do not produce archive pair 7/10 from a physical candidate 11 store.
Once both adapters pass their candidate gates, root can admit coherent public activation and archive pair 7/11 together.
This route avoids temporary public dual-layout semantics and dishonest backup metadata.

Task 5 must collect genuine format 10 through its preserved old schema reader.
Use the existing coherent read-only source snapshot and fresh destination restore publication.
Preserve custom codecs, descriptor bindings, operator state, query profiles, epochs, receipts, effects, and retained Work history.
Validate the complete source within caller-supplied limits before destination publication.
Source bytes and committed WAL remain unchanged; conversion never relabels an existing database in place.
Inject failure after candidate writes, after stage close/sync, and before final destination publication.
The destination must remain absent on rejected publication; the accepted source must remain usable.
Historical canonical archive handling remains root-owned and distinct from native conversion.

#### Required genuine regression gates

1. Prove the current format 10 store lacks the new keyed layout before candidate implementation.
2. Verify keys at `i64::MAX`, `i64::MAX + 1`, and `u64::MAX`, including exact ordering and malformed lengths.
3. Compare candidate commits and retention against the shared canonical oracle, including unchanged mutations and zero-capacity journals.
4. Change each persisted journal/Work/header fact after preparation; validation must reject before any Resource write.
5. Publish Work-only or combined deltas; prior candidates must fail through the shared fence.
6. Reject missing payloads, extra positions/events, bad canonical lengths, duplicate identities, inventory changes, and scalar accounting mismatch.
7. Exceed cumulative journal-plus-Work raw budgets with individually small values; reject before allocation or decode.
8. Verify cursor progress, kind filtering, byte limits, generation reset, and exact page continuation under coherent reads.
9. Inject rollback after each journal/index/header/Work publication boundary; reopen and compare the complete canonical state.
10. Lose acknowledgement after commit; replay must return the stored receipt without another event or Work completion.
11. Convert captured genuine format 10 into a fresh candidate destination using custom limits; verify source preservation and canonical equality.
12. Verify disabled diagnostics and default-feature outcomes after coordinated public activation; preserve FULL and the original load profile.

The largest uncertainty is adapter activation and predecessor collection, not BLOB ordering.
Current snapshot inventory selects the current format through a shared constant; format 10 requires an explicit preserved branch after activation.
The staged layout cannot promise the original workload deadline until both adapters, compatibility, and fresh uninstrumented load gates pass.
