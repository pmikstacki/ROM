# Incremental native layout: independent review

Date: 2026-10-08.

## Scope and result

This review examines an evolving source candidate. It does not approve the release.
The reviewer made no product changes and ran no Cargo commands.
The hashes below identify an inspection snapshot, not a compiled input closure.
Concurrent owners can change these files after this review.

Both adapters use the original coherent writer transaction for Resource bundles and Work deltas.
No separate native commit publishes Work before its Resource, receipt or event.
The shared engine retains domain policy; adapters perform keyed reads and native writes.
Three acceptance gaps remain at this snapshot.

## Acceptance gaps

1. **Shared format marker remains 9.** `crates/rom-backup/src/model.rs:7` exports `STORAGE_FORMAT = 9`.
   SQLite `native_work/schema.rs:4` and redb `format.rs:10` use native format 10.
   Manifest construction uses the shared constant. New native-10 exports therefore still carry the old manifest marker.
   The accepted format-B plan requires marker 10, while readers retain explicit archive-7/native-9 compatibility.
   Coordinate this change with both adapters. Repeat archive, upgrade, restore and predecessor gates after the change.
   This mismatch does not imply a changed canonical archive body.

2. **redb has no fault seam inside new Work writes.** `native_work.rs:83-104` writes records, active entries, roots and header.
   `storage.rs:52-69` commits standalone Work without observer checkpoints before or after commit.
   Bundle checkpoints surround other writes, but cannot interrupt each new Work write.
   The source uses a coherent transaction; this is a required failure-test gap, not observed partial publication.
   Add checkpoints for the complete new write sequence. Test rollback and committed lost acknowledgement for standalone and bundled Work.
   Preserve the actual commit-error uncertainty latch. A simulated lost acknowledgement must not claim a failed commit.

3. **SQLite caller limits follow full operator reconstruction.** `operator.rs:15` reconstructs with default maintenance limits before caller limits.
   `persistence.rs:43` passes default `BackupLimits`, independent of operator `max_records` and `max_bytes`.
   Default bounds prevent an unlimited read. A small caller budget cannot reject raw Work before decoding.
   The owner has staged this correction. This review does not treat it as executed or verified.
   Use malformed oversized native Work as a negative control. Require `TooLarge` before decode.

## Source findings

### Atomic publication and authority

SQLite begins an Immediate transaction before receipt arbitration and Resource revision checks.
It prepares the shared bundle, then applies Resource, references, query indexes, events, receipts, effects, Work and metadata.
The transaction commits once. Errors before commit leave the candidate unpublished.
Some changed Work serialization occurs during writes; a later error still propagates through transaction rollback.

redb holds its writer gate and checks availability before and after native writer acquisition.
It selects Immediate durability. Receipt arbitration precedes Resource revision checks.
Work tables come from this same writer transaction. Changed Work payloads are serialized before the first Work write.
Resource, receipt, event, effect, reference and Work publication remain in one transaction.
An actual redb commit error sets the uncertainty latch before another writer can enter the gate.

A lost acknowledgement is `Unknown`, not `NotCommitted`.
SQLite's maintained test models committed lost acknowledgement and checks exact receipt replay after reopen.
Standalone Claim recovery must inspect durable claim state. Blind replay can consume a different pending record.
No new cache or independent Work receipt authority appears in these changes.

### Exact read facts and context

Both native apply paths check the opaque `ReadFence`, header and complete record/root preconditions before writes.
The iterators retain facts even when a write is filtered out. `None` comparisons protect observed absence.
Both adapters rotate the fence after apply, including an unchanged delta.
A delta cannot cross reader contexts even when they use the same transaction.

SQLite's new readonly regression changes a PendingWork definition after preparation.
It expects `Conflict` for an unchanged delta with a retained record fact.
That test is later than the inspected 39-test checkpoint. It is source evidence here.
redb's added foreign-context test is also later than the inspected three-test gate.
The fence does not provide lock-free MVCC. The originating coherent transaction remains mandatory for ordering and phantom absence.

### Active projection and ordering

Both active tables contain Pending and Leased records only. Retained Done history stays outside candidate scans.
Selection seeks in ascending Work ID order, excluding IDs at or before the prior candidate.
Pending eligibility uses due time or elapsed maximum age. Leased eligibility uses lease expiry.
Missing records and terminal records in the active projection fail closed.
Canonical reconstruction recomputes the projection through `WorkImage::from_native` and checks exact equality.

This index improves the workload with retained Done history. It is not a constant-time eligibility index.
A large set of future Pending or unexpired Leased records can still require an active-table scan.
The shared engine remains responsible for the atomic ID-ordered prefix and preceding stop effects.
The inspected gates do not establish final 10,000-row workload latency or mixed-load performance.

### Bounds and canonical reconstruction

SQLite canonical reads charge borrowed SQL values before owned copies and decoding.
redb canonical reconstruction charges native state, Work, roots and active entries before decoding or owned-key allocation.
`WorkImage::from_native` checks exact accounting, roots and active projection against canonical Work.
Archive collectors charge complete canonical StorageState plus derived native root/header/operator/index representations.
Raw native charging additionally catches whitespace that canonical normalization would remove.

Ordinary keyed reads use startup-validated tables. They do not add a per-read caller byte budget.
Direct native edits during ownership are outside this assumption and appear only in controlled corruption tests.
Full maintenance reads remain bounded, but can scan and allocate the retained graph.
Operator control still reconstructs and rewrites this graph. Do not describe it as an incremental performance result.

### Formats and preserved predecessors

Current open accepts native 10 and refuses native 9 without implicit conversion.
SQLite retains the old native inventory for format 9. redb retains the old single-state layout and nine-table inventory.
Both explicit upgrade readers accept format 9 and publish through the fresh-destination restore path.
Archive input accepts exact archive-7/native-9 and archive-7/native-10 pairs for the same canonical body.
There is no fallback to a weaker legacy decoder when current validation fails.

The redb source test synthesizes native 9 and checks source bytes before and after refusal/upgrade.
It is not an execution of an accepted old writer with custom codecs and populated receipts.
Public upgrade comments still mention formats 3 through 8. Update them when the final range is accepted.
Repeat authentic Runtime/predecessor and whole-state archive proofs against the final native-10 candidate.
Preserve old artifacts and evidence. Synthetic conversion tests cannot replace those proofs.

## Executed evidence inspected

The SQLite file-test log ends with 39 passed, zero failed, in 1.21 seconds.
Its Clippy log ends with successful compilation in 6.11 seconds.
The redb second-integration log contains three passed tests in 0.22 seconds.
It also contains an unused-variable warning from that earlier source checkpoint.
The separate redb first-Clippy log ends successfully in 0.36 seconds.
These logs are distinct checkpoints. They do not prove the latest combined source.

The three redb tests cover fresh layout, format-six upgrade and retained-Done candidate exclusion.
Added native-nine, corruption, foreign-context and operator-budget tests are outside that inspected checkpoint.
No speedup, packaged final consumer, full verifier or release admission is established here.

## Required closure

- Finish the three acceptance gaps and retain their negative controls.
- Exercise each new native write boundary and committed lost acknowledgement on both actual adapters.
- Compare Work updates and composed bundles with the preserved canonical oracle.
- Include readonly roots, absence facts, no-op fence rotation and ID-prefix overflow/error rollback.
- Verify raw and combined canonical/derived limits on startup, backup and operator paths.
- Repeat authentic native-nine conversion, old-writer refusal, custom-codec receipt replay and restored claim fencing.
- Rerun the unchanged retained-work and mixed workloads against final frozen source.
- Run downstream package tests and the full local verifier before integration.

## Inspection snapshot and raw evidence

The following hashes are captured after source inspection. They do not establish a concurrent source freeze.
2026-10-08T13:24:41Z
```text
d188435805b36f47344a4c06accbdb279713993a199a24aeced61740b964aec7  crates/rom-redb/src/native_work.rs
2633d9dc16a2337db3aeb371e7f383d6b9d4c20e1dd3bf17a98fe186bd9384bc  crates/rom-redb/src/native_state.rs
764588393629aa60188d764888b84efac611423ecdb960ee7ed1cc1c65f8ca8c  crates/rom-redb/src/commit.rs
0aa9b3aecb0fb8f81308d874248632a065ef0bbfc95e2142b203ae5b84ba76ba  crates/rom-redb/src/storage.rs
e5b2347af672727a6da21f517359a41903b9e9b1ef1a2e72c4df1d72c606823c  crates/rom-redb/src/maintenance.rs
f037203aa5b0257109cc9aa797efa21d2214f9ff036dc61e8913341f592738e6  crates/rom-redb/src/format.rs
0741cbf6be4e9910f9b3c73ae8359c1ab6bbd95f75eb87e8b10b8926ac9c27ba  crates/rom-redb/src/operator.rs
22b55cba3da3a660c946c03614fb2eef011565fbfe9a5f96b7ab6953f8f475a0  crates/rom-redb/src/native_work_tests.rs
d6c5d436f964ffddb4debfba8485f1e719178ee0f18ea4fe908167b52a3bd38b  crates/rom-sqlite/src/native_work/read.rs
761d9704908d62715e1bee4102649deb63e26afaa0763d22dfef11dbbaae38b1  crates/rom-sqlite/src/native_work/write.rs
d940974fa5dc7ebb364689b5d3300eb0745ac0af70c5bf57f92f6d07d1e60a26  crates/rom-sqlite/src/native_work/canonical.rs
2cf70526b3f291c5610cc4f570bb58a868786e5c2dd410438f794d5d2d01ac34  crates/rom-sqlite/src/native_work/schema.rs
14c5b949c3712d17e3650fd2cc15165490633d0604df8a688e30f1f0959b0ba7  crates/rom-sqlite/src/persistence.rs
b884d9c257e913db119e9acb104fb0ef3a92cfa2ec3192ef407cbd5390c40a76  crates/rom-sqlite/src/snapshot.rs
4e9e2ebcbc4bb8b64b395b597c4bb2ec07e657f23a67c08087a819b21b65b179  crates/rom-sqlite/src/store.rs
f3a3209a5507b1baa5652b8c0d1b0b451d644c028050d36ecd0fc02756296447  crates/rom-sqlite/src/operator.rs
ae0d200a7b92d4e8e29b7707266501b6a9a9267407f4aa9eeeea4c501ac17cc2  crates/rom-sqlite/src/native_work_tests.rs
8314145074e496423eadf094c81c8c62b864cd3e5c85c8954878de3fd452f167  crates/rom-backup/src/model.rs
4aa19a310dc58018fd9c8d07cdfb3455ef5b0386106a1dcf293bc01c033c1f54  crates/rom-backup/src/archive.rs
f06268ad9d49fa90ab8295c362fc968e11b5a6051d5e1c5a740d7ba9513c498e  /var/tmp/rom-010-sqlite-native-file-tests-after-lock-20261008.log
a48fb5a63f97d6694337c2fa7cb22949ea3f9475f9f4044bcdb55253b8bbcfce  /var/tmp/rom-010-sqlite-native-clippy-20261008.log
cff5e8bce4a3ecd47b6080b3980ae1b50297c8dc30d1b80cfa9c86a72d50bad8  /var/tmp/rom-010-redb-native-second-integration.log
ff29f4b77eb78b1029ee0f031ea554e3e85a93b59d29ba91811dd09ddebbbdb1  /var/tmp/rom-010-redb-native-first-clippy.log
```

## Corrective review: owner-final local gates

The original findings above remain as historical evidence. This section supersedes their current status.
The reviewer inspected the corrected source and raw logs without native execution.

All three original gaps are addressed in source and scoped gates:

- `rom-backup/src/model.rs` now exports `STORAGE_FORMAT = 10`.
  The genuine negative control expected `(7, 10)` but observed `(7, 9)` before the correction.
  The subsequent backup library gate has 33 passed tests.
  Original archive-7/native-9 read compatibility remains explicit; archive-version 7 does not change.
- SQLite `operator_snapshot` now calls `admit_operator` before canonical reconstruction.
  Admission charges borrowed logical Work and operator values before decoding.
  Native keys, root summaries, accounting and indexes use the independent configured native budget.
  The final exact snapshot check still counts operator receipts and complete serialized snapshot bytes.
  Both operator and bulk canonical reads retain `self.validation_limits`.
  The negative control includes oversized invalid raw values, not merely an oversized valid result.
- redb `NativeWork::apply` now calls an observer after each record, active entry, root and header write.
  Standalone Work update also exposes before-commit and committed-acknowledgement checkpoints.
  Bundle publication forwards these checkpoints through its existing ordinal sequence.
  An actual native commit error still sets the uncertainty latch.
  A simulated post-commit acknowledgement failure returns `Unknown`; it does not set an invented failed-commit state.

Both adapters now reload the persisted header during apply.
This closes a same-context header-change hole in redb's prior cached header comparison.
redb's same-sized record and root corruption controls require exact `Conflict` rather than a byte-count match.
The originating native writer transaction and opaque context fence remain required.

### Inspected gate scope

SQLite's source-gate file contains 43 source/input hashes and five raw-log hashes.
The reviewer recomputed them: zero mismatches in both sets.
Its raw library gate has 44 passed tests in 0.81 seconds.
Its raw query-index gate has four passed tests in 0.44 seconds.
Its Clippy gate includes SQLite, redb and storage conformance, with successful completion in 5.23 seconds.
The prior operator-budget negative log has one passed and two failed intended controls.

The redb raw all-feature library gate has 13 passed tests in 0.45 seconds.
It covers native-nine conversion, corruption, foreign context, persisted header, exact facts, configured budget and Work fault boundaries.
The raw redb Clippy gate completes successfully in 0.69 seconds.
These tests are actual local execution evidence. They do not establish final shared conformance or the full verifier.

The shared marker migration log contains delayed-delivery, retention and schema gates.
The schema gate has six passed tests and one intentional subprocess fixture ignored.
No current 10,000-row load result, final package closure or release admission follows from these local gates.

### Remaining cost and scope

Raw admission introduces an operator-only scan before bounded canonical reconstruction.
This preserves logical caller limits without charging duplicate native representations against the caller's logical budget.
Full operator controls and archive reconstruction still scan retained history.
The optimized ordinary Work path retains keyed lookup and changed-record publication.
The active index excludes terminal history, but still scans future Pending and unexpired Leased entries.
No constant-time eligibility or measured final latency claim is made.

One bulk-read asymmetry remains in inspected source.
redb `storage.rs:40-50` directly decodes and collects the WORK table in `reaction_records`.
It does not apply `self.validation_limits`, unlike SQLite's corrected bulk canonical read.
Startup validation bounds the existing store, but this API lacks the configured-budget check used during later reconstruction.
Add a bounded bulk-read control if the configured budget applies to every later full Work read.
Use an oversized invalid raw record to distinguish predecode rejection from decode failure.
This is a source concern, not an executed failure or a mutation-path defect.

The review found no new transaction-cohesion blocker in the corrective local source.
Final acceptance still requires actual bundle conformance, final-source load, authentic predecessor evidence and the full release gates.
2026-10-08T13:34:59Z
```text
2bcf4e6de2bf0647b54c0b0b77fc92d2290de7e594d0746c5e02e4ea30aa7f20  crates/rom-backup/src/model.rs
385451194942ae5e2b165194694ad6f4eb3b57474e66c47c35d7fdbcf034d4b6  crates/rom-sqlite/src/operator.rs
7ff493996f387175fbb36e10fefb6289904a58a394088795279d57bda6baa617  crates/rom-sqlite/src/persistence.rs
81dd9ef13fd08c6b380b96e1a1ce59d9b4c5cb72dbecad4e4e4737318b79a6aa  crates/rom-sqlite/src/native_work/operator_admission.rs
761d9704908d62715e1bee4102649deb63e26afaa0763d22dfef11dbbaae38b1  crates/rom-sqlite/src/native_work/write.rs
c6c6fcf000872debcad09d7c3f554270444f6b905d26a116824f48fad1128d4d  crates/rom-redb/src/native_work.rs
a6a1fcbc4a4515d61035686bf792446d211bbc2a8f31322f81e18ee296525571  crates/rom-redb/src/storage.rs
b22496aee70d003649474ff2a3e08f0ea0a22641ac6a80cb70f6a295949ac8bc  crates/rom-redb/src/operator.rs
d7a334a9fe878d8b59e9d1f3a91d9b54d4e0a39d77cd6520a02bfc675b27da5b  crates/rom-redb/src/commit.rs
57727baebc3feca8daa2fc87821417e2d4395e5d414ade1676173ea359600642  crates/rom-redb/src/native_state.rs
4c24b741b82678b0b859a42ebcdfb922a528532b2c68f42ca6912ffb52359f3a  crates/rom-redb/src/native_work/publication_tests.rs
76ed0e68cc9fd46f2518601dea3737ffad9b59ccb934e0a807e9c1cdee97b181  /var/tmp/rom-010-sqlite-native-budget-source-gate-20261008.json
2aeb75308b95ca63431443785c5679385c1825c4fe707996052761e9f174e013  /var/tmp/rom-010-sqlite-native-budget-green-20261008.log
6db72099eaaae9064e95c2e635870b1efcc8a9e92ed2aca384d6cdc8a99b6576  /var/tmp/rom-010-sqlite-query-fixture-green-20261008.log
22217c888ac6f6560d331b1dbd0c26219fdccf6e752b4183a92cd6c2cf0ab2c2  /var/tmp/rom-010-sqlite-native-budget-clippy-20261008.log
84e2851316c880d0f032df113766804eb6a243404705b4f7ee04a02f7a47be67  /var/lib/nixos-containers/rom-dev/var/tmp/rom-010-redb-native-fault-matrix-green.log
a47b1c402f754f109013bba823d204e721efd6e3ee4be32599cba904b9fe3e47  /var/lib/nixos-containers/rom-dev/var/tmp/rom-010-redb-native-fault-final-clippy.log
9e47b9a4d11995cb828c55d6a81458f718e78e4191c4ff54024617d12197221b  /var/lib/nixos-containers/rom-dev/var/tmp/rom-010-native-writer-marker-red-20261008.log
c056b18f28652d9c30eff9d7c4662721d724ed83e235f21f2464d0f94ea8dc03  /var/lib/nixos-containers/rom-dev/var/tmp/rom-010-native-writer-marker-green-20261008.log
```
