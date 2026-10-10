# Keyed Native Journal Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce measured journal publication amplification without changing ROM Resource, journal, Work, durability, or recovery semantics.

**Architecture:** First establish numeric metadata/journal attribution at equal completed batches. If that gate supports keying, add a shared scalar header and opaque journal delta, then implement both native layouts. Preserve the canonical archive representation and use it as the semantic oracle and bounded maintenance projection.

**Tech Stack:** Rust, existing serde/serde_json, rusqlite, redb, existing test-support diagnostics, local verifier. No compression, new database, or query planner dependency.

**Spec:** [Native journal amplification](../../research/rom-0.1.0-native-journal-amplification-2026-10-08.md) and [durable Work group-commit constraints](../../research/rom-0.1.0-durable-work-group-commit-2026-10-08.md).

## Global Constraints

- Preserve the original application load profile: 10000 seed Resources, batches of 100, and `deadline_ms: 120000`.
- Keep SQLite `synchronous=FULL` and redb Immediate durability.
- Do not prefetch Claims, defer Materialize, move callbacks into native transactions, or change committed prefixes.
- Preserve Resource/receipt/event/effect/reference/query/Work atomicity and current error ordering.
- The 544087176-byte WAL prefix proves cumulative write traffic, not journal causality or a promised deadline gain.
- Keep historical sources, archives, failed stores, worktrees, and raw evidence.
- Keep `lib.rs` and `mod.rs` as facades; retain existing public paths and canonical types.
- End each admitted RED/GREEN task with source/lock evidence and a root-coordinated commit of only reviewed owned files.
- Root owns shared core, backup markers, shared tests, lockfile, manifests, and final integration.
- SQLite and redb owners edit separate adapter trees; assign ownership before edits.
- Coordinate every Cargo/probe lease. Use offline locked dependencies, two jobs, disabled incremental compilation, and finite timeouts.
- No product source changes or builds are admitted by this planning task. Execution follows the existing autonomous owner authorization.

## Review Focus

1. Unsupported raw edits must not let a cache or unverified aggregate become authority; touched facts and maintenance collections reject corruption.
2. A cursor may advance across other Resource kinds; SQL filtering must preserve that exact progress.
3. Caller logical byte bounds and configured raw maintenance bounds are different contracts; neither may silently replace the other.
4. Historical archive markers are not sufficient fixtures; their native or canonical predecessor shapes must actually match those versions.
5. Unknown acknowledgement can follow a durable commit; replay must recover the same result without another event or Work completion.

## File ownership and module map

| Owner | Existing files | New files and responsibility |
| --- | --- | --- |
| Root: attribution consumer | `tests/application-load/host/src/stage_measurements.rs`, `runner.rs`, `seed_measurements.rs`; diagnostic harness files | Numeric batch attribution module if required; no change to the load profile. |
| Root: shared semantics | `crates/rom/src/storage_support.rs`, `storage_state/metadata.rs`, `storage_state/metadata/bundle.rs`, `storage_state/bundle.rs`, `storage_state.rs` | `storage_state/native_journal.rs` facade; `native_journal/{header,read,delta,prepare,image,page}.rs`; separate test files. |
| SQLite owner | `crates/rom-sqlite/src/{store,persistence,snapshot,migration,retention,upgrade}.rs`, `native_work/{canonical,read,write,schema}.rs` | `native_journal.rs` facade; `native_journal/{schema,read,write,canonical}.rs`; `native_journal_tests.rs`. |
| redb owner | `crates/rom-redb/src/{format,store,commit,storage,native_state,maintenance,migration,retention,upgrade,preflight}.rs`, `native_work.rs` | `native_journal.rs` facade; `native_journal/{read,write,canonical}.rs`; `native_journal_tests.rs`. |
| Root: compatibility | `crates/rom-backup/src/{model,archive,legacy,collector,native_layout_compatibility_tests,scheduling_upgrade_tests}.rs` | Additional archive predecessor tests when existing files no longer provide a focused boundary. |
| Root: shared conformance | `tests/persistence/tests/{journal_head,retention,retry_epochs,backup,schema_migration,delayed_upgrade,transitions}.rs`, `support/{legacy_native,native_canonical}.rs` | `tests/persistence/tests/native_journal.rs`; genuine predecessor fixture helper if needed. |

Paths in later tasks are relative to `/root/ROM`. New type names below are proposed support API names, not existing exported types.

## Task 0: Establish category attribution before approving a layout change

**Files:** SQLite test-support stage modules; root-owned diagnostic consumer and harness; new private evidence only.

**Interfaces:** Extend the existing opt-in diagnostic snapshot with fixed-cardinality numeric fields for serialized metadata bytes and emitted journal payload bytes. Keep stage timing fields unchanged. Root controls the consumer schema; SQLite controls adapter counters.

- [ ] Write counter tests that assert exact byte totals from the already-produced serialization strings, not a second serialization.
- [ ] Test enabled versus disabled outcomes for success, rollback, lost acknowledgement, receipt replay, and Work Claim/Materialize.
- [ ] Test overflow, fixed capacity, poisoned recorder behavior, and the feature-off absence of observer work.
- [ ] Confirm the current serialization occurs inside unchanged publication/error boundaries before counting its length.
- [ ] Implement numeric counters without payloads, IDs, user callbacks, WAL hooks, or checkpoint calls.
- [ ] Record completed batch baseline/final deltas and Work publication size separately; do not infer category from WAL length.
- [ ] Run bounded scoped tests and all-target Clippy under the coordinated Cargo lease.
- [ ] Capture a new diagnostic binary and source identity after current formatting/lock fixes; never claim the older capture matches current sources.
- [ ] Run one finite fresh-store diagnostic with unchanged profile and reviewed output/disk/lifecycle budgets.
- [ ] Compare equal completed batches and distinguish terminal seed evidence from an interrupted prefix.
- [ ] Root records the gate decision: keying is justified by attributed history rewrite, or investigate the actual dominant category instead.

**Gate:** Tasks 1–5 require root's evidence-based keying decision. A large unclassified WAL counter alone does not satisfy this gate.

### Task 0 evidence and admission boundary

The adapter counter gates passed: 55 test-support tests, 36 default tests, and both direct all-target Clippy checks.
The parent consumer passed 18 feature tests and 15 default tests before its fresh diagnostic capture.
The new capture included 393 source inputs and binary SHA-256 `4d6eec7143bc3c711840547bf7ca5ee62d666e10971f7fbb37701ee3d7942854`.

The fresh HOST trial fully drained 1500 Resources at 115293 milliseconds, then failed the unchanged seed deadline.
Independent validation matched fifteen completed-batch publication snapshots to their call snapshots.
After baseline subtraction, metadata publication totaled 328670214 bytes and event payload publication totaled 197250 bytes.
Equal batches of 100 Resources published 1641631 metadata bytes initially and 42213300 bytes in the last completed batch.
Each batch published 13150 event payload bytes.
WorkUpdate header, record, and root totals were 1304516, 2520000, and 141000 bytes, respectively.
No counter dropped samples, saturated, or recovered poisoning.
Both source fences passed; the process drained and left its cgroup empty.

Root accepted the category attribution for the additive canonical oracle prototype in Tasks 1–2 only.
Tasks 3–6 and the format 11 marker remain unadmitted pending prototype review and explicit coordinated implementation admission.
This narrower boundary supersedes any interpretation that the Task 0 gate alone admits a native layout change.
The original load profile and FULL/Immediate durability remain unchanged.

Counters describe selected successful SQL payload publication, not physical WAL traffic or durable commit bytes.
Terminal totals include 1600 Commit operations and an incompletely drained batch; keep them separate from the completed-batch comparison.
The trial performed worse than the earlier 4200-Resource HOST trial.
Different captures, observer cost, and host variation prevent a causal throughput comparison.
No performance target is satisfied by this attribution result.

Independent HOST evidence is `/var/tmp/rom-010-publication-attribution-validation-20261008.json`.
Its companion `.js` records the checks; the [research report](../../research/rom-0.1.0-native-journal-amplification-2026-10-08.md) lists raw proof paths.
The checkboxes above retain the planned task list; this section records executed evidence and the narrower parent admission.

## Task 1: Add a validated scalar header and canonical journal image

**Files:** Root-owned shared semantics files from the map; create `native_journal/header.rs`, `image.rs`, and `header_tests.rs`.

**Interfaces:** Export through `storage_support::metadata`, preserving `StorageMetadata` and `prepare_bundle`:

```rust
pub struct MetadataHeaderParts {
    pub retry_epochs: RetryEpochs,
    pub limits: StorageLimits,
    pub generation: String,
    pub head: u64,
    pub floor: u64,
    pub receipts: usize,
    pub effects: usize,
    pub journal_records: usize,
    pub journal_bytes: usize,
}
pub struct MetadataHeader { /* private validated fields */ }
impl MetadataHeader {
    pub fn from_parts(parts: MetadataHeaderParts) -> Result<Self>;
    pub fn parts(&self) -> MetadataHeaderParts;
    pub fn retry_epochs(&self) -> RetryEpochs;
    pub fn check_retry_epoch(&self, epoch: u64, replay_exists: bool,
        completed_work: Option<(&ClaimKey, u64)>, work: &impl WorkRead) -> Result<()>;
}
pub struct JournalImage { /* canonical ordered events and validated header */ }
impl JournalImage {
    pub fn from_metadata(metadata: StorageMetadata) -> Result<Self>;
    pub fn from_native(header: MetadataHeader, events: Vec<JournalEvent>) -> Result<Self>;
    pub fn header(&self) -> &MetadataHeader;
    pub fn events(&self) -> &[JournalEvent];
    pub fn into_metadata(self) -> StorageMetadata;
}
```

- [ ] Write genuine failing tests for invalid generation, epoch ordering, count/head/floor disagreement, zero/invalid limits, and numeric overflow.
- [ ] Add canonical image tests for duplicate identities, holes, incorrect bytes, malformed events, and retention limits.
- [ ] Implement scalar/image validation using existing canonical validation helpers; avoid a duplicate event-validation algorithm.
- [ ] Preserve exact canonical `StorageMetadata` serde fields and archive body representation.
- [ ] Run `timeout 180 cargo test --offline --locked -p rom --lib` and the affected all-target Clippy gate.
- [ ] Record source/lock identity, RED/GREEN logs, and independently review the public support seam.

**Checkpoint:** Shared format stays 10; existing adapters still compile and use the old canonical route. This is additive support only.

### Task 1 executed evidence and review

The additive header and image constructors are implemented. Existing adapter serialization and format 10 remain unchanged.
The preserved source is `/var/tmp/rom-task1-journal/final-source.tar.gz` on the host.
Independent review verified its checksum, five owned source hashes, and the archived lockfile identity.
The RED log contains four intended validation failures and two successful cases.
The GREEN log contains 156 successful core tests, including all six new journal tests.
The final Clippy log records successful completion. Its invocation is absent; the all-target flags remain implementer-reported evidence.

Review found no blocker in this additive scope.
Header validation reuses canonical generation, limit, and retry-epoch rules.
Image validation preserves canonical ordering and identity rules. It does not establish agreement with independent native rows.
The constructors rely on caller admission before copying an event inventory.
Native adapters must bound cumulative raw values before reconstruction, as Tasks 3–5 require.

Causal `check_retry_epoch` remains part of Task 2, not a completed Task 1 API.
Extreme aggregate rejection was tested. Actual allocation-scale byte overflow was not executed.
The tests do not yet establish exact error variants, boundary-success cases, or malformed Resource-row parity.
Task 2 must cover these equivalence requirements before the shared engine replaces canonical publication.
These results do not admit a native layout change or establish a performance improvement.

## Task 2: Implement one bounded coherent journal preparation and paging engine

**Files:** Root shared `native_journal/{read,delta,prepare,page}.rs`; separate `oracle_tests.rs` and `readset_tests.rs`; existing metadata/bundle and canonical bundle integration.

**Interfaces:**

```rust
pub struct JournalEntry { pub event: JournalEvent, pub encoded_bytes: usize }
pub trait JournalRead {
    fn coherence(&self) -> ReadFence;
    fn header(&self) -> Result<MetadataHeader>;
    fn entry(&self, position: u64) -> Result<Option<JournalEntry>>;
}
pub struct JournalDelta { /* opaque exact read facts, append/retire/header */ }
pub struct NativeBundleDelta { /* opaque JournalDelta and existing WorkDelta */ }
pub fn prepare_native_bundle(
    journal: &impl JournalRead, work: &impl WorkRead, bundle: &Bundle,
) -> Result<NativeBundleDelta>;
pub fn journal_page(reader: &impl JournalRead, kind: &str,
    after: Option<&JournalCursor>, max_rows: usize, max_bytes: usize) -> Result<JournalPage>;
```

The delta accessors are fixed as follows:

```rust
impl JournalDelta {
    pub fn coherence(&self) -> &ReadFence;
    pub fn before_header(&self) -> &MetadataHeader;
    pub fn header(&self) -> &MetadataHeader;
    pub fn appended(&self) -> Option<&JournalEntry>;
    pub fn retired(&self) -> &[JournalEntry];
    pub fn entry_preconditions(&self)
        -> impl Iterator<Item = (u64, &Option<JournalEntry>)>;
}
impl NativeBundleDelta {
    pub fn journal(&self) -> &JournalDelta;
    pub fn work(&self) -> &WorkDelta;
    pub fn into_parts(self) -> (JournalDelta, WorkDelta);
}
```

A JournalRead implementation charges borrowed raw values against its configured physical budget before allocating an entry.
Its per-operation cumulative budget bounds all prefix reads, not each value independently.
The adapter validates all preconditions before the first journal publication and invalidates the shared transaction context after combined publication.
Root and both adapter owners must agree on that publication sequence before implementation.
A journal delta must not remain valid after an earlier Work or journal publication in its reader context.

- [ ] Write oracle tests comparing every resulting canonical field, returned retired identity, Work result, and error with the pre-change canonical implementation.
- [ ] Cover changed-false, receipt replay, stale claims, overload/error precedence, mixed kinds, exact byte thresholds, multiple evictions, and maximum positions.
- [ ] Preserve a test-only independent reference before refactoring canonical publication; do not compare two wrappers around the same implementation alone.
- [ ] Implement shared scalar accounting and bounded prefix eviction. Calculate each incoming event's canonical size once.
- [ ] Validate each touched prefix event's position, identity, actual encoded bytes, and payload before subtracting its size.
- [ ] Make canonical publication delegate to this shared engine through `JournalImage`; remove the duplicate production retention loop.
- [ ] Test paging against the canonical oracle for other-kind gaps, row/byte cutoffs, initial TooLarge, floor/head bounds, and generation/kind mismatch.
- [ ] Test same-context header/read-only fact mutation and cross-context fence rejection; include a Work change that invalidates the journal candidate.
- [ ] Run core lib, all-target Clippy, and downstream adapter compile gates before freezing these APIs.

**Corruption decision:** Open/import/maintenance validate the entire inventory. Ordinary updates validate authoritative scalar headers and all touched facts.
Do not advertise detection of arbitrary untouched physical corruption on every point operation.
If an existing regression requires that behavior, preserve it through bounded validation or stop the integration gate; do not weaken the assertion silently.
No process cache substitutes for those checks.

### Task 2 executed evidence and review

The shared engine and canonical delegates are implemented. The native adapters still use format 10 and their existing serialization.
Final checks passed 167 core tests, all-target core Clippy, and all-target compilation for SQLite, redb, and backup.
The final source fence matched before and after these commands.
Exact commands, exits, compiler, lockfile, and source hashes are in `/var/lib/nixos-containers/rom-dev/var/tmp/rom-task2-journal/`.
The frozen source archive SHA-256 is `fb80302514b5f325fdcb40a7c149c9b7587d0e56d1bf0d3e788f7b64cb722dd0`.
The initial RED lacks contemporaneous source hashes. Later failed compiler checks remain preserved alongside the successful final gates.

Independent review verified the final archive, source fence, commands, and successful results. It found no remaining blocker in this shared scope.
`WorkDelta::validate` now provides the common Work read-fact validation for image application and combined publication.
Oracle tests preserve independent pre-change journal and paging loops, canonical state comparisons, error order, thresholds, and mixed-kind cursor progress.
Read-fact tests cover changed headers, absent append positions, retired entries, context replacement, Work-only invalidation, and cumulative reader bounds.
Native adapters must still enforce physical admission, index/payload agreement, receipt identity arbitration, and joint transaction invalidation.
Canonical bridges reconstruct complete admitted images. These results do not establish an optimized native path or a performance improvement.

Root subsequently executed both explicit accepted-predecessor gates on fresh copies for SQLite and redb.
Native format 8 upgraded to format 10; archive 6 converted to archive 7.
Historical Runtime custom-codec receipts replayed without additional effects; current denial prevented disclosure of those receipts.
All twelve original and copied input hashes remained unchanged after execution.
The result record is `/root/ROM/.superpowers/rom-010-journal-predecessor-TWJKvW/result.json`.
These two gates passed; they do not replace complete conformance, the full verifier, or genuine format-10 fixture admission for future conversion.

### Current shared integration checkpoint

Root ran complete storage conformance and the full `./scripts/check` command after the shared engine refactor. Both ended with exit zero.
The full verifier's before/after inventory matched all 3524 selected working source files and the same tracked deletion.
Generated build/test directories and private `.superpowers` capsules were explicitly outside that source inventory.
The result is `/var/tmp/rom-010-journal-current-full-verifier-result-20261008.json`.
Its log SHA-256 is `a2e270aaf56e916ea70d12f9d06630030549fccbd9b14e29f2a1b57473581ec7`.
This is a complete verifier result for that source, not clean-source release admission or workload acceptance.

A genuine current-format-10 fixture is now preserved at `/root/ROM/.superpowers/rom-010-format10-writer-SX103q/` for each backend.
It includes 35 rows, receipts, and events; a reference; five Work records; operator history; and advanced epochs and delivery profiles.
Its physical admission profile is 8 MiB and 4096 records. Future conversion tests must supply these bounds explicitly.
The writer's execution witness records source, lock, binary, and input fences. Root independently matched 19 output and binary hashes.
Native readers validated private verification copies, preserving the finalized original fixture bytes.
Failed fixture trials remain preserved. Candidate conversion and its negative cases remain unexecuted.

Root next adds one strict shared serialized `MetadataHeaderParts` representation with scoped tests.
Native candidate implementation follows that API freeze.
Candidate tests use isolated private constructors and real databases. Public format-10 open, write, and backup behavior stays unchanged during staging.
Candidate physical format 11 must not publish an archive claiming native format 10.
Bounded canonical snapshots support candidate equivalence tests until coordinated archive-marker admission.

## Task 3: Implement SQLite keyed journal under an unreleased local marker

**Files:** SQLite-owned files from the map, plus new `native_journal` modules and tests.

**Interfaces:** Adapter-private `header(connection, raw_budget)`, `Reader` implementing `JournalRead`, `validate_delta`, `publish`, `import`, and `collect`.
Keep public `Sqlite` and `Storage` paths stable. Retain configured `validation_limits` on the instance.

- [ ] Write an actual SQLite layout RED showing that current format 10 still embeds retained events and lacks the position index.
- [ ] Use a clearly unreleased adapter-local candidate marker 11 for isolated adapter tests; do not change the shared constant yet.
- [ ] Add `journal_positions(position BLOB PRIMARY KEY, identity TEXT UNIQUE NOT NULL, canonical_bytes BLOB NOT NULL)`.
- [ ] Encode u64 positions as fixed eight-byte big-endian values so SQLite byte ordering preserves the complete public range.
- [ ] Encode byte accounting with checked conversion to/from u64; reject invalid key/value lengths and overflow.
- [ ] Store scalar MetadataHeader in `rom_state` and reuse `events(identity,data)` for Row payloads.
- [ ] Implement bounded borrowed raw reads before allocation/decode and same-transaction header/entry/payload reads.
- [ ] Prepare through Task 2; verify all exact facts/fences and publish index, payload, evictions, scalar header, and existing Work/Resource data atomically.
- [ ] Preserve checkpoint injection between writes and existing acknowledgement behavior.
- [ ] Implement scalar epoch/head reads and coherent read-transaction paging through the shared engine.
- [ ] Test real rollback at every new publication boundary, lost ACK/replay, retained history, header/index/payload mismatch, raw oversize before decode, and readonly-fact rejection.
- [ ] Test that ordinary commit/epoch reads do not decode or rewrite retained journal payloads; instrument actual touched entries under test-support.
- [ ] Run SQLite lib and existing SQLite-specific integration/corruption tests with all-target Clippy and formatting.

**Checkpoint:** Old source collection remains explicitly format-10 capable. Ordinary open of a format-10 store is refused by the candidate layout. No relabeling is permitted.

## Task 4: Implement redb equivalent under the same unreleased candidate marker

**Files:** redb-owned files from the map, new `native_journal` modules and tests.

**Interfaces:** Same shared `JournalRead`, `MetadataHeader`, `NativeBundleDelta`, and canonical JournalImage.
Use `TableDefinition<u64, &str>` for the position index value containing identity and checked canonical byte count.
Retain existing `EVENTS` Row payloads and `STATE` keys for metadata/work_header/operator.

- [ ] Write a genuine redb layout RED against current format 10.
- [ ] Add the position table and exact inventory validation under adapter-local candidate 11.
- [ ] Keep scalar metadata and position/payload/Work publication inside one Immediate write transaction.
- [ ] Implement bounded raw value accounting, exact header/read-fact checks, and transaction-context invalidation.
- [ ] Implement coherent read-transaction paging and scalar epochs through the shared support API.
- [ ] Execute the same semantic scenarios as Task 3, including unknown durability outcome, reopened canonical equality, touched corruption, and budget propagation.
- [ ] Keep active Work selection and lifecycle publication unchanged.
- [ ] Run redb lib/all-feature corruption tests, all-target Clippy, and formatting under a separate Cargo lease.

**Checkpoint:** Both adapter owners provide a source freeze and logs before root admits the global marker change.

## Task 5: Preserve bounded maintenance and implement actual format conversion

**Files:** Adapter canonical/snapshot/import/upgrade/migration/retention/operator modules; root-owned shared fixture and backup files.

**Interfaces:** Existing maintenance APIs continue to accept `BackupLimits`. Full reconstruction uses Task 1's `JournalImage::from_native` only after physical admission.

- [ ] Add genuine populated format-10 fixtures with complete old native inventory, metadata events, Work/operator history, epochs, profiles, and query/reference state.
- [ ] Verify these fixtures through current format-10 readers before testing candidate conversion.
- [ ] Convert old format 10 to candidate 11 into a fresh destination through canonical export/import; preserve source bytes and committed WAL.
- [ ] Populate position rows during initialization, restore, migration, schema mapping, retention, and explicit operator maintenance.
- [ ] Test exact canonical equality before/after, destination refusal, interrupted staging, old-open refusal, and source preservation on rejection.
- [ ] Keep existing accepted native 3–9 upgrade scenarios; use their real legacy shapes, not a changed marker on new tables.
- [ ] Bound aggregate raw index/header/payload bytes and records before allocation or JSON decoding.
- [ ] Charge canonical journal payload once in logical backup/page bounds and derived index bytes separately in physical admission.
- [ ] Test configured small maintenance limits against oversized malformed native values before decode, and valid larger custom limits without falling back to defaults.
- [ ] Preserve separate caller Work snapshot limits and configured full-native admission limits. Full reconstruction remains allowed only on explicit bounded maintenance paths.

**Default-limit decision:** No hidden `BackupLimits::default()` is permitted on an instance opened with a custom validation profile. Stateless APIs use their explicit supplied bounds.
The default applies only when the existing public caller deliberately chooses the default constructor.

## Task 6: Coordinate format and archive compatibility after both adapters pass

**Files:** Root backup marker/archive/tests and shared conformance files from the map.

- [ ] Add a genuine RED proving the current writer still identifies native format 10 and rejects the proposed new archive pair.
- [ ] Change shared `STORAGE_FORMAT` to 11 only after Tasks 3–5 pass; remove local candidate constants and align both native markers.
- [ ] Keep canonical archive version 7; accept exact pairs (7,9), (7,10), and (7,11) in the ordinary canonical reader.
- [ ] Preserve original manifest markers when reading historical archives and preserve their bytes.
- [ ] Preserve explicit `upgrade_v6_archive` acceptance of genuine (6,8) predecessor archives; ordinary `read` still refuses that pair.
- [ ] Test actual predecessor scheduling shape without current scheduling fields, populated operator receipts/Work, and the existing legacy rejection rules.
- [ ] Test unsupported cross-pairs, future markers, dishonest inventory, oversized envelopes, and double-normalized or merely relabeled bodies.
- [ ] Update only CURRENT writer assertions; historical 9/10 reader fixtures retain those markers.
- [ ] Run `cargo test --offline --locked -p rom-backup --lib` and focused conformance migration/retention/backup gates before broad integration.

## Task 7: Run full cross-adapter conformance and acceptance

**Files:** Root-owned shared conformance tests and evidence harness; affected adapters only for defects found by their owners.

- [ ] Run every existing native Work corruption/fence/budget test unchanged, then the new journal tests on both adapters.
- [ ] Run shared journal, idempotency, references, queries, operator, restart, retention, schema migration, delayed upgrade, and backup checks.
- [ ] Run the full local `./scripts/check` with a fresh source/lock identity and a finite coordinated verifier lease.
- [ ] Review the complete change for duplicate algorithms, facade-only roots, exact error precedence, coherent snapshots, and unsupported default-limit bypasses.
- [ ] Capture fresh optimized default binaries with all diagnostic features disabled and verify source/binary identities before and after execution.
- [ ] Run the ORIGINAL 10000/120000 seed profile and remaining application acceptance gates on both durable adapters.
- [ ] Preserve a failed result as failed; do not reduce Resources, widen deadlines, shorten retention, or weaken FULL/Immediate to pass.
- [ ] Compare fresh diagnostic byte attribution only at equal completed batches; distinguish improvements from uninstrumented release admission.
- [ ] Root records measured performance, remaining limitations, migration procedure, artifact identity, and the release decision.

## Cost, uncertainty, and handoff

The largest engineering cost is exact shared journal accounting plus real predecessor conversion, not the extra index table.
Coordinating joint Work/journal read fences, maintenance budgets, and cursor equivalence is required before adapter implementation.
Category attribution may show that journal keying addresses only a minority of native commit cost.
Work FULL/Immediate syncs remain even if journal writes become smaller.
No task promises that this optimization alone admits the original workload.

This plan was checked against the report for semantic, compatibility, budget, corruption, and evidence coverage.
All steps are initially unchecked; no implementation or verification result is claimed by this plan.
Root retains the existing autonomous execution and file-ownership coordination model.

## Execution checkpoint: shared gates and native candidates

The shared engine passed 167 core tests and all-target Clippy before native adapter changes.
The complete local verifier then passed with matching hashes for 3524 working source files before and after execution.
Its log SHA-256 is `a2e270aaf56e916ea70d12f9d06630030549fccbd9b14e29f2a1b57473581ec7`.
The result is `/var/tmp/rom-010-journal-current-full-verifier-result-20261008.json`.
This result does not cover subsequent scalar DTO or native adapter changes.

Strict serialization of `MetadataHeaderParts` subsequently passed 171 core tests and all-target Clippy.
The tests reject unknown fields, missing fields, incorrect types, and invalid decoded scalar values.
`MetadataHeader` still requires validated construction; deserialization does not establish authority.
Evidence is in `/var/lib/nixos-containers/rom-dev/var/tmp/rom-task2-header-serde/`.

Actual accepted-release fixtures passed native 8 to 10 and archive 6 to 7 upgrade tests on both adapters.
Runtime tests also verified custom-codec receipt replay without duplicate effects.
Fresh fixture copies and original input hashes remained unchanged.
These gates do not establish native 11 compatibility.

The genuine current-format fixtures are in `.superpowers/rom-010-format10-writer-SX103q/`.
Both fixtures contain 35 Resources, receipts, and events, plus references, descriptors, Work, operator history, and advanced epochs.
Their explicit physical validation profile uses 8 MiB and 4096 records.
Root verified 19 artifact hashes; the writer recorded matching source and dependency identities.

SQLite and redb owners now implement separate native candidates with sequential Cargo leases.
Public constructors and the shared marker remain at format 10.
Candidate archive publication must refuse a false format-10 identity.
Neither native 11 nor the original performance workload has release admission.

Root review found additional candidate read paths that need cumulative physical-budget tests before decoding.
These paths include receipts, Resources, descriptors, relation targets, and relation indexes.
Adapter owners must verify them before source freeze and integration.
The full verifier must run again after final adapter and compatibility changes.
