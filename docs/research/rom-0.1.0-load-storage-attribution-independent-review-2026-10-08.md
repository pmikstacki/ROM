# Independent load-fixture storage attribution review

Date: 2026-10-08. Scope: fixture source and recorded unit-test evidence only.
No native execution, adapter changes, or actual load probe occurred during this review.

## Result

No reviewed source blocker prevents a bounded attribution probe after the owner's fresh fixture checks finish.
The wrapper preserves adapter Results and does not hold observation locks during adapter operations.
The evidence measures synchronous storage-boundary wall intervals. It does not measure internal codec time, CPU time, or cumulative write bytes.
This review does not admit the release or establish a final source inventory.

## Transparent observations

`host/src/storage_calls.rs` defines exactly 18 fixed categories. The five WorkUpdate variants have distinct categories.
`ObservedStorage` forwards each actual operation and its arguments. Boolean capability methods remain uninstrumented and return the inner adapter's answer.
Owner acquisition is forwarded. The wrapper does not create an independent ownership lease.

`StorageCalls::measure` calls the adapter before taking its observation lock.
Its generic Result is returned unchanged after recording. The reentrant observation test exercises this ordering.
`ObservedStorage::commit` releases queue bookkeeping before the inner commit. Confirmation bookkeeping follows the adapter result.
Work claims likewise complete before queue bookkeeping. The observer does not fabricate a commit decision or a queue timestamp after an unconfirmed result.

Counts and total nanoseconds use checked arithmetic. An unrepresentable interval or exhausted counter increments an overflow flag instead of wrapping.
If the overflow counter is exhausted, a separate boolean records that condition.
A poisoned observation mutex skips recording or returns None. It does not replace the adapter Result.
Counters can therefore be incomplete; evidence consumers must inspect overflow fields and None snapshots.

The metric JSON contains operation categories, counts, intervals, and row counts. It contains no Resource payloads, keys, or principal identities.
The existing queue observer uses work IDs privately to match confirmations. Its exported evidence contains intervals and numeric missing/rejected counts.

## Bounds and failure scope

`runner.rs:61` captures the Result of `prepare_workload` before attempting terminal storage evidence.
An ordinary workload error therefore still reaches the bounded JSON write.
If both fail, the workload error remains primary and a fixed diagnostic reports unavailable evidence.
If only evidence writing fails, the preparation fails. It cannot return success without its terminal metric file.

The terminal file uses create-new mode 0600, a 65536-byte limit, and sync_all.
Batch records contain six fixed cumulative call-count/nanosecond pairs. Their capacity is 100, with at most 10000 completed rows.
The complete terminal snapshot retains all 18 categories, errors, maxima, and overflow flags.
Batch pairs omit those extra fields; they must not be treated as complete per-batch error records.

The workload still requests 100 batches of 100 Resources and 10000 retained completed work records.
Admission, create, and drain boundaries retain the 120-second deadline.
`prepare-run.mjs` retains the 130000-millisecond outer timeout and checks the 120000-millisecond proof bound.
Synchronous adapter calls can exceed an inner check interval. The outer process deadline remains necessary.

The terminal-write claim has explicit limits:

- Directory creation, database opening, runtime construction, and identity seeding fail before `runner.rs:61`.
- A panic, outer kill, disk failure, or process loss can prevent terminal JSON.
- No maintained test currently demonstrates terminal metric writing after an injected workload error.

These limits do not block the intended finite seed attribution probe. They prevent a claim that all runtime failures produce terminal evidence.
For that broader claim, include setup errors within the observed result boundary and add an injected ordinary-failure test.
Preserve external process output when forced termination prevents in-process evidence.

## Interpretation

The baseline follows identity seeding. Final counts include the baseline, workload, final snapshots, and shutdown when preparation succeeds.
Subtract baseline counts for workload attribution. Do not label the resulting totals as a database internal profile.
Concurrent intervals can overlap. Nested observations can also overlap, as the maintained reentrancy test demonstrates.
Do not sum arbitrary category durations into an exclusive percentage of total wall time.
The record correctly states `live_encoded_state_bytes_measured: false`.

## Recorded checks

The supplied native log reports 13 passing unit tests, zero failures, and zero ignored tests in 0.12 seconds.
It includes Result preservation, elapsed overflow, reentrancy, and actual SQLite/redb registration and reaction cases.
Its SHA-256 is `f209dbc312f0fcf6954c59812f217367a57e3d4e6d550fd2020b5966ba8ef487`.
Host path: `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-load-storage-call-green-20261008.log`.

The current source also contains two new `seed_measurements` tests.
They check worst-case numeric JSON size and refusal of an extra batch without changing prior observations.
They do not appear in that 13-test log. Obtain fresh execution evidence before claiming those tests passed.
The owner's subsequent build and Clippy were still pending when this source review began.

| Reviewed source | SHA-256 |
| --- | --- |
| `observed_storage.rs` | `d33ef2d0ce11aa244e1a587fd40f4f1dccb0c59988a8f07ebf3d735de2105825` |
| `storage_calls.rs` | `d206248ab0590c332df11dc112b5c91e33e93a3f98af4ad4e16ac7f695661d94` |
| `storage_calls_tests.rs` | `7b330ff0a3a527c55dc7338d42f10df922091ddbe1384aa3ae14cf4cbc60b7f2` |
| `seed_measurements.rs` | `055c63438b4840a3f4a28797865e9d662f26ebf86dace0d35076e1bc596cf652` |
| `seed_measurements_tests.rs` | `acc1c82532d76acfbbdad64f90b5ab4ed7190441e2a3b30688d5ae3e36bb163c` |
| `runner.rs` | `d988011d0df2b182c7d45c8c52666b608b2a477c80303a29f0c972cfada64f18` |

All source paths in this table start with `tests/application-load/host/src/`.
This table identifies the inspected files, not a complete execution-source fence.

## Fresh fixture gate

The subsequent raw gate reports 15 passing tests, zero failures, and zero ignored tests in 0.08 seconds.
Both `seed_measurements` tests appear by name and pass. The scoped Clippy log reaches successful completion.
The six source hashes above remain unchanged after this evidence audit.

| Log under `/var/lib/nixos-containers/rom-dev/var/tmp/` | SHA-256 |
| --- | --- |
| `rom-010-load-storage-call-full-green-20261008.log` | `02769ded667903357a775b06c4adc0ea89cc2672ae1d9df186dd2bed3e867298` |
| `rom-010-load-storage-call-clippy-20261008.log` | `85b65f410635f891ad4e168629fe406a482cde602c6c9d95fe53a288f7293ca3` |

The fresh gate closes the two previously missing maintained-test observations.
No pre-probe blocker remains from this source and evidence review.
Retain the 10000-Resource profile, deadlines, terminal-evidence limits, and interpretation limits stated above.
An optimized build and actual probe still need their own complete source and execution evidence.

## Actual failed attribution probes

Both subsequent optimized seed probes report failure with the unchanged 10000-Resource target and 120000-millisecond preparation deadline.
Their result records retain before/after source fences, drained exit-code-one processes, empty terminal cgroups, and zero recorded OOM events.
SQLite's stderr reports `finite seed drain deadline`. redb's stderr reports `finite seed create deadline`.
These are actual failed admission results, not successful load acceptance.

The independent audit read each result, process output, and terminal metric file directly.
It recalculated all six reported call counts and nanosecond totals by subtracting the identity baseline from terminal observations.
Every recalculated value matches `/var/tmp/rom-010-attribution-summary-20261008.json` and the root attribution report.
The SQLite metric contains 11 completed batch records; its last batch is 1100 rows at 103945 milliseconds.
The redb metric contains 14 completed batch records; its last batch is 1400 rows at 114107 milliseconds.
Metric files contain 7728 and 8208 bytes respectively, within the maintained 65536-byte limit.

| Adapter | Calls and boundary intervals after baseline subtraction |
| --- | --- |
| SQLite | Commit: 1200 / 37774630061 ns; Claim: 1186 / 35356915197 ns; Materialize: 1164 / 40059146276 ns |
| redb | Commit: 1485 / 36949554696 ns; Claim: 1428 / 34014664660 ns; Materialize: 1400 / 38637954516 ns |

All 18 terminal categories report zero failed calls, observer overflows, and exhausted overflow counters.
Both fixtures report zero Finish calls. The successful empty-child reactions complete through Materialize.
The metrics expose expensive storage boundaries. They do not isolate serialization, synchronization, or scheduler cost inside those boundaries.
They do not establish a measured benefit for the proposed incremental correction.

Evidence roots start with `/var/tmp/rom-010-authentik-20261007/run/volume/`:

| Adapter | Result suffix under `evidence/` | Metric suffix under `private/` | Metric SHA-256 |
| --- | --- | --- | --- |
| SQLite | `application-load-seed-63551767c2659fee265ce662/result.json` | `application-recovery-63551767c2659fee265ce662/source/load-storage-calls.json` | `c256cedd764d30b355d695a8cf53c7b91fc421f0be69bf74289404dc504e74f4` |
| redb | `application-load-seed-a57f4a54daf51195f73d9fca/result.json` | `application-recovery-a57f4a54daf51195f73d9fca/source/load-storage-calls.json` | `54414e8d331b7ddfec74f006e56b1469cbea2b8b504302c30969e21c8d6986d5` |

The audit independently verified both metric hashes.
The root report's copied terminal-state counts remain inspection-owner reports in this review.
Their exact inspection paths had not been reconciled; this audit did not open those copies or the original databases.
No native execution or product edit occurred during the audit.
