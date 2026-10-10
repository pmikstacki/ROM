# Native-10 SQLite seed: diagnosis and controlled probes

Date: 2026-10-08.

## Result and evidence scope

The unchanged seed failed at its 120-second drain deadline.
It created 2,500 Resources. Its last complete 100-row batch drained 2,400 Resources at 116,257 milliseconds.
The final create progress records 2,500 at 118,883 milliseconds.
The failure is preserved. It does not establish a successful 10,000-row seed or production throughput.

The result records unchanged source fences before and after execution.
The owned process closed with exit code 1 and drained its group.
The recorded cgroup has zero population, zero OOM events and no PID-limit event.
This establishes bounded failure and cleanup. It does not exclude CPU throttling or filesystem waits.

The build identity contains 382 source/input hashes and an optimized binary of 24,441,032 bytes.
Its binary SHA-256 is `cf152939ff7c2a301678c7550825047e26a91a51b5aeb924d6495445d5df387d`.
The reviewer recomputed all 382 hashes: zero current mismatches.
The private binary also matches its recorded hash.
These checks precede any future owner change. They are not a perpetual source freeze.

No native command, trace, new probe or product edit was performed for this review.
The separate redb seed remains outside this result until its terminal evidence is inspected.

## Numerical attribution

The following values subtract the recorded identity/setup baseline from terminal storage observations.
They measure synchronous storage-boundary wall time, including locks, decoding, transactions, sync I/O and scheduler delay.
They are not exclusive CPU time or nested database-stage attribution.

| Operation | Calls | Total wall seconds | Mean milliseconds |
| --- | ---: | ---: | ---: |
| commit | 2,500 | 48.930103959 | 19.5720415836 |
| Claim | 2,512 | 31.696568233 | 12.6180606023 |
| Materialize | 2,464 | 31.854038213 | 12.9277752488 |
| retry_epochs | 7,500 | 7.117800587 | 0.9490400783 |
| load | 14,928 | 0.198735282 | 0.0133129208 |
| receipt | 7,500 | 0.057240495 | 0.0076320660 |

The summed boundary wall time is 119.854486769 seconds.
Do not equate this sum with process CPU consumption.
All terminal observed failed-call and observer-overflow counts are zero.
The fixture deadline is a failed workload result, not an adapter error.

`reaction_records`, snapshot, query_read, work_snapshot, operator controls, journal reads and separate Finish calls all have zero observed calls.
Therefore, repeated full canonical maintenance reconstruction cannot explain this seed's measured path.
The per-resource path still performs approximately three durable transitions: Resource commit, Claim and Materialize.
At 10,000 Resources in 120 seconds, 30,000 such transitions would allow about four milliseconds per transition before other work.
This is a budget calculation, not a predicted implementation limit or a proposal to remove transitions.

The first full batch drains at 2,947 milliseconds. The 1,000-row batch drains at 46,956 milliseconds.
The 2,000-row batch drains at 90,060 milliseconds. The 2,400-row batch drains at 116,257 milliseconds.
Individual drain durations vary; they do not form a simple monotonic CPU-cost curve.

Metadata-read growth is clearer.
The first batch's retry_epochs total is 18.251380 milliseconds including the setup baseline.
The last complete batch adds 459.788423 milliseconds for 300 retry_epochs calls, about 1.532628 milliseconds each.
This is consistent with increasing metadata decoding cost. It does not establish its exclusive cause.

## Ranked falsifiable hypotheses

| Rank | Hypothesis | Existing evidence | Discriminating observation |
| --- | --- | --- | --- |
| 1 | Sync I/O or private filesystem latency dominates durable transitions. | Claim and Materialize each average about 12.6–12.9 ms despite keyed Work publication. Both commit each transition. | Aggregate owned-process fsync/fdatasync counts and wall time; adapter commit-stage intervals; matched filesystem control with identical durability. |
| 2 | Retained journal metadata still amplifies JSON reads, copies and writes. | StorageMetadata includes the complete retained event vector. retry_epochs decodes this vector; bundles clone it twice. | Numeric metadata bytes/event counts, decode/encode/clone/journal intervals and explicit commit intervals for the same operation categories. |
| 3 | Native page/index and WAL checkpoint work amplifies filesystem writes. | Resource commit updates several tables and rewrites journal metadata. SQLite may checkpoint at WAL thresholds. | Aggregate write bytes, WAL frame/checkpoint counters and sync timing aligned to fixed batch progress; no raw payload logging. |
| 4 | Guard waits or scheduling inflate inclusive boundary time. | Observations begin before adapter calls, including the connection lock. The cgroup limits four CPU cores. | Separate lock-acquisition wall interval, thread CPU interval and cgroup throttle deltas; compare with storage-stage totals. |
| 5 | Remaining active-candidate scans or codec allocation dominate the keyed path. | Pending/Leased selection is ID-ordered; future active entries can still be scanned. Work headers and affected values require codecs. | Aggregate examined/decoded candidate counts, affected record/root counts and codec intervals by fixed operation kind. |

Rank 1 and rank 2 can both be true.
No current measurement distinguishes database commit sync from Rust work inside the same storage call.
Rank 5 is weaker for this seed: each batch drains at most 100 fresh active Work items, while terminal history is excluded.
It remains relevant to the later mixed workload with future Pending and Leased records.

## Source basis

SQLite selects WAL and `synchronous=FULL` in `crates/rom-sqlite/src/store.rs`.
Its standalone Work and Resource bundle paths each commit a native transaction in `persistence.rs`.
The [SQLite synchronous documentation](https://www.sqlite.org/pragma.html#pragma_synchronous) specifies an additional WAL sync for each FULL commit.
It supports testing sync latency as a hypothesis. It does not measure this host.

The [SQLite WAL documentation](https://www.sqlite.org/wal.html) describes checkpoints and their impact on commit latency.
A checkpoint can introduce additional work. Its occurrence and cost in this run are unmeasured.
The [SQLite VFS documentation](https://www.sqlite.org/c3ref/vfs.html) places OS interaction behind its native interface.
Do not add a new VFS wrapper before simpler aggregate tracing establishes a need.

`storage_state/metadata.rs` stores `events: Vec<JournalEvent>` alongside scalar epochs and counters.
SQLite `native_work/read.rs::metadata` decodes that entire object even when the caller requests only retry epochs.
`metadata/bundle.rs::prepare_bundle` clones metadata for its candidate and again for its before-state.
`storage_state/bundle.rs::journal_bundle` serializes event lengths across the vector to enforce its byte limit.
Its retention loop can repeat that fold and shift the vector on removal.
These are bounded retained-journal costs, not remaining full Work-history clones.

The numerical wrapper in `tests/application-load/host/src/observed_storage.rs` does not hold its metrics mutex during adapter I/O.
The fixture's `runner.rs` preserves 10,000 Resources, batches of 100 and the 120-second deadline.
No recommendation here weakens that profile, durable commit or retained history.

## Smallest controlled probe sequence

1. Wait until the redb seed is terminal. Preserve its result, failed store and cleanup evidence.
2. Record the tested binary, source closure, database options, private filesystem mount/device and finite process envelope.
3. Add an aggregate-only diagnostic trial for the owned fixture process and its descendants.
   Limit syscall collection to fsync/fdatasync counts, failures and elapsed wall time.
   Do not capture write buffers, environment, request bodies, SQL values, keys or callback payloads.
   Keep output and deadline bounded. Measure tracing overhead against the same untraced profile.
4. Add fixed-category storage-stage numerical counters if syscall totals cannot explain the boundary cost.
   Separate guard acquisition, metadata decode, shared preparation, native writes and native commit.
   Record counts, total/max nanoseconds and bytes only. Do not persist row or Work identities.
5. Repeat the unchanged seed on the same private filesystem with the same FULL/Immediate durability.
   Record aggregate fsync/fdatasync time and stage timings against the existing operation counts.
6. If filesystem latency dominates, use a separately owned scratch store on a documented comparison filesystem.
   Preserve the same durability and workload. This diagnoses environment cost; it does not erase the original required gate.

Syscall timing can overlap across threads. Do not subtract its summed duration from wall time as an exclusive decomposition.
Stage counters should report their nesting, so their totals are not added as disjoint costs.
A userspace native-commit interval includes more than fsync; a syscall summary excludes serialization and guard waits.

If journal decoding dominates, the next proposal should separate scalar metadata from keyed journal records within the same transaction.
Exact journal positions, floor, byte accounting, retirement, canonical export and predecessor compatibility must remain unchanged.
Do not start that layout change from a hypothesis alone. First obtain numerical stage attribution and negative controls.

## Evidence paths

- Result: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-3c1b19d27a6ffea06af048ec/result.json`.
- Progress/error: the same directory's `process-output.json`.
- Numerical observations: `/var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-3c1b19d27a6ffea06af048ec/source/load-storage-calls.json`.
- Build: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-ac7a8a6f5268055d68fa2030/build-identity.json`.

## Evidence hashes

```text
00c13986407c674e6f3e431a43a2d4f594c81d34a7c6c4aef45970873a0183e3  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-3c1b19d27a6ffea06af048ec/result.json
8824d890a18843a719c2b324a9fa18679fb780f38c4f86dfe26db6de450ceff3  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-3c1b19d27a6ffea06af048ec/process-output.json
76b5e199f59f4eb109728bfdc660e237d62ebe6ca125c16d4c66b6a51fd305c2  /var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-3c1b19d27a6ffea06af048ec/source/load-storage-calls.json
8b52b27ba54269e6ec122d394521913026fa753e499dbcf4e3bd4d27532782f4  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-ac7a8a6f5268055d68fa2030/build-identity.json
247f3e50b5a4043f50360021a4c53c8264abd6d0a97e1e56189362af3cbfb6e5  crates/rom/src/storage_state/metadata.rs
f2ae3f86ca56d8b0a6ccefb2abdbdadfd697fc9978cc16a5a3212303e5b0315f  crates/rom/src/storage_state/bundle.rs
62b7c4809416f57f216afcca6781658ee057411af3ca3ea7f88f017788614211  crates/rom/src/storage_state/metadata/bundle.rs
7accbbc3d319f55e739883aa2d4da5b7140fb2760f1d055ba2760ebf8ca51567  crates/rom-sqlite/src/store.rs
7ff493996f387175fbb36e10fefb6289904a58a394088795279d57bda6baa617  crates/rom-sqlite/src/persistence.rs
d6c5d436f964ffddb4debfba8485f1e719178ee0f18ea4fe908167b52a3bd38b  crates/rom-sqlite/src/native_work/read.rs
```

## Terminal redb result

The redb seed is now terminal. The reviewer inspected its result, numerical observations and progress output.
It uses the same captured optimized binary and unchanged 10,000-row, 100-row-batch, 120-second profile.
The recorded source fences are true before and after execution.
The owned process closed with exit code 1, drained its group and left cgroup population zero.
Recorded OOM and PID-limit events remain zero.

redb created 3,800 Resources at 119,518 milliseconds.
Its last complete batch drained 3,700 Resources at 117,723 milliseconds.
It failed with `finite seed drain deadline`; it did not complete the required seed.
The partial improvement over SQLite does not constitute production throughput admission.

| Operation | Calls after baseline | Total wall seconds | Mean milliseconds |
| --- | ---: | ---: | ---: |
| commit | 3,800 | 58.916712895 | 15.5043981303 |
| Claim | 3,838 | 23.262275255 | 6.0610409732 |
| Materialize | 3,764 | 24.508598291 | 6.5113172930 |
| retry_epochs | 11,400 | 12.961160097 | 1.1369438682 |
| load | 22,728 | 0.087001680 | 0.0038279514 |
| receipt | 11,400 | 0.019965702 | 0.0017513774 |

The summed boundary wall time is 119.755713920 seconds, not exclusive CPU time.
All final failed-call and observer-overflow counts are zero.
Bulk maintenance, queries, operator controls and separate Finish calls again have zero observed calls.

Both adapters therefore remain below the unchanged seed target after native Work separation.
redb's Claim and Materialize means are lower in this run, while its Resource commit mean remains substantially higher.
This strengthens the distinction between keyed Work transitions and Resource/journal publication.
It does not isolate native sync cost from journal codecs, native writes or scheduling.
The ranked hypotheses above remain hypotheses until stage or syscall attribution is available.

### Pending aggregate-only diagnostic trial

The root coordinator reports a finite SQLite diagnostic trial now running under `strace -f -c -w -e trace=fsync,fdatasync`.
Its runner is `/var/tmp/rom-010-native10-sync-diagnostic-20261008.mjs`, with adjacent provenance evidence.
This reviewer did not launch, attach to or alter that process.
Its result is pending. No syscall attribution is claimed from a running trial.
Tracing changes execution cost; even a successful trial is diagnostic evidence, not a new release admission gate.
Preserve its source identity, aggregate output, finite deadline and owned cleanup evidence when it becomes terminal.

### redb evidence identities

```text
454c9919e6c5c05022607de2b40a91770af6384641eadd4932ae9d27f7157fb8  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-4aec219f10242bdf215e17b6/result.json
01341fe3263fbff67b7bb9b7366190ed2823381c4bd9a669e6a755c8ebe97fab  /var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-4aec219f10242bdf215e17b6/source/load-storage-calls.json
2cc7213d74e966c70098831caa2af7df9f187c8bfda0c231d3fa11dfd1a8e286  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-4aec219f10242bdf215e17b6/process-output.json
```

## Terminal aggregate sync diagnostic

The aggregate-only SQLite trial is now terminal. The reviewer inspected its result, metrics and complete aggregate stderr.
It used the same binary identity and unchanged finite seed profile.
The result explicitly sets `acceptance: false` and records unmeasured tracing overhead.
Source fences are true before and after the run.
The owned process closed with exit code 1, drained its group and left cgroup population zero.
OOM and PID-limit event counts remain zero.

The trial created 2,000 Resources at 116,879 milliseconds.
Its last complete batch drained 1,900 Resources at 112,428 milliseconds.
It failed with `finite seed drain deadline`.

The `strace -f -c -w` aggregate records 6,611 `fsync` calls and 89.080815 syscall-wall seconds.
Its mean is 13,474 microseconds per call. The error column is empty, indicating no recorded syscall errors.
No `fdatasync` calls appear in the selected syscall summary.
The captured summary contains counts and timings, not request bodies or write buffers.
This is direct evidence that sync calls consume substantial observed time during this diagnostic trial.

| Operation | Calls after baseline | Total boundary wall seconds | Mean milliseconds |
| --- | ---: | ---: | ---: |
| commit | 2,000 | 53.995212241 | 26.9976061205 |
| Claim | 2,034 | 26.723972285 | 13.1386294420 |
| Materialize | 1,996 | 27.862089869 | 13.9589628602 |
| retry_epochs | 6,000 | 8.725362891 | 1.4542271485 |
| load | 11,992 | 1.155846902 | 0.0963848317 |
| receipt | 6,000 | 0.628698522 | 0.1047830870 |

The summed boundary wall time is 119.091182710 seconds.
All terminal failed-call and observer-overflow counts remain zero.
The syscall summary includes setup and all traced descendants; storage counts subtract the setup baseline.
Therefore, their call counts and intervals are not exactly equivalent scopes.
Sync wall time is nested within storage operations and may overlap across threads.
Do not subtract 89.080815 seconds from the boundary sum as an exclusive CPU or codec decomposition.

The diagnostic performed fewer completed batches than the untraced run.
This is not a regression measurement or an adapter speed comparison.
Tracing overhead and host variation are unmeasured. The load and receipt intervals also increased under tracing.
The trial supports the sync hypothesis; it does not quantify the untraced sync fraction.
Retained-journal metadata growth remains a separate supported source hypothesis.

### Filesystem observation and next controlled step

A narrow read-only `findmnt -T` observation identifies the private volume as `/dev/loop2`, filesystem `ext4`.
Its mount options are `rw,nosuid,nodev,relatime`.
This identifies the mounted filesystem, not the backing device, cache durability, storage hardware or causal latency.
No mount, allocation or tracing change was made by this reviewer.

The next single-variable control should compare the same binary and unchanged workload across documented owned filesystems.
Keep FULL durability, batch size, retained work, database options, finite deadlines and resource envelopes identical.
Use separate fresh stores. Preserve both failed stores and numerical evidence.
Run the filesystem pair without tracing first; otherwise tracing overhead becomes another changing variable.
A comparison on tmpfs would not prove power-loss durability and must not serve as production admission.
If no equivalent durable comparison volume is available, prefer fixed-category stage timing over a weaker storage profile.
Record native commit, metadata decode/preparation and lock intervals without raw identifiers or values.

### Diagnostic evidence identities

```text
28677b7f6769f7f7f057017d34da5cd15fc17e5deada7affc4b433536fa8bdf9  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a66b19bd75f22be6d430d49c/result.json
f74de63d60e2859e2a4342d2e95d6dfaf06a482964b896a39da72da2e0f47a7e  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a66b19bd75f22be6d430d49c/process-output.json
a6a8915c576f90e04b81ce51f2b9157e3ec6047db4f42782898c01cfc74e1905  /var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-a66b19bd75f22be6d430d49c/source/load-storage-calls.json
```

## Matched file-sync control and host-seed harness review

The reviewer inspected `/var/tmp/rom-010-native10-filesystem-sync-control-20261008.json`.
The control records three rounds of 100 fixed 4-KiB overwrites followed by fsync on each filesystem.
It alternates filesystem order and retains six fresh files, 24,576 data bytes in total.
Total recorded duration is 4,901.568593 milliseconds. It explicitly excludes database and seed throughput admission.

| Round | Loop sync total ms | Host sync total ms |
| --- | ---: | ---: |
| 0 | 1226.033644 | 549.899935 |
| 1 | 988.718552 | 464.605741 |
| 2 | 973.157336 | 672.695096 |

The loop filesystem is slower in all three controlled rounds.
This supports a filesystem contribution under the fixed-page workload.
It does not predict the database seed ratio, native transaction overhead or production admission.
Recorded file-control process CPU is small relative to elapsed duration. This does not measure full seed CPU cost.
No durability setting was reduced in this control.

The root coordinator has launched an untraced host-filesystem seed with the same captured binary and unchanged profile.
The reviewer inspected its external runner and namespace bridge without launching another process.
Its result is pending in this review. No host-seed throughput conclusion is made.

### Harness safety and evidence limits

The bridge rejects the shared parent mount namespace before any bind mount.
It verifies closed nonce paths, matching nonce components, real owned 0700 directories and the captured executable path.
The runner creates a fresh host directory. The bridge binds it only within a fresh private mount namespace.
After mounting, the projected directory must match the host directory's device and inode.
The target still uses the original permitted configuration path through that namespace projection.
The inherited private namespace disappears with its owned process group; no shared-host mount is requested.

The outer launcher owns the wrapper identity and cgroup admission before target execution.
It sets a 130-second deadline, finite termination/drain intervals and a 65,536-byte output limit.
The inner binary has a 125-second subprocess timeout; the outer group supervisor remains necessary for stubborn descendants.
The bridge records that mount-tool continuous-birth admission is false. Do not expand this into a stronger process-identity proof.
The preserved negative control proves shared-parent namespace rejection, not every possible path or mount failure.

The 256-MiB directory envelope is planned capacity, not a hard quota.
The runner requires free space but cannot infer a maximum allocation from that check.
Source and preserved-binary hashes are checked before execution and again in final processing.
Preparation failure or a final evidence-write failure can leave no complete result file.
This harness must not be described as guaranteeing evidence after every failure or external interruption.
No blocking namespace/source-safety issue was found for this trusted-owner, finite diagnostic scope.

### Next numerical stage seams

If the host-seed control still fails, the smallest next attribution change is adapter-local fixed-category timing.
Keep instrumentation behind an explicit fixture/test-support opt-in. Do not change the public Resource contract.
Use bounded aggregate counters and checked overflow flags. Never store row, Work, request or receipt identities.
Keep numerical observer bookkeeping outside native I/O and avoid metrics mutexes across transaction operations.

For SQLite `persistence.rs`, separate connection-lock acquisition, native transaction acquisition, metadata decoding, bundle preparation, native publication and commit.
For standalone Work, separate header/affected-record reads plus shared preparation, delta publication and native commit.
For `retry_epochs`, isolate complete metadata decoding from lock acquisition.
For redb, use the corresponding writer-gate, transaction, metadata, shared preparation, publication and Immediate commit seams.
Record fixed operation category, count, total/max nanoseconds and optional encoded metadata/event counts only.

The first stage pass should time existing boundaries. It should not add a new journal representation or recalculate new graph summaries.
`metadata/bundle.rs::prepare_bundle` contains candidate metadata copying and journal work.
A second narrower pass can separate copying from `journal_bundle`, if that existing stage is dominant.
Current serialization lengths can contribute numeric byte counts; do not serialize a second time merely for measurement.
Nested measurements must identify their scope. Do not sum inclusive preparation and its substage as separate elapsed costs.
Maintain a disabled-instrumentation control and report observer overhead before treating timings as optimization evidence.

```text
450d3f0df41029bf4e1ceffc60d62f11ca86049e78bfcb5762b4273b374ff916  /var/tmp/rom-010-native10-filesystem-sync-control-20261008.json
75c45dabf6ed66e404236fe53a3a7bc7d7007aebd57d41c576fb796dc3e88520  /var/tmp/rom-010-native10-host-filesystem-seed-20261008.mjs
18e2f18ef3475b9b2f23bf6330d4e33bc3cbfd6b865edcc3fd95f4122b019736  /var/tmp/rom-010-native10-host-filesystem-bridge-20261008.mjs
522c4283ab35eadbd96cca3c2f5ce688c7845657a1eb76a4fca9ad4283d0b4d2  /var/tmp/rom-010-native10-host-filesystem-negative-20261008.json
```

## Terminal matched host-filesystem SQLite seed

The untraced host-filesystem diagnostic is now terminal. It failed at the unchanged create deadline.
The reviewer inspected the result, progress, namespace witness and numerical observations.
The same captured binary and 10,000-row, 100-row-batch, 120-second profile were used.
The diagnostic result explicitly sets `acceptance: false`.

The last complete batch drained 3,200 Resources at 118,985 milliseconds in progress output.
The numerical batch checkpoint records 118,986 milliseconds, a separate observation one millisecond later.
The terminal storage counter records 3,229 successful Resource commits after baseline subtraction.
Do not describe the partially created next batch as fully drained.

| Operation | Calls after baseline | Total boundary wall seconds | Mean milliseconds |
| --- | ---: | ---: | ---: |
| commit | 3,229 | 58.416049764 | 18.0910652722 |
| Claim | 3,264 | 21.745443134 | 6.6622068425 |
| Materialize | 3,200 | 18.965418103 | 5.9266931572 |
| retry_epochs | 9,687 | 18.093492600 | 1.8678117683 |
| load | 19,316 | 0.617388781 | 0.0319625586 |
| receipt | 9,687 | 0.361339056 | 0.0373014407 |

Summed boundary wall time is 118.199131438 seconds, not exclusive CPU time.
The final observed failed-call and overflow counters are zero.
The failed fixture deadline remains distinct from an adapter call failure.
Source fences are true before and after execution.
The owned process closed with exit code 1, drained its group and left cgroup population zero.
The recorded OOM and PID-limit counters remain zero.

### Namespace evidence

The witness records different parent and child mount namespace identities.
It records a successful bind from host device 66306, inode 74757827.
The projected permitted path has that exact device and inode.
Its original directory had device 1794, inode 162984.
The process membership matches the recorded owned cgroup.
These observations support a private filesystem projection. They do not provide continuous-birth admission for the mount helper.
The coordinator separately checked the original parent namespace view remained unchanged; that check is owner-reported here.

### Interpretation and next stage

This diagnostic reached more fully drained batches than the original loop-filesystem run.
It still failed the required seed. No general database speed ratio or release admission follows from the comparison.
Runs occurred at different times; host variation is not controlled by one pair.
The host trial's Claim and Materialize means are lower, while metadata epoch-read cost remains substantial.
The larger retained journal also differs at the terminal sample. Compare equal batch positions before attributing that growth.
Moving the database alone does not close the workload target.

The root has assigned test-support-only numerical stage instrumentation to the SQLite owner.
The intended seams are lock, transaction acquisition, metadata decode, shared preparation, native publication and native commit.
The reviewer has not yet inspected that implementation or its tests.
The proposed instrumentation does not authorize changes to persistence semantics, format or durability.
It should preserve nested timing scope, numerical-only aggregates and a disabled observer control.
A subsequent source review must precede treating those counters as trustworthy attribution.

### Host diagnostic evidence identities

```text
89db848f526f349f91e48307a8683c3727a8bebe4ea886bfe634176a707cd2b0  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-cf7500321f5b232c73bcf0ef/result.json
f67ee2ed657c46ebc88e4b597cd8041dc773a69cb7ad2e78c0ac66a400e4822f  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-cf7500321f5b232c73bcf0ef/process-output.json
0cc5f435a16d96a51f015a452c663a293cba472fa0cdf3d6eae0858c28826147  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-cf7500321f5b232c73bcf0ef/host-filesystem-namespace.json
12402f10190b8c54725f57fb831caf8901fe985bbf7e179975093e5280d4707c  /var/tmp/rom-owned-host-seed-cf7500321f5b232c73bcf0ef/source/load-storage-calls.json
```

## Initial stage-instrumentation source review

This section reviews the current interface and timer placement. Implementation and native verification remain in progress.
The earlier 382-input match applied to the terminal seed candidate before these intentional source edits.
It must not be read as a current hash match for the instrumented source.

`stage_observation.rs` exposes three fixed operation categories and six fixed stage categories.
The aggregate contains exactly 18 entries, count/elapsed/dropped/saturation fields and a poison-recovery flag.
It stores no Resource, Work, receipt, request or callback values.
`observe_stages` uses OnceLock opt-in and returns a shared observation handle.
The test-support module, store field, imports and timers are absent when that feature is disabled.
With test-support enabled but observation disabled, the timer uses a lazy optional initializer and does not read a clock.

RAII timer completion records early returns and unwinding.
Checked sample and nanosecond addition preserve the prior totals on overflow and mark dropped/saturated observations.
The observation mutex is not held during native I/O.
Timer completion itself takes that mutex briefly, often while the database guard remains held.
This opt-in measurement therefore adds synchronization overhead; it is not a nonblocking production exporter.
No callback or observer Result can change the adapter's domain Result.

The inspected timer placement preserves transaction and checkpoint order.
Connection acquisition, Immediate transaction acquisition, preparation, native publication and native commit occur in their prior order.
Post-commit acknowledgement checks remain after native commit timing ends.
The new script comparison covers enabled versus disabled outcomes for success, rollback, replay and committed lost acknowledgement.
That maintained test is source evidence until its actual native result is available.

### Exact category meaning

WorkUpdate has no separate MetadataRead timer in this snapshot.
Its header, record and root reads/codecs occur during SharedPrepare.
Apply-time header and read-fact rechecks occur during NativePublication.
A zero WorkUpdate MetadataRead cell therefore does not mean there were no metadata or codec reads.

Commit SharedPrepare also includes receipt arbitration, Resource load/revision checks and reference preparation.
It is a broad supervised preparation interval, not pure shared-core CPU time.
RetryEpochs MetadataRead times complete StorageMetadata decoding after connection-lock acquisition.
NativePublication includes codecs, native writes and test checkpoints. NativeCommit excludes the subsequent acknowledgement simulation.
Publish these category scopes with the numerical result before drawing codec-versus-sync conclusions.
Top-level category intervals do not overlap in normal success; their external storage-boundary totals include them.
Early returns can also include unwind cleanup within the currently active timer.

### Fixture and admission behavior

The fixture's optional `storage-stage-timings` feature enables the SQLite test-support dependency.
The default fixture does not enable observations or write stage evidence.
The stage-diagnostic build uses a distinct optimized target directory and identifies its scope as diagnostic only.
Stage evidence contains fixed enum names and numerical counters, with `acceptance: false`.
No raw callback, query, row or identity data is introduced.

Eager `proof.and(write_bounded(...))` attempts the stage write even when the existing call-evidence write fails.
The final match preserves an original workload error when either evidence write also fails.
On workload success, an evidence write failure still fails preparation.
This does not guarantee evidence after early setup failure, process kill or disk failure.

The root corrected successful diagnostic status from `passed` to `diagnostic-complete`.
It retains `acceptance: false`; uninstrumented preparation still uses `passed` on success.
This removes the ambiguity where a status-only reader could mistake diagnostic completion for the admission gate.
No downstream admission was launched. The workload gate remains failed.
Node parse and 33 load tests are owner-reported at this checkpoint; Rust compilation and fresh native tests are pending.

No new persistence, format, checkpoint-order or Result-preservation blocker was found in this source snapshot.
The exact stage meanings above remain necessary when the measurements are interpreted.

```text
1b53dbcbd4767aa69d933e8b0e058c83f7f32e072a40d4ae9cc75a2d7d4e3bfa  crates/rom-sqlite/src/stage_observation.rs
bc494f0bb02beb07dd6ca9c968498ca88bcef7295d8083906e016e9619d7b795  crates/rom-sqlite/src/stage_observation/timing.rs
a943f1d68179ad1740e276886e00dd0f2f6e4cb43beed6173980dcbaf795cdf5  crates/rom-sqlite/src/stage_observation_tests.rs
db25efbcbe0ecd9e4038740dc6ec3a2bee1bbac7ed8429b4a3a2e434e2303ac3  crates/rom-sqlite/src/persistence.rs
0ea707781169f9082c12560dc561cbbde627c5819271a8ca96339c87c89192cd  crates/rom-sqlite/src/store.rs
5a1cb3729a7a3cd06b1a2a49f73e2fc021d7c24936896326607ca69bbe930a14  crates/rom-sqlite/src/lib.rs
807c0ac85618b8353c52422f857034598a347b944053ae48e7e84086001ba25f  tests/application-load/host/Cargo.toml
2d53b98fe232907ba0d9ff640229d6e8099f3deda912d9ba7101c8d7ebf510cf  tests/application-load/host/src/lib.rs
874a61a478d90a0da679da2bb649ef52dca4cda9d9a6f84f41e58cc4dd238dfc  tests/application-load/host/src/stage_measurements.rs
3227097963916f0e0f9b987637660f945af6a5b3e0351d5a7fca235cd6bdbbff  tests/application-load/host/src/runner.rs
07748eb9427cebb02778956a325b0ddd4337e43d41e04a840343e6ac1a59eee0  tests/application-load/build-plan.mjs
0301a8dcbf3944e2efe660dca41c67cbc2166e77e498a15bb76148c40972bbcd  tests/application-load/build.mjs
3b6fdda1d6d25edb93f593e6b65a396f3918e7602a781a2f34e3661cf2c8da5b  tests/application-load/prepare-run.mjs
```

The root's subsequent `stage_scope` strings are now present in emitted stage evidence.
The reviewer compared them with current timer placement.
They correctly identify StorageMetadata-only decoding, broad shared preparation, publication read-fact checks and post-commit acknowledgement outside commit timing.
This resolves the category-interpretation concern in the source. Actual stage measurements and compiler gates remain pending.

## Frozen SQLite observer: final local source and gate review

The SQLite owner has frozen its source and released its native lease.
The reviewer inspected the completed observer, timers, overflow/poison controls, outcome-equivalence tests and raw gate logs.
No native commands, probes or product edits were performed.

The audit is `/var/tmp/rom-010-sqlite-stage-observation-source-gate-20261009.json`.
Its recorded date is 2026-10-08. The filename's `20261009` suffix is preserved as evidence, not corrected.
It explicitly describes an after-command source identity, not a before-command capture.
The reviewer recomputed all 43 recorded source/input hashes and nine log hashes: zero mismatches.
This supports current source correspondence to the owner's captured gate, with the stated after-command provenance limit.
It does not establish a before/after compiled-source fence for those commands.

The final test-support library raw log has 49 passed tests, zero failed, in 0.72 seconds.
The default library raw log has 36 passed tests, zero failed, in 0.59 seconds.
Both all-target Clippy logs show successful completion, in 0.78 and 0.44 seconds respectively.
The audit records exit code zero for the scoped formatting check; its raw formatting log is empty.
An empty formatting log alone is not proof of exit status. The exit status comes from the audit record.

The original behavioral negative control compiled and failed its intended stage-count assertion.
That is stronger than a missing-module setup failure.
The final maintained controls demonstrate the following local behaviors:

- Enabled and disabled observation produce equal native receipts, Work records and administrative counts for the scripted outcome sequence.
- Success, rollback, receipt replay and committed lost acknowledgement reach the expected stage counts.
- Rejected shared Work preparation adds a prepare sample without adding publication or commit samples.
- Disabled timers have no clock sample or aggregate handle.
- Numeric overflow drops the complete sample without wrapping prior totals or growing category cardinality.
- Poisoned observation recovers and does not reject a subsequent actual native commit.

The counter update computes checked sample/time additions before replacing either total.
Overflow keeps prior totals and sets saturation plus a saturating dropped-sample count.
The poison path retains the fixed aggregate and records recovery.
No callback, payload, user-defined label or Result channel exists in the observer.
The lock order does not create a source-visible cycle: snapshot takes only the observation mutex, never the database mutex.
Native I/O occurs without the observation mutex held.
Timer completion briefly acquires that mutex while database guards can remain held, so measurement overhead is still unmeasured.
The observer is not claimed to be a production nonblocking exporter.

Feature-off source removes observation fields, imports and timers through cfg.
Feature-on disabled timers do not read a clock or record samples.
The native transaction and fault-checkpoint order remains unchanged in the inspected source.
Actual persisted-header/read-fact validation still precedes delta writes.
Post-commit acknowledgement checks remain outside the NativeCommit timer.
The emitted fixture stage_scope strings correctly describe broad preparation and publication intervals.

No new preservation, overflow, poison or lock-order blocker was found for this frozen observer source.
The outcome tests are finite scripted coverage, not proof of every possible failure interleaving.
The diagnostic feature consumer and optimized stage build are separate root-owned work in progress.
No instrumented full-seed measurements or measured observer-overhead bound are available in this section.
A diagnostic success must remain `diagnostic-complete`, never release `passed`.
The unchanged release workload gate is still failed, and default admission must use the uninstrumented build.

```text
5e134d28e9883a93d24671c649154570a679272d8ef929733ce509409bce65bd  /var/tmp/rom-010-sqlite-stage-observation-source-gate-20261009.json
820f76b3dc172e25c98cd2a20379c255c6faa0eb67e31a379f15369983b8dc43  /var/tmp/rom-010-sqlite-stage-observation-all-lib-final-20261009.log
6f402b8939055f378c04f258e0f970e2f3febf1ba2db11e3daaa2ec49d5be66f  /var/tmp/rom-010-sqlite-stage-observation-default-lib-20261009.log
bf33022ca61935eb68c322ea8b8b0539bab14ee4d6340111553c35b8c254cac8  /var/tmp/rom-010-sqlite-stage-observation-clippy-20261009.log
d97f96afb1ca68e5e0dd0a9bfbf49b7dbf8921f81de738734e0bdf78c5a834ba  /var/tmp/rom-010-sqlite-stage-observation-default-clippy-20261009.log
14dd7e3fe5f6a78d1b9899c068fd7657b5eff17417a3307052415dfc9be590d1  crates/rom-sqlite/src/stage_observation/counter_tests.rs
bc494f0bb02beb07dd6ca9c968498ca88bcef7295d8083906e016e9619d7b795  crates/rom-sqlite/src/stage_observation/timing.rs
3227097963916f0e0f9b987637660f945af6a5b3e0351d5a7fca235cd6bdbbff  tests/application-load/host/src/runner.rs
874a61a478d90a0da679da2bb649ef52dca4cda9d9a6f84f41e58cc4dd238dfc  tests/application-load/host/src/stage_measurements.rs
3b6fdda1d6d25edb93f593e6b65a396f3918e7602a781a2f34e3661cf2c8da5b  tests/application-load/prepare-run.mjs
```

## Terminal fixed-stage diagnostic

The instrumented optimized SQLite trial is now terminal. The reviewer independently recomputed its numerical stage differences.
The result records a failed drain deadline, unchanged source fences and owned cleanup with cgroup population zero.
It created 2,200 Resources at 119,519 milliseconds. Its last complete batch drained 2,100 at 116,923 milliseconds.
The diagnostic explicitly excludes acceptance and records observer overhead as unmeasured.

Its separate optimized build contains 387 captured source/input identities.
The reviewer recomputed those 387 current hashes: zero mismatches at this checkpoint.
The preserved binary matches SHA-256 `907302a3e5457b3ac588590343658eedc55f79e8db631454ea7696253b433fff` and size 24,486,024 bytes.
It is scoped `storage-stage-diagnostic-only`, not the default admission binary.

### Recomputed stage attribution

These totals subtract each matching baseline entry from its final entry.
The labels are the previously reviewed broad wall-clock stages, not exclusive CPU measurements.

| Operation and stage | Samples after baseline | Elapsed seconds |
| --- | ---: | ---: |
| Commit / connection lock | 2,200 | 0.000063065 |
| Commit / transaction begin | 2,200 | 0.007407735 |
| Commit / metadata read | 2,200 | 1.429765674 |
| Commit / shared prepare | 2,200 | 2.588525454 |
| Commit / native publication | 2,200 | 1.213283598 |
| Commit / native commit | 2,200 | 43.248034681 |
| WorkUpdate / connection lock | 4,306 | 0.000376276 |
| WorkUpdate / transaction begin | 4,306 | 0.033010006 |
| WorkUpdate / shared prepare | 4,306 | 0.405251569 |
| WorkUpdate / native publication | 4,306 | 0.227359415 |
| WorkUpdate / native commit | 4,306 | 62.402274084 |
| RetryEpochs / connection lock | 6,600 | 0.000220774 |
| RetryEpochs / metadata read | 6,600 | 6.376582435 |

The remaining category cells have zero samples.
No observation reports dropped samples, saturation or poison recovery.
WorkUpdate MetadataRead remains zero because its codecs reside inside preparation and publication.

NativeCommit totals 105.650308765 seconds.
All observed stage intervals total 117.932154766 seconds.
NativeCommit is therefore about 89.586 percent of those observed stage intervals in this trial.
This is not a CPU share, syscall-only share, total-process decomposition or production performance bound.
The separately observed storage-boundary calls include these stage intervals and must not be added to them.
Observer recording, fixture processing and unmeasured intervals are outside some stage timings.

### What NativeCommit measures

The reviewed timer starts immediately before `tx.commit().map_err(|_| Error::Unknown)?`.
It ends immediately after the call returns successfully; the post-commit acknowledgement checkpoint follows outside it.
Shared preparation, native publication, read-fact rechecks and pre-commit checkpoints are outside this successful interval.
The adapter does not perform new journal serialization or Work preparation inside that timer.

The interval still includes SQLite transaction finalization, pager/WAL work, possible checkpoint work, native sync and host scheduling delay.
It is not equivalent to fsync time alone.
A failed native commit could also include transaction error/cleanup work; this trial has zero failed observed storage calls.
The earlier aggregate fsync diagnostic is separate execution evidence and cannot be substituted for a syscall breakdown of this trial.

### Decision supported by the result

Native durable commit is the largest measured stage in this instrumented workload.
This strengthens the case for researching durable transaction grouping while preserving the original model.
It does not justify weakening FULL durability, acknowledging before durability, removing retained history or changing the deadline.
The existing metadata decode cost remains measurable, particularly on retry_epochs, but it is smaller than NativeCommit in this trial.
No disabled-observer matched overhead control has been executed for this instrumented build.
The workload gate still fails; no optimization is admitted solely from these diagnostics.

Any grouping proposal must preserve exact Work policy, coherent read facts, atomic prefix behavior, receipts, events and current claims.
Independent actions must receive truthful outcomes after the shared native transaction becomes durable.
A group failure must not expose a prepared but uncommitted success or duplicate an unknown committed action.
These are research constraints, not approval of an unimplemented batching API.

### Fixed-stage diagnostic evidence identities

```text
c54736d96e7b5fae7cca760b981a08303962ba828ad9c92b35454ca79198c23b  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a266d623db66470be8e99989/result.json
133c3d6de23c55a8073074dd47c3148c48f9ef02cec7c1fcea84283e409a4f07  /var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-a266d623db66470be8e99989/source/load-storage-stages.json
6abc1770da53be70e37f3a71eb6343ba2296489c907734da2ba3f657b3a19be6  /var/tmp/rom-010-authentik-20261007/run/volume/private/application-recovery-a266d623db66470be8e99989/source/load-storage-calls.json
91981ebf457bedc359eec588c8b0d2e2a1dcd715d5fff8e82fec6ad80e3c6ab3  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a266d623db66470be8e99989/process-output.json
99a59dbdcdc3e8983ca71b073f84fd41dcc31cd36478636f9865bf59fb1ae41d  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-c9b022cd679579754243e118/build-identity.json
```

## Terminal host-filesystem fixed-stage trial

The reviewer independently inspected the host stage result, progress, namespace witness and both numerical observation files.
This trial used the same captured c9 build and `907302...` diagnostic binary as the loop stage trial.
All 387 current source/input hashes and the preserved binary still match at this review checkpoint.
It remains a diagnostic profile, with acceptance false and observer overhead unmeasured.

The trial failed at the unchanged create deadline.
It committed 4,249 primary Resources after baseline subtraction.
Its last complete batch drained 4,200 Resources at 119,191 milliseconds.
Do not describe the next partial batch as fully drained.
The result records true source fences before and after, process exit 1, owned group drain and zero cgroup population.
Recorded OOM and PID-limit event counts remain zero.

| Operation and stage | Samples after baseline | Elapsed seconds |
| --- | ---: | ---: |
| Commit / connection lock | 4,249 | 0.000104801 |
| Commit / transaction begin | 4,249 | 0.012329357 |
| Commit / metadata read | 4,249 | 3.688375814 |
| Commit / shared prepare | 4,249 | 5.754228953 |
| Commit / native publication | 4,249 | 2.562279187 |
| Commit / native commit | 4,249 | 39.831457582 |
| WorkUpdate / connection lock | 8,484 | 0.000320568 |
| WorkUpdate / transaction begin | 8,484 | 0.039754257 |
| WorkUpdate / shared prepare | 8,484 | 0.459414714 |
| WorkUpdate / native publication | 8,484 | 0.313920882 |
| WorkUpdate / native commit | 8,484 | 48.194176183 |
| RetryEpochs / connection lock | 12,747 | 0.000386356 |
| RetryEpochs / metadata read | 12,747 | 15.657736854 |

The remaining category cells have zero samples.
No observation has dropped samples, saturation or poison recovery.
The emitted category scopes match the reviewed instrumented source.

NativeCommit totals 88.025633765 seconds.
The sum of observed stage intervals is 116.514485508 seconds.
NativeCommit is about 75.549 percent of that stage sum, not an exact share of total process wall time or CPU.
The separate storage-boundary total is 119.365593571 seconds and includes these stages.
Do not add boundary and stage totals or treat their difference as an exclusive CPU attribution.

The host stage trial reached more complete batches than the loop stage trial, but neither met the required target.
They occurred at different times, with different terminal retained-journal sizes and unmeasured observer overhead.
These observations support durable native commit and metadata decoding as material costs.
They do not establish a general filesystem speed ratio or prove any group-commit optimization.
Compare equal completed batches when examining retained metadata growth.

### Harness and namespace scope

The reviewed host-stage harness requires `build.plan.diagnostic === true` before launch.
It retains the prior fresh private mount namespace, owned cgroup, 130-second outer deadline and finite output limits.
Successful diagnostic completion would use `diagnostic-complete`, not `passed`.
Planned directory capacity remains 256 MiB, not a hard quota.
The namespace witness records a host device/inode of 66306/74623298 projected exactly onto the allowed private path.
The original path was 1794/162997; parent and child namespace identities differ.
The recorded cgroup membership matches the result's owned group.
No claim of continuous-birth mount-tool admission is added.

No Cargo command, probe, source edit or new allocation was performed by this reviewer.
The next root-owned WAL-attribution research remains separate from this executed evidence.
The default uninstrumented seed and final release gates remain unfulfilled.

### Host stage evidence identities

```text
caa9f05c10111c244b1cb5769a5fd1e39f2f79fe163616aea0caa2d2aa86e8a2  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-51ea749afbe4a0cf4cf50dbf/result.json
824a9e1c4b3923b3c7ec8b0509a890301fe73f53e746a17bd6eac85c9f37d5bf  /var/tmp/rom-owned-host-seed-51ea749afbe4a0cf4cf50dbf/source/load-storage-stages.json
5f31a53cff156b6bff8e6c021b67e2646a44f92248680da75e7fc6858831bf85  /var/tmp/rom-owned-host-seed-51ea749afbe4a0cf4cf50dbf/source/load-storage-calls.json
27ee444ecbad020ea3c9a3efb6a7a11d95702fd59958631c313d870fe7378c1b  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-51ea749afbe4a0cf4cf50dbf/process-output.json
f73591410457051c2d611f955642b92a623c4056df6b07ee12d94ed96beb3fd6  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-51ea749afbe4a0cf4cf50dbf/host-filesystem-namespace.json
```

```text
7e77d3a12ee870a2c33a36a2e78b9feea5dd932fd26818252ed8f371e527a829  /var/tmp/rom-010-native10-host-stage-seed-20261008.mjs
```

## Independent WAL-prefix review, 8 October 2026

### Reviewed evidence and lifecycle

This review inspected the preserved WAL probe, both numerical summaries, the namespace harness and the version 2 parser.
The probe directory is `/var/tmp/rom-owned-wal-amplification-50dc2af04abb8342a8acc136`.
The scripts are in `/var/tmp/rom-wal-trace-control-20261009/`.
No native command, new probe or product edit was performed by this reviewer.

The result records `diagnostic-failed`, `acceptance: false` and `monitor-fail-closed`.
Execution lasted 44.191 seconds, from 14:43:38.612Z to 14:44:22.803Z.
The trace has 16,782,826 bytes: **5,610 bytes** above its 16 MiB monitored boundary.
The aggregate monitor observed a maximum of 26,753,688 bytes across 441 samples.
These sampled limits are not a hard aggregate quota.

The result records SIGTERM shutdown, a drained process group and an empty owned cgroup.
Its memory and task-limit counters record no OOM or task-limit event.
The harness selected the fresh store's exact `database-wal` path.
Its raw positional-write mode prints numerical arguments and a pointer address, without reading the pointed-to buffer.
It installed no WAL hook and invoked no checkpoint API.

The result records both historical source fences as true.
It identifies the preserved diagnostic binary as `907302a3e5457b3ac588590343658eedc55f79e8db631454ea7696253b433fff`.
These terminal fences do not establish a match against subsequent workspace formatting or source edits.
No current full-verifier result is inferred from this probe.

### Independent numerical reconstruction

A separate read-only Node reducer parsed the complete preserved trace with anchored numerical patterns.
It verified thread association for unfinished and resumed calls.
It verified each frame-header/page pair against the same thread, descriptor and offset difference of 24 bytes.
It printed numerical aggregates only, without trace buffers or configuration contents.

| Independent observation | Value |
| --- | ---: |
| Complete successful positional-write returns | 264246 |
| Cumulative returned write bytes | 544087176 |
| WAL-header writes | 128 |
| Frame-header writes | 132059 |
| Corresponding full-page writes | 132059 |
| Header/page association mismatches | 0 |
| Unpaired frame headers at end | 0 |
| Short, failed or differently shaped writes | 0 |
| Distinct positional-write descriptors | 1 |
| Successful fsync returns | 4672 |
| Final fsync with unknown return | 1 |
| Unfinished/resumed syscall pairs | 1 / 1 |
| Pending partial calls at end | 0 |
| Terminal signal records | 8 |

All numerical values were safe integers.
The trace contains 268,928 lines and ends with a newline.
Its eight non-numerical terminal records comprise one SIGTERM delivery and seven killed-by-SIGTERM records.
The sole unfinished/resumed pair is the final fsync; its return is `?` and is not counted as successful.
There are no remaining unclassified records after this reconstruction.

The byte total also satisfies the observed write-shape identity:

```text
132059 * (24 + 4096) + 128 * 32 = 544087176
```

This identity alone would not prove correct pairing.
The separate thread, descriptor and offset checks provide the additional association evidence for this trace.

### Interpretation of the parser flags

The version 2 parser sets `lost_or_truncated_indicator` when it sees killed-by-signal text.
Thus `complete_numeric_parse: false` is a conservative terminal-interruption classification here.
It does not identify an observed malformed or missing positional-write return in this particular file.
The independent reconstruction supports the cumulative total for **complete successful returns present in the preserved prefix**.
It does not support a complete workload trace.

The parser's `all_pwrite_returns_parsed` flag checks only `malformed_pwrite === 0`.
It does not reject missing thread prefixes, orphan resumed lines or pending positional writes through that flag.
Its `header_page_pair_counts_match` flag compares counts; it does not verify individual thread, descriptor or offset association.
Its positional-write regex is not anchored at the end, and resumed syscall names are not checked against their pending names.
These are parser hardening requirements before reuse, rather than demonstrated byte errors in this captured trace.

A subsequent parser gate should reject those cases and distinguish terminal signals from unknown numerical records.
It should check cumulative integer arithmetic and classify partial writes separately.
Tests should include orphan resumes, mismatched syscall names, malformed suffixes, interleaved threads and a pending positional write at termination.
Preserve both existing summary versions and the original trace when adding that gate.

The safe claim is **544,087,176 bytes returned by observed successful WAL positional writes in a bounded diagnostic prefix**.
The number is not unique WAL length, committed event bytes or block-device write traffic.
Repeated offsets, overwritten WAL generations and uncommitted spill can contribute to the cumulative count.
The evidence does not assign traffic to Resource state, Work, retained journal metadata or checkpoints.
Do not divide it by a progress line to infer cost per committed Resource.
Do not infer a production throughput limit or an improvement from the tracing run.

### Priority before another native-layout change

The highest-priority next step is targeted attribution of retained journal publication, rather than approval of format 11 from this byte total.
Use a separately reviewed, opt-in numerical fixture to count serialized metadata bytes and retained journal entries at equal operation checkpoints.
Compare equal completed batches, with fixed inputs, unchanged atomic publication, full durability and the existing deadline.
Exclude identifiers, event contents and raw buffers from these counters.
These measurements identify logical publication growth; they must not be labelled physical WAL-page attribution.

If that evidence supports a journal-specific correction, test a small keyed-journal/header prototype against the existing canonical-state oracle.
Require identical receipts, event order, revisions, retention, recovery and unknown-outcome behavior before measuring its write traffic.
A retained-work improvement must not remove history or relax durability to meet the workload.
A bounded layout comparison can then justify the migration cost and format decision.
This review does not authorize or claim implementation of that prototype.

### Evidence identities

```text
f3da5b530ac34ed8aa55efc07643315a2841e434df1abded40280fbb0c762d1d  /var/tmp/rom-owned-wal-amplification-50dc2af04abb8342a8acc136/result.json
74d1d6ce72378d93fb522814794676613ed9f0ecfdf001552c15f593e14a3d21  /var/tmp/rom-owned-wal-amplification-50dc2af04abb8342a8acc136/numeric-wal-summary-v2.json
4fe36a43204350a27a5c09af7d4976a51ddfff0e55d47eee2fc75e04de4cb711  /var/tmp/rom-owned-wal-amplification-50dc2af04abb8342a8acc136/numeric-wal.trace
2ecbbdbecc6582da0b9f9c443c2d18190691d225cbf2114a1221b3dd2fee14d1  /var/tmp/rom-wal-trace-control-20261009/aggregate-v2.mjs
e806f08af88372f4cb87eded2fc7a35c3cbb924a4214eb71bb138d6f370c03a5  /var/tmp/rom-wal-trace-control-20261009/probe-namespace.mjs
```

## Task 0 publication consumer source review

This checkpoint reviewed `tests/application-load/host/src/publication_measurements.rs`, its maintained tests and the runner's feature wiring.
It also inspected the fixed SQLite publication snapshot shape and checked the existing compact evidence writer.
No native command, workload or product edit was performed by this reviewer.
The SQLite observer owner was still completing its gate; this is not final source acceptance.

No launch-blocking consumer defect was found in the reviewed source.
The schema contains ten fixed operation/category cells, rather than Resource identifiers or payload contents.
The cells combine Commit and WorkUpdate with Metadata, EventPayload, WorkHeader, WorkRecord and WorkRoot.
They do not represent all database writes: Resource rows, receipts, keys and index overhead are outside this measurement.
Make that exclusion explicit in the emitted scope before comparing it with WAL traffic.

Each batch snapshot follows the complete reaction drain and the existing storage-call batch record.
The snapshot reads observer counters; it adds no storage operation.
The workload retains 100 batches of 100 rows, the 120-second deadline and existing candidate/work limits.
Feature-off compilation excludes the consumer module and all publication snapshot calls.
The diagnostic feature remains SQLite-only, with redb rejected by the existing stage-enablement boundary.

The compact batch representation stores ten pairs of cumulative sample and byte totals.
It limits the batch vector to 100 entries and rows to 10,000.
The maintained envelope test uses maximum `u64` values for every pair and both detailed endpoint snapshots.
It serializes through `serde_json::to_vec`, as does the production evidence writer, and requires at most 65,536 bytes.
The writer checks the encoded length before opening its create-new output file.
The overflow test preserves the existing 100 completed batch entries when another record is rejected.

The runner eagerly attempts calls, stages and publication evidence writes.
An earlier evidence-write error does not prevent the later write attempt.
If the workload failed, the final match returns that original workload error, even if evidence writing also failed.
If the workload succeeded, evidence-writing failure remains a failure.
Early database-open, runtime-construction and identity-seed failures occur before this final evidence path.
Killed processes do not have a guaranteed final publication snapshot.

The validity flag checks baseline and final poison recovery, dropped samples and saturation.
The underlying observer retains these flags cumulatively, so a completed-batch counter failure also invalidates the final comparison.
The emitted byte scope correctly includes successful SQL statements later rolled back.
It explicitly rejects durable-commit-byte and WAL-byte interpretations.
It records no page, checkpoint, committed-transaction or whole-database traffic attribution.

Before using the flag for admission, add direct tests for clean `batch_comparisons_usable: true`.
Also test each baseline and final poison, dropped-sample and saturation condition separately.
The existing maximum-value test includes all three invalidity conditions but does not assert the validity flag itself.
These are targeted coverage improvements, not observed failures in this consumer path.

The actual feature consumer log `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-publication-consumer-green-20261008.log` contains 17 passing tests.
The adjacent `rom-010-publication-consumer-clippy-20261008.log` ends with successful release-profile completion but includes a SQLite unused-import warning.
That Clippy command is not a warning-free dependency gate.
The concurrent default gate and final SQLite observer tests were not verified by this checkpoint.
No fresh workload result, format change, performance improvement or release admission is claimed.

## Actual Task 0 publication trial: independent evidence review

The reviewed trial is `application-load-seed-7b1a9997aa0760c97a5d58b3`.
Its private HOST store is `/var/tmp/rom-owned-host-seed-7b1a9997aa0760c97a5d58b3`.
This review read the actual result, process output, calls, stages and publication records.
It launched no workload and changed no product source.

The result is `failed`, with stderr reporting `finite seed drain deadline`.
It retains the original 10,000-row profile, 100-row batches and 120-second seed deadline.
Fifteen batches fully drained: 1,500 rows at 115,293 milliseconds.
The next 100 rows were created at 118,871 milliseconds; their drain did not complete before failure.
The final Work counters include 64 additional materializations beyond the completed 1,500-row checkpoint.
Thus the final totals must not be presented as another fully drained checkpoint.

The process exited with code 1 and drained its owned group.
The result records an empty cgroup, no OOM or task-limit event, and both terminal source fences true.
The directory allowance remains planned at 256 MiB, not a hard quota.
The diagnostic record explicitly has `acceptance: false`.

The build identity contains 393 source hashes and binary SHA256 `4d6eec7143bc3c711840547bf7ca5ee62d666e10971f7fbb37701ee3d7942854`.
This reviewer independently matched all 393 hashes against source at this review checkpoint.
The preserved binary also matched its recorded hash and 24,511,864-byte size.
A later source edit would require a fresh closure; this statement is not a continuous source fence.

### Equal completed-batch attribution

The following values subtract the recorded identity baseline.
The completed 1,500-row checkpoint is compared with the previous completed 1,400-row checkpoint for the last-batch column.

| Selected SQL payload category | Through 1,500 drained rows, bytes | Last completed 100 rows, bytes |
| --- | ---: | ---: |
| Commit metadata | 328670214 | 42213300 |
| Commit event payload | 197250 | 13150 |
| Commit Work header | 644989 | 43400 |
| Commit Work record | 1217250 | 81150 |
| Commit Work root | 70500 | 4700 |
| WorkUpdate Work header | 1304516 | 87668 |
| WorkUpdate Work record | 2520000 | 168000 |
| WorkUpdate Work root | 141000 | 9400 |

WorkUpdate metadata and event-payload cells remain zero.
The first completed 100-row batch published 1,641,631 metadata bytes.
The fifteenth published 42,213,300 metadata bytes for the same 100 Commit samples.
Each completed batch published 13,150 event-payload bytes.
This demonstrates increasing selected metadata publication volume, rather than an increase in the number of Commit samples per completed batch.

The baseline-subtracted final metadata total is 373,788,514 bytes across 1,600 Commit samples.
It includes the partly drained final batch.
It is not comparable to the 1,500-row completed checkpoint as an equal completed workload.
All publication snapshots have zero dropped samples, no saturation and no poison recovery.
The record marks `batch_comparisons_usable: true`, consistently with these flags.

### Source attribution and remaining uncertainty

The metadata observer counts the already encoded `StorageMetadata` string after successful SQL publication.
The current metadata structure contains retained `events: Vec<JournalEvent>` alongside counters, limits, epochs and generation data.
The selected category growth therefore supports a concrete metadata-republication concern.
The counter does not separately measure each metadata member.
It cannot alone quantify the retained journal's exact contribution within that encoded string.

The new record explicitly excludes Resource, receipt, effect, row-key and index writes.
Successful SQL statements later rolled back remain included.
The record measures neither durable commit bytes nor physical WAL or block-device traffic.
There is no joined per-operation WAL attribution in this trial.
The earlier WAL prefix and this trial support separate observations; they do not establish an exact causal byte ratio.

The final stage totals still place substantial wall time within native transaction commit.
Commit native-commit time is 56.289215393 seconds after baseline subtraction.
WorkUpdate native-commit time is 52.611657776 seconds.
Those intervals include native commit work and waiting; they are not exclusive CPU or fsync-only time.
The new observer's overhead and HOST variation have not been isolated by a matched control.
Completing 1,500 rows here versus 4,200 in the prior HOST stage trial is therefore not a measured performance regression ratio.
It is also not evidence of a speed improvement.

This evidence is sufficient to prioritize an **additive canonical-oracle prototype** for keyed journal/header publication.
Require exact state, receipt, event, revision, retention and recovery equivalence before adapting production storage.
Compare publication volume at equal completed checkpoints, then run the unchanged durable workload with controlled measurement conditions.
No format 11 marker, weakened deadline or release admission follows from this diagnostic result.

### Consumer corrections and gate scope

The current consumer scope now lists excluded write categories explicitly.
The maintained tests assert invalidity in the maximum-capacity case.
A separate table test covers a clean valid case and baseline/final poison, dropped-sample and saturation invalidity independently.

The actual `rom-010-publication-consumer-reviewed-green-20261008.log` in the native container contains 18 passing tests.
The corresponding reviewed Clippy log ends with successful release-profile completion and no warning text.
These supersede the earlier consumer warning checkpoint for that scoped gate.
They do not establish a new full verifier result or production load acceptance.

### Task 0 evidence identities

```text
737be0abfefc31f333279b6efee5723a2c6099015aece7e9c07fea4547d1036d  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-818d165119f6f3fd0e733e2d/build-identity.json
3421c299f61849cc374a65ee249894bc7b9e0f542324ed6c7ebb4da50a4b29c3  /var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-7b1a9997aa0760c97a5d58b3/result.json
afb058ae85eafae09e13887cee948e9a909d854f3a556e53e6467f05f62d7230  /var/tmp/rom-owned-host-seed-7b1a9997aa0760c97a5d58b3/source/load-storage-publication.json
2b2a7814a8e17216b806ce5b209ba94ac62826748f94126c27abd144fe02f835  /var/tmp/rom-owned-host-seed-7b1a9997aa0760c97a5d58b3/source/load-storage-stages.json
eab7268476f16fbed2b443a2bda9793b014d768fd6ed26787bee5274a1470135  /var/tmp/rom-owned-host-seed-7b1a9997aa0760c97a5d58b3/source/load-storage-calls.json
```
