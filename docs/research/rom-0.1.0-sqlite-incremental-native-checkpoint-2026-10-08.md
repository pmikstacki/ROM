# SQLite incremental native checkpoint

Date: 2026-10-08. Status: implemented adapter subset; final integration remains open.

This report uses the project writing rules and the ASD-STE100 skill as guidance. It does not certify standard compliance.

## Implemented boundary

SQLite now stores Work records by ID. Separate tables contain root summaries, active IDs, the Work header, and operator metadata.
The `rom_state` singleton contains bounded Resource metadata and journal state. It excludes retained Work and operator histories.

Ordinary Resource commits and Work updates use the shared opaque delta contract.
The adapter validates its originating reader fence, header, and exact record/root read preconditions before Work writes.
Resource rows, references, query projections, receipts, effects, events, Work records, roots, active IDs, and headers use the same IMMEDIATE transaction.
SQLite retains WAL mode and FULL synchronous mode. No history is discarded to reduce ordinary write cost.

The adapter reads active IDs in lexicographic order. It counts each actual examined row, including ineligible active records.
Done records remain in the authoritative Work table. They are absent from the active-ID table.
Worst-case selection cost still depends on active records. These tests do not establish a constant-time scheduler.

Full reconstruction remains explicit for open validation, archives, restore, and operator maintenance.
Before decoding, the native reader applies cumulative raw-byte and record limits.
It compares the persisted header, root summaries, and active IDs with the shared canonical import result.
The archive collector counts canonical Work payloads once. It additionally charges native header, root, active-ID, and operator projections.
These guards are conservative. They must not normalize oversized whitespace away before the raw-byte guard.

## Executed checks

| Check | Result | Host evidence |
| --- | --- | --- |
| First layout test | One genuine failure: native marker 9 instead of 10. | `/var/tmp/rom-010-sqlite-layout-red-behavior-20261008.log` |
| First native subset | 35 library tests passed, including six new native tests. | `/var/tmp/rom-010-sqlite-native-first-green-attempt-20261008.log` |
| File and recovery subset | 39 library tests passed, including ten new native tests. | `/var/tmp/rom-010-sqlite-native-file-tests-after-lock-20261008.log` |
| Clippy | All targets with `test-support` passed with warnings denied. | `/var/tmp/rom-010-sqlite-native-clippy-20261008.log` |

The native commands used offline, locked dependencies, two build jobs, no incremental compilation, and a 120-second outer limit.
Original command logs remain in the container's `/var/tmp`. The host copies retain the same filenames.

The tests cover these concrete cases:

- Two hundred retained Done records remain available; a claim examines one active row.
- A delta from another native reader is rejected before writes.
- Corrupt root summaries, accounting, active IDs, identities, singleton rows, and object inventory are rejected.
- Oversized native whitespace fails the raw-byte limit.
- Each of four Claim writes can fail without publishing partial state.
- A post-commit Claim acknowledgement failure retains the claim and attempt count.
- Each final Resource Work/header/metadata write can fail; reopening shows no partial bundle.
- A lost Resource acknowledgement recovers the same receipt after reopening.
- A format-9 source upgrades into a fresh destination without changing source bytes.
- Canonical backup/restore retains Done history and fences an old claim.

`/var/tmp/rom-010-sqlite-native-source-gate-20261008.json` contains after-command source and log hashes.
It is not a retroactive before/after source fence. A subsequent read-only-precondition regression is staged but not covered by the 39-test result.

## Limits and remaining checks

The adapter-local native marker is 10. At this checkpoint, the shared archive/storage constant remains 9.
This is an unreleased integration state. Final format coordination and compatibility gates remain open.

No current full verifier, independently reviewed adapter acceptance, clean-source artifact, or release tag is established by this subset.
The large retained-work workload has not been repeated with this source.
These checks demonstrate behavior in the listed real-store fixtures; they do not establish a production throughput result.

## Subsequent marker and budget gate

The root agent admitted shared native marker 10 after its archive compatibility checks.
SQLite now uses the shared constant. The initial marker-9 checkpoint above remains historical evidence.

Three additional operator tests exposed two real errors before their fixes:

- Oversized invalid raw Work reached JSON decoding and returned `Storage`, instead of the caller-budget error `TooLarge`.
- Full reads used the default native budget instead of the profile supplied when SQLite opened.

The adapter now retains the configured native validation budget.
Open, operator controls, operator snapshots, and bulk Work reads use that profile for full reconstruction.
Separate operator admission counts Work records and raw logical payload values before decoding.
Native ID keys, journal data, accounting, and root/index projections do not consume the caller's logical snapshot budget.
Shared core validation still checks the exact complete snapshot size and Work-plus-operator-receipt count.

The configured-budget regression uses a 4 KiB profile and an invalid 8 KiB payload.
It proves profile propagation and rejection before decoding. It does not prove acceptance of a store larger than the default 128 MiB profile.
An exact-byte snapshot test passes with a large journal present; one fewer byte returns `TooLarge`.

| Subsequent check | Result | Host evidence |
| --- | --- | --- |
| Operator budget regressions | One passed and two genuinely failed before production fixes. | `/var/tmp/rom-010-sqlite-operator-budgets-red-20261008.log` |
| Complete SQLite library subset | 44 tests passed, including 15 new native tests. | `/var/tmp/rom-010-sqlite-native-budget-green-20261008.log` |
| Historical SQLite fixture | The original direct-header read failed with missing `work`. | `/var/tmp/rom-010-sqlite-query-legacy-fixture-red-20261008.log` |
| SQLite query integration | All four tests passed after public canonical fixture export replaced the direct-header read. | `/var/tmp/rom-010-sqlite-query-fixture-green-20261008.log` |
| Clippy | SQLite and storage conformance, all targets, passed with warnings denied. | `/var/tmp/rom-010-sqlite-native-budget-clippy-20261008.log` |

The new library gate also executes the read-only-precondition regression staged earlier.
A separate test modifies the persisted header in the same reader context. Applying its prior delta fails before Work writes.

`/var/tmp/rom-010-sqlite-native-budget-source-gate-20261008.json` contains subsequent after-command hashes and log identities.
Large-workload performance, broad shared conformance, independent adapter review, and final release gates remain open at this checkpoint.

## Opt-in stage observations after the native layout gate

The native layout did not pass the complete seeded workload deadline.
The parent requested stage observations before another storage design change.
This diagnostic implementation does not establish a throughput improvement or release acceptance.

`Sqlite::observe_stages()` enables observations when `test-support` is enabled.
Its handle returns 18 fixed measurements through `snapshot()`.
The three operation labels are `commit`, `work_update`, and `retry_epochs`.
Each operation has six stage labels: `connection_lock`, `transaction_begin`, `metadata_read`, `shared_prepare`, `native_publication`, and `native_commit`.
Unused stages retain zero samples.

The measurements contain elapsed nanoseconds and completed sample counts.
A completed guard records its duration, including an early error or unwinding.
The measurements do not classify operation success.
Obtain the final snapshot after operations stop.
The recorder contains no Resource identifiers, payloads, user callbacks, or per-request history.

These are wall-clock intervals, not exclusive CPU measurements.
`metadata_read` includes SQLite access and metadata decoding.
Commit preparation includes authoritative reads, decoding, and shared validation.
Work preparation includes the keyed Work reader and shared preparation.
Native publication includes transactional writes and existing failure checkpoints.
Native commit measures `tx.commit()`; the acknowledgement checkpoint remains outside that interval.
Recorder synchronization occurs after the elapsed-time reading.

Without `test-support`, the timer imports, guards, observer field, and public observer API are absent.
With that feature enabled, an instance without an observation handle does not read a clock or update counters.
The aggregate has fixed capacity.
Overflow drops the complete sample and increments a saturating dropped-sample counter.
A poisoned recorder mutex recovers its data and sets `poison_recovered`.
It does not return a storage error.

The same-feature control test compares real database outcomes with observations disabled and enabled.
It covers Resource commit, Work claim and materialization, rollback, lost acknowledgement, replay, and retry epochs.
Separate tests cover a missing Work preparation result, duration/count overflow, disabled timing, and poisoned-recorder recovery during a real commit.
Storage decisions, transaction boundaries, error ordering, and durability settings remain unchanged.

| Gate | Executed result | HOST raw evidence |
| --- | --- | --- |
| Initial behavior regression | Failed because the expected stage samples were zero. | `/var/tmp/rom-010-sqlite-stage-observation-red-20261009.log` |
| Wired behavior test | One test passed; 44 were filtered. | `/var/tmp/rom-010-sqlite-stage-observation-wired-green-20261009.log` |
| Complete test-support library | 49 tests passed. | `/var/tmp/rom-010-sqlite-stage-observation-all-lib-final-20261009.log` |
| Test-support all-target Clippy | Passed with warnings denied. | `/var/tmp/rom-010-sqlite-stage-observation-clippy-20261009.log` |
| Default library | 36 tests passed. | `/var/tmp/rom-010-sqlite-stage-observation-default-lib-20261009.log` |
| Default all-target Clippy | Passed with warnings denied. | `/var/tmp/rom-010-sqlite-stage-observation-default-clippy-20261009.log` |
| SQLite source formatting | Passed. | `/var/tmp/rom-010-sqlite-stage-observation-fmt-20261009.log` |

The original logs are in CONTAINER `rom-dev:/var/tmp/` with the same filenames.
Cargo used offline locked dependencies, two jobs, disabled incremental compilation, and a 180-second timeout per command.
`/var/tmp/rom-010-sqlite-stage-observation-source-gate-20261009.json` records log hashes and after-command source identity.
It is not a source capture taken before the commands.
The earlier failed scripting attempt and incorrect test expectation remain in separate raw logs.
The final test uses the existing authoritative `Missing` result; no product behavior changed to satisfy that assertion.

The parent owns the diagnostic application consumer and matched workload run.
Those measurements and the broad release verifier remain separate gates.
