# Retained work latency implementation plan

> **For agentic workers:** Use superpowers:subagent-driven-development or superpowers:executing-plans. Assign separate file ownership before implementation.

**Goal:** Pass the unchanged 10000-Resource and 10000-retained-work admission workload without weakening ROM's durable semantics.

**Architecture:** Keep transitions and accounting in the driver-free shared model. Persist only affected work and root records inside the existing native transaction. Reconstruct canonical logical state at archive boundaries.

**Tech stack:** Rust, existing SQLite/redb adapters, the maintained application-load fixture and current backup/upgrade contracts.

This is a proposed correction, not an implemented layout or measured speedup.
The [independent investigation](../../research/rom-0.1.0-persistence-latency-strategy-independent-review-2026-10-08.md) establishes current failures and constraints.

## Boundaries and ownership

| Boundary | Existing files | Proposed responsibility |
| --- | --- | --- |
| Attribution fixture | `tests/application-load/host/src/observed_storage.rs`, `runner.rs` | Fixed call counts, durations and transparent outcomes |
| Shared transition | `crates/rom/src/reaction_work/ledger.rs`, `storage_state/work.rs` | Validated changed records and root accounting, without copying unrelated history |
| Incremental model | New `crates/rom/src/reaction_work/incremental.rs` and separate tests | Driver-independent transition input and delta; preserve public legacy paths |
| SQLite persistence | `crates/rom-sqlite/src/persistence.rs`, `store.rs`; new `work_records.rs` | Native work/root records and claim index in the existing transaction |
| redb persistence | `crates/rom-redb/src/storage.rs`, `state.rs`, `format.rs`; new `work_records.rs` | The same logical delta in native tables |
| Archive and upgrade | `crates/rom-backup/src/model.rs`, adapter snapshot/maintenance/migration/upgrade modules | Explicit layout conversion and canonical reconstruction |

The attribution agent owns only its existing fixture and controllers.
Root retains ownership of product code until the causal measurement and interface review complete.
Assign SQLite and redb separately after the shared contract freezes. Keep integration and compatibility ownership explicit.
Do not place implementation in `lib.rs` or `mod.rs`.

## Edge cases

- A retained Done record remains available after another record changes.
- A stale claim cannot complete a newer lease or consume another root's budget.
- An overflow or rejected transition leaves the original state unchanged.
- DeliveryStarted commits before any external attempt; an unknown delivery remains recoverable.
- A failed commit and an unknown acknowledgement have different outcomes.
- A current permission denial cannot disclose an old receipt or work projection.
- Restored claims and journal cursors retain their generation fences.
- A legacy archive remains byte-identical; conversion writes a fresh destination.
- An old binary refuses an incompatible new native layout.
- Retention updates its accounting and indexes atomically without silently deleting additional history.

## Task 1: Attribute the existing cost

- [x] Write failing fixed-counter tests for result transparency, reentrant calls and arithmetic overflow.
- [x] Implement fixed names, checked totals and maxima around inner Storage calls.
- [x] Release observer locks before each inner call. Preserve its returned Result.
- [x] Use the wrapper during preparation, not only the HTTP serving phase.
- [x] Emit bounded numeric batch summaries under the existing output and time limits.
- [x] Repeat both optimized seeds with the same 10000 target, retained records and 120/130-second boundaries.
- [ ] Separate storage-call time from serialization, sync and scheduler hypotheses.
- [ ] If needed, measure codec CPU on detached copies. Do not label it live write-byte accounting.
- [ ] Obtain independent review before selecting the product correction.

The instrumentation passed 15 maintained tests and scoped Clippy. The [independent attribution review](../../research/rom-0.1.0-load-storage-attribution-independent-review-2026-10-08.md) confirms these five fixture steps. Both optimized attribution probes finished with deadline failures. The [attribution report](../../research/rom-0.1.0-retained-work-attribution-2026-10-08.md) records baseline-subtracted timings and the selected correction. Boundary timings can overlap; they are not exclusive codec CPU or encoded write bytes. Terminal metrics are attempted after ordinary preparation errors. Setup errors or process termination can prevent them.

## Task 2: Share incremental transitions

- [ ] Specify the smallest transition input and delta after the attribution result. Do not duplicate policy in adapters.
- [ ] Add differential tests against the current logical model for all WorkUpdate variants.
- [ ] Cover roots, retry epochs, limits, revisions, claim generations and operator receipts. Include bundle completed-work transitions, exact receipt arbitration and composite revision-once behavior.
- [ ] Compute exact current and lifecycle-reserved canonical JSON totals incrementally, including map keys, commas, envelope and frozen limits. Compare both totals against full serialization in differential tests.
- [ ] Ensure ordinary transitions do not clone, serialize or scan unrelated terminal records.
- [ ] Keep archive validation as a full integrity check outside the ordinary mutation path.
- [ ] Preserve existing public paths. Add narrow modules and dedicated tests.
- [ ] Freeze the shared interface and obtain review before adapter edits.

The [incremental contract investigation](../../research/rom-0.1.0-incremental-work-contract-2026-10-08.md) proposes transaction-local reads and opaque validated deltas. It is source-only. Product interface selection and independent review remain open. Full operator listing and archive reconstruction retain their explicit bounded observation costs.

### Accounting checkpoint

The accounting helper currently compiles only in tests. Six differential accounting tests passed against whole canonical serialization.
Three reference tests passed against preserved pre-incremental sources, including the orphan-root control.
An earlier relevant Work run passed 41 tests before that third reference case was added.
Scoped Clippy passed after correcting a test initializer; its initial failure remains in the evidence.
This checkpoint does not prove production integration, a full 42-test rerun or a latency improvement.

Evidence: `/var/tmp/rom-010-work-accounting-final-focused.log`, `/var/tmp/rom-010-work-accounting-reference-three.log` and `/var/tmp/rom-010-work-accounting-frozen-source-hashes.txt`.

## Task 3: Persist the same delta in both adapters

- [ ] Write real-adapter tests that change one record while retaining many unrelated terminal records.
- [ ] Apply Resource, receipt, event, effect, work and root changes in the existing atomic transaction.
- [ ] Preserve SQLite durability and redb Immediate durability.
- [ ] Preserve lexicographic work-ID Claim ordering and atomic preceding expiry/reconciliation effects. Measure native entries examined; a time-ordered index alone does not prove bounded minimum-eligible-ID selection.
- [ ] Inject failure between native writes, before commit and after committed acknowledgement loss.
- [ ] Reopen and compare canonical logical state with the shared model.
- [ ] Introduce explicit layout-version refusal before accepting new tables.
- [ ] Convert copied predecessors into fresh destinations; reconstruct canonical archives without touching historical evidence.
- [ ] Repeat the actual predecessor Runtime/custom-codec replay proof and current backup/restore tests.
- [ ] Verify application rollback independently of data restoration.

## Task 4: Re-run release acceptance

- [ ] Repeat the unchanged seed and complete bounded mixed HTTP load on both adapters.
- [ ] Measure latency, queue age, RSS, disk/WAL growth and records or bytes touched where actually instrumented.
- [ ] Inject disk-full, slow readers, provider unavailability and process interruption.
- [ ] Repeat diagnostics overhead after the persistence correction.
- [ ] Run affected conformance, public consumers, compile fixtures and the full local verifier.
- [ ] Complete independent review before integration.
- [ ] Build and verify matching clean-source Rust/Studio artifacts.
- [ ] Re-run extracted consumer and deployment acceptance before source/tag publication.

No checklist item is complete merely because the source suggests it should work.
A smaller seed, weaker durability, discarded history or wider deadline cannot substitute for the selected acceptance workload.

## Coherent bridge checkpoint

The shared WorkImage bridge currently compiles only in tests. Its nine tests passed after two genuine regression failures.
A no-write enqueue had lost its record preconditions. An Idle delta could cross distinct contexts with equal accounting totals.
Separate read facts now survive write filtering. An opaque ReadFence associates a delta with its originating reader context.
The bridge checks the context, header, record facts and root facts before publication. Successful no-op publication also replaces the context fence.
Same-context altered-record and altered-root controls exercise exact facts independently of foreign-context rejection.
The fence does not establish database generation, authorization or lock-free MVCC. Native adapters still require the original coherent writer transaction.
Active-ID selection excludes terminal history but can still scan all active records. No latency improvement is claimed.

Evidence: `/var/tmp/rom-010-work-bridge-readset-red.log`, `/var/tmp/rom-010-work-bridge-context-red.log` and `/var/tmp/rom-010-work-bridge-fence-green.log`.
Affected Work tests, Clippy, production integration and the full verifier remain separate gates.

## Native format selection

Select native format 10 with canonical archive version 7. Explicitly accept existing archive-7/storage-9 backups through the same canonical validator.
Keep the actual imported manifest marker. New backups use storage marker 10 after both native layouts are implemented.
The [independent format review](../../research/rom-0.1.0-incremental-format-decision-review-2026-10-08.md) describes the source consequences and negative controls.
This selection is not an implemented format bump or compatibility test result.
Both adapters still need native-9 conversion, exact inventory checks, bounded canonical reconstruction and separate physical metadata accounting.

### Affected verification

A fresh root run passed 57 affected Work tests and all-target Clippy with warnings denied.
Evidence: `/var/tmp/rom-010-root-work-fence-affected-20261008.log` and `/var/tmp/rom-010-root-work-fence-clippy-20261008.log`.
These are scoped checks, not the full verifier or production adapter acceptance.
An earlier root export attempted to read two agent logs from the wrong container location.
Its host redirections truncated the named final-focused log and source inventory. That export failure is not test evidence.
The fresh root logs use distinct paths and explicit source-file existence checks before export.

## Production support implementation checkpoint

`rom::storage_support` now exposes named trusted Work and metadata support modules.
`StorageMetadata` separates bounded metadata from retained Work and operator collections. Canonical split/reassembly preserves the archive representation.
The new bundle coordinator uses shared counter and journal helpers from the existing StorageState bundle path.
It prepares completion, counters, enqueue and journal changes without cloning retained Work or operator history.
`BundleDelta` remains opaque and has no serialized authority. Native adapters must publish all its parts in the original writer transaction.

Three metadata tests passed while two bundle tests failed against an explicit unimplemented publication stub.
After implementation, all five passed. The successful composite case matches the existing canonical completion-plus-enqueue result.
The failure case confirms that a receipt limit does not publish completion or counter changes.
The missing-API compile run also reported an accounting Copy error, corrected by the shared-support owner before behavioral verification.

Evidence: `/var/tmp/rom-010-native-bundle-api-red-20261008.log`, `/var/tmp/rom-010-native-bundle-publication-red-20261008.log` and `/var/tmp/rom-010-native-bundle-publication-green-20261008.log`.
Public-editor ignored-error protection, full affected core checks and independent support review are still pending at this checkpoint.
Neither database adapter uses the new native layout yet. This checkpoint does not demonstrate a load improvement or release readiness.

## Subsequent production and SQLite verification

The production core passed 150 library tests. All-target Clippy passed with warnings denied.
WorkEdit now retains the first error. An ignored error cannot permit partial publication.
StorageMetadata checks retry admission before Resource revision checks. Saved receipts bypass stale claims but not future retry epochs.
WorkImage validates native headers, root summaries and active identities against the canonical ledger.

Root inspected the host logs `/var/tmp/rom-010-work-production-core150-host.log` and `/var/tmp/rom-010-work-production-alltarget-clippy-host.log`.
The backup library passed 32 tests and all-target Clippy. Archive reads accept only archive-7/storage-9 and archive-7/storage-10 through the current canonical validator.
The original manifest marker remains present. No failed current decode selects a legacy decoder.
Evidence: `/var/tmp/rom-010-backup-native-marker-green-20261008.log` and `/var/tmp/rom-010-backup-native-marker-clippy-20261008.log`.

SQLite passed 39 library tests, including ten new native-layout tests. All-target Clippy passed with test-support and warnings denied.
These tests cover retained terminal history, native corruption, write interruption, acknowledgement loss, file reopening, archive restoration and format-9 conversion.
Root inspected the container logs `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-sqlite-native-file-tests-after-lock-20261008.log` and `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-sqlite-native-clippy-20261008.log`.

Both adapter sources select local native format 10. The shared archive writer marker remains 9 during integration.
The redb integration test run is pending. Shared conformance, unchanged load acceptance and the full verifier remain pending.
These scoped results do not establish latency improvement, accepted-source compatibility or release readiness.

## Shared legacy fixture verification

The first mixed migration run failed because the fixture treated bounded metadata as the old complete StorageState.
The fixture now exports canonical state through the public adapter backup boundary before it creates the old native layout.
It removes the new derived Work tables and split state entries. The old fixture retains its original format-specific state conversion.
This test conversion does not prove compatibility with an independently built predecessor artifact.

After the correction, schema migration passed six tests and retention passed seven tests.
Each suite reported one ignored subprocess entry point. Their parent tests invoke those entry points.
All-target conformance Clippy passed with warnings denied after removal of an unused import.
The first implementation compile attempt had two setup errors: a private close call and a missing trait import.
Those errors remain in the original log. They are not behavioral test results.

Evidence is in the container-mounted `/var/lib/nixos-containers/rom-dev/var/tmp/` directory:

- `rom-010-shared-legacy-layout-red-20261008.log`: genuine missing canonical Work fixture failure.
- `rom-010-shared-legacy-layout-green-20261008.log`: implementation compile errors.
- `rom-010-shared-legacy-layout-repaired-20261008.log`: the targeted migration passed.
- `rom-010-shared-legacy-affected-20261008.log`: both affected suites passed.
- `rom-010-shared-legacy-clippy-20261008.log`: unused-import failure.
- `rom-010-shared-legacy-clippy-repaired-20261008.log`: all-target Clippy passed.

The first redb integration also passed three library tests and all-target Clippy.
Additional native invariant tests remain queued. The format-8 mixed fixture still needs full layout conversion before its migration gate.
The unchanged load, full verifier, independent native review and release artifacts remain pending.

### Format-8 fixture correction

Both format-8 tests initially failed against relabelled current tables. The valid upgrade returned Storage; the invalid-field fixture could not find canonical Work.
The shared fixture now supports format 8 without removing operator data, Work revisions or delivery profiles.
The format-8 suite uses this complete old-layout conversion instead of changing only the marker.
Its two tests passed. The retained schema and retention suites also passed: six and seven tests respectively.
All-target conformance Clippy passed with warnings denied.

Container evidence is available through `/var/lib/nixos-containers/rom-dev/var/tmp/`:

- `rom-010-shared-format8-layout-red-20261008.log`: both original fixture failures.
- `rom-010-shared-format8-layout-green-20261008.log`: two format-8, seven retention and six schema tests passed.
- `rom-010-shared-format8-clippy-20261008.log`: all-target Clippy passed.

The native review identified separate remaining gates: caller pre-decode operator limits and redb fault injection at each new Work write.
Configured maintenance budgets must remain distinct from the caller's logical Work snapshot budget.
Synthetic old-layout tests do not replace the accepted predecessor executable, cutover and rollback trials.

## Shared native marker integration

A new writer test failed because a newly created archive identified storage format 9 instead of 10.
The shared STORAGE_FORMAT is now 10. Current export assertions now require archive-7/storage-10.
Existing archive-7/storage-9 read fixtures remain unchanged. Their imported manifests retain marker 9.
The backup library passed 33 tests after this correction. The two format-8, seven retention and six schema tests also passed.
Combined all-target backup and conformance Clippy passed with warnings denied.
An initial Clippy run detected duplicate loading of a test helper. The SQLite test now imports the shared helper instead.

Evidence is in `/var/lib/nixos-containers/rom-dev/var/tmp/`:

- `rom-010-native-writer-marker-red-20261008.log`: actual writer marker mismatch.
- `rom-010-native-writer-marker-green-20261008.log`: all 33 backup tests passed.
- `rom-010-native-marker-migration-green-20261008.log`: all three affected migration suites passed.
- `rom-010-native-marker-clippy-20261008.log`: duplicate test-module failure.
- `rom-010-native-marker-clippy-repaired-20261008.log`: combined all-target Clippy passed.

The adapter fault and operator budget corrections remain in progress. No unchanged load result or final verifier result is claimed here.

## Tooling and Studio checkpoint during native integration

The Node acceptance command ran 318 tests in the rom-dev container. All passed without skipped tests.
An earlier host run failed 21 tests because rustc was absent from the host PATH; 297 tests passed.
The host failure remains in `/var/tmp/rom-010-node-pre-native-acceptance-20261008.log`.
The successful container log is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-node-native-environment-20261008.log`.
These results cover the Node portion of scripts/check, not the Rust portion or the complete verifier.

Studio svelte-check reported zero errors and zero warnings.
Evidence: `/var/tmp/rom-010-studio-typecheck-native-integration-20261008.log`.
This is type-check evidence, not a browser or human usability result.

The active load, recovery, maintenance-portal and AI-flow lockfiles now include redb's direct serde dependency.
No package version, source or checksum changed in that lockfile correction.
Locked native consumer builds remain a separate gate.

## Full conformance integration attempts

The first full native-10 conformance attempt stopped at native_profile.
The extension manifest still declared native format 9, while the compiled storage contract required 10.
The manifest now declares 10. No historical archive marker changed in that correction.

The next full attempt stopped at lost_target_ack_is_done_atomically_and_restart_does_not_repeat.
New Work acknowledgement checkpoints caused its armed fault to affect Claim before the intended target mutation.
The injector now passes the Claim acknowledgement and fails the following target bundle acknowledgement.
It asserts that the injector reached that phase. All nine runtime reaction tests passed after the fixture correction.
The original assertions still require atomic Done state, one target mutation and no repeated action after restart.

Container logs are under `/var/lib/nixos-containers/rom-dev/var/tmp/`:

- `rom-010-native10-full-conformance-first-20261008.log`: extension profile mismatch.
- `rom-010-native10-full-conformance-profile-repaired-20261008.log`: target acknowledgement phase mismatch.
- `rom-010-native10-target-ack-phase-green-20261008.log`: nine runtime reaction tests passed.

Neither stopped full attempt proves complete conformance. The remaining redb bulk-read budget correction precedes the next full run.

### Complete shared conformance checkpoint

The next full rom-storage-conformance command completed with exit status 0.
Evidence: `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-native10-full-conformance-second-repaired-20261008.log`.
This run includes both real database adapters. Explicit historical-writer trials remain ignored until their separate evidence inputs are supplied.
The redb bulk-read regression failed before correction. Its configured budget now applies before decoding the validated canonical Work projection.
All 14 redb library tests and all-target/all-feature Clippy passed after that correction.

Studio's unit command passed 417 tests with no failures or skipped tests.
Evidence: `/var/tmp/rom-010-studio-unit-native-integration-20261008.log`.
These tests do not replace browser, installed-consumer or human usability acceptance.

The unchanged load consumer release build has a new 382-file source capture.
Preparation: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-ac7a8a6f5268055d68fa2030/preparation.json`.
Its compiler command permits two jobs, disables incremental compilation and has a 900-second deadline.
The planned shared target allowance is 8 GiB; it is not a filesystem quota.
No seed, load or performance result is claimed until the build and its source fence pass.

### Captured optimized consumer build

The release consumer build completed in 42.75 seconds with exit status 0.
Build finalization verified the 382-file source fence and retained a detached private executable.
Its size is 24,441,032 bytes. Its SHA-256 is `cf152939ff7c2a301678c7550825047e26a91a51b5aeb924d6495445d5df387d`.
Build identity: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-ac7a8a6f5268055d68fa2030/build-identity.json`.
The unchanged SQLite seed is now a separate execution gate. A successful build does not prove seed admission or throughput.

### Native-10 seed failures and diagnostic admission

Both unchanged seeds failed at the 120-second drain deadline.
SQLite created 2,500 Resources; its last complete batch drained 2,400 Resources at 116,257 milliseconds.
Redb created 3,800 Resources; its last complete batch drained 3,700 Resources at 117,723 milliseconds.
Neither result admits the later mixed workload or establishes production throughput.

Both results retain matching source fences and drained process groups.
The failed databases, progress, observations and result files remain available.
The result directories are under `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/`:

- SQLite: `application-load-seed-3c1b19d27a6ffea06af048ec/result.json`.
- Redb: `application-load-seed-4aec219f10242bdf215e17b6/result.json`.

SQLite's observed commit calls consumed 48.930103959 seconds of boundary wall time.
Claim consumed 31.696568233 seconds; Materialize consumed 31.854038213 seconds.
These inclusive intervals do not identify exclusive CPU time or database sync time.
The bulk maintenance read counters are zero for this seed.

The next diagnostic uses the same private binary, fresh SQLite data and the unchanged workload.
An owned launcher runs `strace -f -c -w -e trace=fsync,fdatasync` with the original finite process envelope.
It retains the 120-second fixture deadline and 130-second launcher deadline.
Its cgroup retains four CPU cores, 4 GiB memory and 512 process entries.
The aggregate trace does not collect payload buffers. Trace overhead remains unmeasured.
This execution is a diagnostic trial, not an acceptance result.

The external harness is `/var/tmp/rom-010-native10-sync-diagnostic-20261008.mjs`.
Its adjacent `.provenance.json` records the input and harness hashes and the three deliberate changes.
The harness does not change product source, durability or the captured source closure.
Its output is `/var/tmp/rom-010-native10-sqlite-sync-diagnostic-20261008.log`.
The result remains pending until the owned process ends and its drain evidence passes.

### Aggregate sync diagnostic result

The owned diagnostic ended with the same finite drain failure. Its process group drained to zero.
Both source fences passed. It created 2,000 Resources and fully drained the preceding 1,900.
The aggregate trace recorded 6,611 `fsync` calls and 89.080815 seconds of syscall wall time, with no syscall errors.
It recorded no `fdatasync` calls. Tracing overhead remains unmeasured.
These totals establish significant sync waits in this trial, not an exclusive decomposition of the original seed time.

Diagnostic evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-a66b19bd75f22be6d430d49c/result.json`.
The adjacent `process-output.json` preserves the progress and aggregate summary.
No original failed database was changed by this diagnostic.

A separate file-sync control used six fresh files, with 24,576 total data bytes.
It repeated 100 fixed 4-KiB page writes and `fsync` calls per filesystem in three rounds.
The filesystem order alternated. No database, secret or Resource data entered these files.
The private loop filesystem's sync totals were 1226.034, 988.719 and 973.157 milliseconds.
The host filesystem's sync totals were 549.900, 464.606 and 672.695 milliseconds.
The complete control ended in 4.902 seconds and preserved all files.
Evidence: `/var/tmp/rom-010-native10-filesystem-sync-control-20261008.json`.

`findmnt` identifies the private filesystem as `/dev/loop2`, ext4, with `rw,nosuid,nodev,relatime` options.
The host filesystem is ext4 with `rw,relatime` options.
This file control supports a filesystem contribution to sync latency on this host.
It does not establish database throughput, a successful host-filesystem seed, or a general performance ratio.
The original 10,000-Resource acceptance gate remains failed.

### Matched host-filesystem seed

The untraced SQLite comparison used the same captured binary and unchanged seed profile on fresh host-filesystem data.
The owned bridge created a private mount namespace with private propagation before its bind operation.
It verified closed nonce paths, directory ownership, permissions and the projected device and inode.
A negative probe rejected the parent namespace before any mount operation.
The parent namespace retained its original directory device and inode during the seed.
No other process's mount view changed.

The comparison failed at the finite create deadline. It fully drained 3,200 Resources at 118,985 milliseconds.
The numerical observations record 3,229 seed commit calls after setup subtraction.
Source fences passed before and after execution. The owned process group drained to zero.
This result does not meet the 10,000-Resource gate.

The observed host-filesystem intervals include 58.416049764 seconds for commit and 18.093492600 seconds for retry_epochs.
Claim consumed 21.745443134 seconds; Materialize consumed 18.965418103 seconds.
The comparison supports a filesystem contribution, but it does not establish a general adapter performance ratio.
The remaining metadata and preparation costs need finer numerical attribution.

Evidence: `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-seed-cf7500321f5b232c73bcf0ef/result.json`.
Its adjacent namespace witness and process output remain available.
Fresh data and measurements remain under `/var/tmp/rom-owned-host-seed-cf7500321f5b232c73bcf0ef`.
The bridge and harness provenance remain under `/var/tmp/rom-010-native10-host-filesystem-*20261008*`.
The 256-MiB data allowance was a plan, not a hard quota.

### Opt-in storage-stage attribution

The next diagnostic separates connection lock, transaction begin, metadata read, shared preparation, native publication and native commit.
The SQLite owner implements fixed counters behind `test-support`; production storage decisions and transaction boundaries remain unchanged.
The load fixture enables those counters only through its optional `storage-stage-timings` feature.
Its diagnostic build uses a separate target and explicitly records a diagnostic scope.
Default release build commands do not enable this feature.

The build separation test first failed because the diagnostic compiler profile did not exist.
It then passed with the separate profile, explicit feature flag and unchanged default release command.
Evidence: `/var/tmp/rom-010-stage-build-separation-{red,green}-20261008.log`.
Rust instrumentation, consumer compilation and actual stage measurements remain pending at this checkpoint.
The earlier captured binary evidence remains valid for its terminal runs; new source edits require a fresh build capture.

### Completed storage-stage trials

The optional SQLite instrumentation passed 49 feature-enabled library tests and 36 default library tests.
Both adapter Clippy commands passed. These checks do not prove the full release gates.
The captured diagnostic consumer compiled in 147 seconds and records 387 source inputs.
Its binary SHA-256 is `907302a3e5457b3ac588590343658eedc55f79e8db631454ea7696253b433fff`.

The loop-filesystem trial failed the unchanged finite drain deadline.
It created 2,200 Resources and fully drained 2,100 Resources.
Primary native commits consumed 43.248034681 seconds; WorkUpdate native commits consumed 62.402274084 seconds.
Their sum is 105.650308765 seconds of the 117.932154766 observed stage seconds.
These intervals include database finalization, possible checkpoints, sync and scheduling.
They are not exclusive CPU measurements. Observer overhead remains unmeasured.

The host-filesystem trial also failed the unchanged finite create deadline.
It fully drained 4,200 Resources at 119,191 milliseconds and recorded 4,249 primary commits after setup subtraction.
Primary native commits consumed 39.831457582 seconds; WorkUpdate native commits consumed 48.194176183 seconds.
RetryEpochs metadata reads consumed 15.657736854 seconds.
Both trials retained matching source fences and drained process groups, with no dropped or saturated stage observations.

The captured build identity is under `application-load-build-c9b022cd679579754243e118/build-identity.json`.
The result paths under `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/` are:

- Loop filesystem: `application-load-seed-a266d623db66470be8e99989/result.json`.
- Host filesystem: `application-load-seed-51ea749afbe4a0cf4cf50dbf/result.json`.

The consumer subsequently passed 15 release-profile tests with the diagnostic feature enabled.
Its all-target release-profile Clippy command passed with warnings denied.
Container logs are `/var/tmp/rom-010-stage-consumer-tests-20261008.log` and `/var/tmp/rom-010-stage-consumer-clippy-20261008.log`.
Their host copies are accessible under `/var/lib/nixos-containers/rom-dev/var/tmp/`.
These commands reused the separate diagnostic target with two build jobs and a 900-second command timeout.
The target's 8-GiB allowance remains a plan, not an enforced disk quota.

Naive Claim prefetch and deferred Materialize can change ordering, lease timing and committed-prefix behavior.
The group-commit report therefore does not approve either technique as a transparent optimization.
Journal layout research also identifies a limit: WorkUpdate commits do not rewrite the journal.
The next diagnostic must distinguish WAL write amplification from native sync costs before another layout change.
No release admission or native-format change follows from these measurements.

### Current verifier and WAL prefix checkpoint

Strict OpenSpec validation passed all 17 items.
The first current `./scripts/check` passed 319 Node tests, then failed the workspace format gate.
Its log is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-current-full-verifier-20261008.log`.
Workspace formatting corrected five files without changing behavior or public API paths.
This changes their source hashes. It does not invalidate historical terminal trials with matching before/after source fences.
The next full verifier uses a new log: `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-current-full-verifier-formatted-20261008.log`.
Its result remains pending at this checkpoint.

The numerical WAL diagnostic stopped at its trace monitor threshold after approximately 44 seconds.
Its owned process group drained. Both source fences passed before the later formatting changes.
The parser records 544,087,176 cumulative successful WAL write bytes across repeated offsets and WAL resets.
It identifies 132,059 paired frame headers and aligned 4,096-byte page payloads.
These are syscall write counts, not committed event counts or retained file size.
The trace monitor sampled 5,610 bytes beyond its 16-MiB threshold before stopping the owned group.
The diagnostic has no final stage snapshot and cannot establish bytes per completed Resource or per operation category.

Evidence is `/var/tmp/rom-owned-wal-amplification-50dc2af04abb8342a8acc136/numeric-wal-summary-v2.json` and the adjacent `result.json`.
The independent WAL investigation owns the parser and interpretation review.
The observed traffic supports further write-amplification investigation; it does not establish the proposed journal layout's performance.
The unchanged workload gate remains unmet.

### External compile fixtures restored

The formatted full verifier progressed through workspace tests, documentation, core isolation and the public consumer.
It then failed because `tests/compile/Cargo.lock` did not contain the core's current HMAC dependency.
Cargo stopped before the expected macro diagnostics. This was an integration failure, not a valid negative fixture result.
The failure remains in `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-current-full-verifier-formatted-20261008.log`.

Offline checks refreshed `tests/compile/Cargo.lock` and `tests/compile-renamed/Cargo.lock`.
Both changes add `hmac` 0.12.1 and `subtle` 2.6.1 plus their dependency edges.
No existing package version changed. The original lockfiles remain under `/var/tmp/rom-010-*-fixture-lock-before-20261008.lock`.
The repaired `node tests/compile/check.mjs` command passed with `--locked` Cargo invocations.
It confirmed 38 intended failures at their expected primary source lines and completed its positive fixtures.
Its log is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-compile-fixture-gate-repaired-20261008.log`.

The complete verifier runs again under the same two-job, no-incremental, finite-timeout settings.
Its new log is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-current-full-verifier-locks-repaired-20261008.log`.
That run remains pending at this checkpoint. Earlier partial checks are not a complete verifier result.

The subsequent run reached the external AI consumer, then stopped at the maintenance portal's stale lockfile.
Its failure remains in the `rom-010-current-full-verifier-locks-repaired-20261008.log` log.
The portal's offline all-feature check added the same HMAC dependency edges without changing existing package versions.
Its previous lockfile is `/var/tmp/rom-010-portal-lock-before-20261008.lock`.
The successful check log is `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-portal-lock-refresh-20261008.log`.

Locked offline metadata resolution subsequently passed for all five external manifests used by the verifier:

- `tests/compile/Cargo.toml`.
- `tests/compile-renamed/Cargo.toml`.
- `tests/ai-tools-compile/Cargo.toml`.
- `examples/ai-flows/Cargo.toml`.
- `examples/maintenance-portal/Cargo.toml`.

This check resolves dependencies; it does not execute their behavior tests.
The new full run writes `/var/lib/nixos-containers/rom-dev/var/tmp/rom-010-current-full-verifier-all-active-locks-20261008.log`.
Its result remains pending. No release admission follows from the lockfile repairs.

### Complete verifier and publication attribution implementation

The complete `./scripts/check` run ended with exit zero before the publication-counter changes.
Its log is `rom-010-current-full-verifier-all-active-locks-20261008.log` under the host-mounted container temporary directory.
The log SHA-256 is `e0818f4bb86e2e973407a7572f7a82d25c8ca60b04f15f362c44ec7d9e0c1edb`.
`/var/tmp/rom-010-full-verifier-post-source-20261008.json` records command identity and a selected post-command source snapshot.
It is not a before/after fence or clean-source artifact admission.
The source inventory contains 2,189 selected files, 20 omitted entries and one tracked deletion.
The subsequent diagnostic changes require renewed affected checks and integration verification.

Task 0 now records successful SQL publication bytes in ten fixed operation/category cells.
Metadata includes retained journal data; event payload, Work header, Work record and Work root have separate cells.
The counter uses each already-encoded String length. It does not serialize a second copy.
Successful statements remain counted if a later checkpoint rolls back their transaction.
Failed SQL and receipt replay do not add publication samples.
Resource, receipt, effect, key and index writes are outside this diagnostic's selected categories.
The counter does not measure durable commit bytes or physical WAL traffic.

SQLite passed 55 test-support tests, 36 default tests and both direct all-target Clippy gates.
The consumer passed 18 feature-enabled tests, 15 default tests and its reviewed all-target Clippy command.
The earlier dependency warning remains in its original log; the final adapter and consumer checks have no such warning.
Consumer tests cover 100 completed batches within the 64-KiB evidence limit and invalid comparisons after recorder faults.
The reviewed consumer writes `load-storage-publication.json` without changing the original seed profile or stage timing fields.

A new diagnostic binary compiled in 11.68 seconds and captures 393 source inputs.
Its SHA-256 is `4d6eec7143bc3c711840547bf7ca5ee62d666e10971f7fbb37701ee3d7942854`.
Its captured identity is `/var/tmp/rom-010-authentik-20261007/run/volume/evidence/application-load-build-818d165119f6f3fd0e733e2d/build-identity.json`.
The original untraced host-filesystem diagnostic uses this new binary and the unchanged finite profile.
Its result remains pending. No native-layout change or performance acceptance follows from compiling the instrumentation.

### Terminal attribution and additive journal prototype

The diagnostic subsequently ended with exit one at the unchanged seed deadline.
Its result is `application-load-seed-7b1a9997aa0760c97a5d58b3/result.json` under the same private evidence root.
It created 1600 Resources and fully drained 1500 at 115293 milliseconds.
Both source fences passed. The process drained and left its cgroup empty.

At 1500 completed Resources, metadata publication totaled 328670214 bytes; event payload publication totaled 197250 bytes.
The first completed batch published 1641631 metadata bytes; the last published 42213300 bytes.
Each batch contained 100 Resources and published 13150 event payload bytes.
Terminal totals include the incomplete final batch. They do not describe the same completed workload.
The counters measure selected SQL payloads, not physical WAL traffic or durable commit bytes.
Different captures and unknown observer cost prevent a causal comparison with earlier throughput results.

Two independent reviews confirmed the completed-batch attribution and its limits.
This evidence admits additive Tasks 1–2 in the [keyed journal plan](2026-10-08-keyed-native-journal.md).
It does not admit format 11, adapter conversion, or performance acceptance.

Task 1 added `MetadataHeaderParts`, `MetadataHeader`, and `JournalImage` through the existing metadata support facade.
Its scoped checks passed 156 core unit tests and all-target Clippy with warnings denied.
Raw logs and a final source archive are in `/var/tmp/rom-task1-journal/` on the host.
These constructors reuse canonical structural validation. They do not prove agreement with independent native event rows.
Causal retry checks remain part of Task 2. Independent Task 1 review remains pending at this checkpoint.
Existing adapter serialization and the shared format 10 marker remain unchanged.
Task 2 now implements the coherent preparation and paging contract under a separate source ownership and Cargo lease.
