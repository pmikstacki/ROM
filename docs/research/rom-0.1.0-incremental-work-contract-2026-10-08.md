# Incremental retained-work contract investigation

Date: 2026-10-08. Status: source-only proposal; no implementation, native test, Cargo command or measured speedup in this investigation.

Use a shared transition engine over transaction-local record reads, producing affected work/root records and exact accounting changes. Keep `Storage::reaction_update(WorkUpdate) -> Result<WorkResult>`, `WorkLedger`, `StorageState` and operator public paths stable. Native adapters apply the resulting changes inside their existing transaction. Reconstruct the canonical `StorageState` at archive, restore and maintenance boundaries.

This agrees with the [retained-work plan](../superpowers/plans/2026-10-08-retained-work-latency.md). Product selection remains conditional on the identity lane's storage attribution. The [independent latency review](rom-0.1.0-persistence-latency-strategy-independent-review-2026-10-08.md) records both optimized 10000-record seeds failing the same 120-second boundary. Its timings establish failure, not serialization's causal share. Source establishes amplification; it does not establish the correction's speedup.

## Inspected paths and observed contracts

| Source | Finding |
| --- | --- |
| [ledger.rs](../../crates/rom/src/reaction_work/ledger.rs:74) | Enqueue clones the ledger, checks full lifecycle capacity, scans root epochs and serializes the ledger. |
| [ledger.rs](../../crates/rom/src/reaction_work/ledger.rs:157) | Apply clones the ledger; candidate publication scans existing records and increments each changed record's revision exactly once. |
| [work.rs](../../crates/rom/src/storage_state/work.rs:7) | Update clones the surrounding StorageState and scans retained retry epochs before publishing. |
| [storage_state.rs](../../crates/rom/src/storage_state.rs:152) | Resource bundles clone metadata and atomically complete a claim, enqueue work, count receipts/effects and update bounded journal history. |
| [SQLite persistence](../../crates/rom-sqlite/src/persistence.rs:40) | State read decodes complete `rom_state`; save encodes and replaces it. Reaction update and Resource commit both use this path. |
| [redb state](../../crates/rom-redb/src/state.rs:5), [storage](../../crates/rom-redb/src/storage.rs:47), [commit](../../crates/rom-redb/src/commit.rs:8) | Complete `state` JSON is decoded and rewritten inside the native write transaction with Immediate durability. |
| [control](../../crates/rom/src/reaction_work/control.rs), [operator state](../../crates/rom/src/storage_state/operator.rs:57) | Control mutates a record and generation, then creates an exact operator receipt atomically. Receipt arbitration precedes expected revision checks. |
| [retention](../../crates/rom/src/reaction_work/retention.rs), [storage retention](../../crates/rom/src/storage_state/retention.rs:46) | A causal root has one retry epoch; only entirely completed roots below replay floor may retire. Floors, native dependency counts and journal retirement are coordinated. |
| [worker](../../crates/rom/src/reactions/worker.rs:41), [claims](../../crates/rom/src/reactions/claims.rs) | Existing worker invokes shared Claim, then current-authority processing. Worker admission and batch bounds already exist; this proposal adds no scheduler. |
| [operator execution](../../crates/rom/src/operator/control_execution.rs:84) | Current operator authority and saved receipt scope are checked before replay; verifier runs without guards, then authority and exact version are checked again. |
| [backup model](../../crates/rom-backup/src/model.rs:44) | Snapshot includes canonical StorageState, native rows/receipts/events/effects/references. Current archive version is 7, native format 9. Full archive validation remains necessary. |

There is no local mutation benefit if only work storage is split while `StorageState::bundle`, `update_work`, retry-epoch checks or operator control still reconstruct the complete ledger. All ordinary callers must use the same shared incremental engine. Full public exports such as `reaction_records()` may remain intentionally proportional to retained history; they must not become an internal claim prerequisite.

## Proposed implementation-support seam

Add a named `reaction_work/incremental.rs` module. The following names are proposed, not existing API. Keep these adapter-support types opaque where practical; do not add a second public scheduling protocol or accept a serialized delta as authority.

```rust
trait WorkRead {
    fn header(&self) -> Result<WorkHeader>;
    fn record(&self, id: &str) -> Result<Option<WorkRecord>>;
    fn root(&self, id: &str) -> Result<Option<RootAccount>>;
    fn next_candidate(&self, now: u64, after_id: Option<&str>)
        -> Result<Option<WorkRecord>>;
}
fn prepare_update(read: &impl WorkRead, update: WorkUpdate) -> Result<WorkDelta>;
fn prepare_enqueue(read: &impl WorkRead, limits: &ReactionLimits,
                   work: Vec<PendingWork>) -> Result<WorkDelta>;
```

`WorkHeader` contains frozen optional ReactionLimits, current RetryEpochs, exact record/root counts and exact logical current/reserved byte totals. RetryEpochs remains owned by storage metadata; this header is a coherent transaction read, not a new independent epoch authority. `RootAccount` contains the existing used-attempt count, immutable root epoch, member count and incomplete-member count. A completed member means the exact existing predicate: Done; Notification additionally Accepted; other payloads delivery None. Derived accounting fields are native format metadata, not additions to canonical root JSON.

`WorkDelta` contains ordered work replacements/insertions, affected root replacements/insertions, updated accounting, index removals/insertions, transaction read preconditions and the existing WorkResult. An existing work precondition includes exact ID, revision and claim generation/state; an insertion includes confirmed absence. Existing records keep their entire frozen PendingWork unchanged. Each replacement carries a complete affected WorkRecord, avoiding fragile adapter-level field patches. Root preconditions and header/epoch preconditions prevent lost counters. All changed existing records receive checked revision +1 exactly once; unchanged records do not. New records retain revision 0. DeliveryFinished's nested Finish and Materialize's enqueue are composed in one overlay and versioned once at finalization.

The read implementation refers to one coherent native transaction. The core keeps a small overlay for touched records/roots, so repeated reads during one transition observe earlier candidate changes. It alone derives policy, stop reasons, accounting and index keys. Adapters provide key lookup, candidate selection and atomic writes; they do not implement separate retry, epoch, budget or delivery policies. Any necessary cross-crate public exposure should be adapter-support API in a named module with redacted Debug and no Deserialize, while legacy root exports remain stable.

An in-memory WorkLedger bridge implements the same reader and applies the delta to touched map entries. It preserves rollback on error without cloning unrelated history. Keep a test-only frozen reference implementation of the current model for differential tests; two wrappers calling the same new engine are not independent oracles.

## Five update variants

| Update | Required reads and exact effects |
| --- | --- |
| DeliveryStarted | Load exact claim. Require now at/after immutable eligibility floor, unexpired lease, matching generation, Notification and no resolution-only lease. Set delivery Unknown. Commit before callback; duplicate identical state is not an invented revision change. |
| DeliveryFinished | Validate exact live Notification claim; store observed outcome, then derive Finish from that candidate. ReconcileBeforeRetry + Unknown/TimedOut/Panicked moves to AwaitingReconciliation with generation +1. Otherwise Accepted -> Done, Permanent -> DeliveryPermanent, Panicked -> CallbackPanicked, Unknown/Retryable/TimedOut -> ordinary retry. One revision increment for the composite update. |
| Claim | Visit relevant records in ascending work-ID order, preserving all preceding expiry/reconciliation changes; stop after the first lease admission. Pending future-due but age-expired -> Stopped(Age), then continue. Eligible unresolved delivery -> AwaitingReconciliation, generation +1, then continue. A claim always increments generation, including resolution-only. Only a claim without a stop reason increments record attempts and root used. Lease until uses checked now + lease_seconds. Returned claim contains the finalized revision. Idle/Changed/Claimed distinction remains exact. |
| Materialize | Validate live Source claim without resolution-only marker. Enforce max_fanout and exact parent retry epoch/root/depth/started_at/service/definition/version; children must be Action. Enqueue all children and mark parent Done in one candidate and native commit. Exact duplicates remain no-ops; changed identity fails the entire transition. |
| Finish | Validate live claim. Done/Stop preserve attempts/root use. Retry with reconciliation-required knowledge enters AwaitingReconciliation. Otherwise preserve existing precedence: attempts/work budget before age at completion; claim admission uses depth then age then attempts/work budget. Retry delay is checked retry_seconds * (1 << min(attempts - 1 saturating, 10)); checked now + delay; due is max with immutable eligibility floor. |

With limits None, existing update returns Idle before matching a variant; do not silently change this edge behavior. All numeric overflows produce the existing error before publication. Root usage equals the sum of retained member attempts, not count of leased or unfinished records. Terminal records keep their budget charge and lifecycle reservation.

## Admission, epochs and exact byte reservation

Enqueue uses keyed existing-record lookup and a candidate overlay for duplicates within the input batch. Existing ID + exact PendingWork is a no-op; changed PendingWork is IdentityMismatch. Frozen limits must match exactly. Non-Notification work requires AtLeastOnce. A new record begins Pending with attempts/generation/revision 0, due equal to max(started_at, not_before), delivery None. Root epoch must match all existing members; root used begins 0 only for a genuinely new root.

Validate Action invocation retry_epoch against Cause at admission/import; immutable payloads do not need reparsing on every unrelated mutation. New work may not be future-epoch or below replay floor unless the existing completed predicate permits it. Resource bundle reaction epochs must equal its receipt epoch. Original receipt replay is arbitrated before a live completed-work claim check; stale claim must not block exact committed receipt replay. New causal execution still requires exact live claim and the existing causal epoch rule. Global floor advancement remains maintenance-only with complete dependency validation.

Preserve BOTH current serialized-ledger bytes and lifecycle reserved bytes against max_bytes, plus max_records. The current reserve transformation sets each root use to u32::MAX, numeric work fields to their maxima, state to the maximal reserved Leased representation and delivery Retryable; it retains frozen payloads. Implement that transformation and exact JSON contribution calculation once in core. Sum escaped serialized map key + colon + serialized affected value, commas for nonempty maps, and fixed envelope/limits bytes. Separate current and reserved totals. Account root-key insertion/removal and map comma transitions; update replacement differences with checked subtraction/addition. Do not estimate using struct size, SQL text length, a compressed encoding or payload-only bytes. Differential tests must compare totals to whole canonical serialization for escaped/Unicode keys, optional not_before, every state/outcome and boundary-sized payloads. Changing canonical serde representation invalidates the accounting codec and requires review/version handling.

At import/open integrity validation, recompute totals and root summaries from canonical records; reject disagreement. Persist summaries in the same transaction as record changes. They are maintained durable invariants, not an unchecked cache. Ordinary changes use exact local arithmetic; maintenance may perform a full rebuild. Saturation would conceal corruption or lost limits and is forbidden.

## Selection, expiry and next wakeup

Current Claim walks a BTreeMap by ID, not by due time. The index must return the lowest relevant ID at or below the query time, then the next higher relevant ID. Relevant Pending means due <= now OR age expired while due > now; relevant Leased means until <= now. Done, Stopped and AwaitingReconciliation are absent. The core rechecks state and immutable policy before mutation.

Derive each Pending event time as min(due, finite age expiry); Leased event time is until. Overflow computing started_at + max_age_seconds means no representable age event, not an early event or saturation. At equal due/age boundary, the core follows ordinary eligible-claim logic rather than the future-due age-stop branch. not_before remains included in due and never grants extra lifetime. Derive event keys from core policy; adapters maintain native indexes atomically.

A plain due-ordered queue changes fairness and preceding effects. A plain ID-ordered scan still scans retained pending work. An index query returning all due records can allocate unbounded intermediate output. Require an ordered seek interface with bounded pages and prove examined native entries, not merely decoded records. A simple (time, ID) index does not by itself guarantee efficient minimum eligible ID. Select a native query/index strategy only after measured seek costs; a shared ordered index with subtree minimum event-time summaries is one possible implementation, not a required new scheduler. No claim of constant-time selection is made here.

One existing Claim can change many preceding relevant records before returning one claim or Changed. Preserve that observable atomic prefix; do not silently cap it into several transactions. Its worst case remains bounded by frozen max_records and genuinely affected records, rather than all unrelated terminal history. A tighter changed-record cap would require a separately reviewed scheduling contract. No hidden policy change is needed for the local-update correction.

A read-only next-wakeup query returns the minimum event time for active records, with current policy/header coherence. It includes early age expiry of delayed Pending work and lease expiry. It does not reactivate held uncertainty. The current worker polls every 100 ms; optional wake optimization can use this value through the existing worker, but is separate from persistence correction.

## Operator, retention and archive boundaries

Prepare operator control through the same affected-record engine plus operator receipt lookup/write. Resolve WorkHandle using a durable handle-to-ID index; preserve ambiguous-handle detection rather than treating a hash as unique authority. Receipt identity remains principal + request identity, with exact request comparison and IdentityMismatch before expected-version CAS. Current Runtime authorization, saved scope on replay, guard-free verifier and post-verifier authorization/CAS remain unchanged. Operator change, generation bump, work revision, counters/indexes and receipt commit together. Existing Leased/Done control refusal and immutable lifecycle budgets remain. A stale request cannot consume a generation; exact receipt replay does not rewrite work or create another receipt.

The current Runtime operator path requests a complete bounded raw snapshot before projection/control. Native local control alone does not remove that observation cost. Keep existing snapshot API, but propose a separate bounded targeted trusted read (handle candidates + exact principal/request receipt + generation/epochs/scope) for control only if attribution identifies it as relevant. Preserve full envelope limits and current authority; do not weaken projection limits through an uncharged partial snapshot. Public operator listing remains an explicitly bounded observation, not a work scheduling input.

Maintain a root-to-member index and epoch-to-root index. Retention selects roots below replay floor, refuses any incomplete member, then removes entire selected roots and associated records/index/accounting in the same maintenance transaction as receipt/effect dependency closure, journal floors and epoch floors. A zero-used root is valid only with retained zero-attempt members. Root metadata without any retained member is invalid: validate_archive derives used entries only from retained records and requires exact equality with roots. Import must preserve valid zero-used roots and reject orphan roots. No automatic retention or discarded history is part of this optimization.

Archive export assembles canonical WorkLedger (limits, ID-ordered records, existing root-used map) and unchanged StorageState fields from one coherent native snapshot. Derived native summaries/indexes are not extra canonical work records. Run full existing validate_archive and Snapshot.validate, plus new native-to-canonical consistency checks. Restore keeps existing storage-generation fencing and per-record lease recovery: reconciliation-required knowledge held; other leases generation +1, Pending at eligibility floor; changed record revisions +1. Rebuild native summaries/indexes from that validated candidate.

A native layout change needs a new explicit format number and existing fail-closed preflight. Do not casually change archive version 7: determine whether unchanged canonical serialization warrants retaining it while storage-format manifest changes. Convert copied supported predecessors into fresh destinations; old artifacts remain byte-identical. Prove actual prior Runtime/custom codec receipt replay and explicit old-binary refusal separately from application rollback. Full scans during conversion/archive/restore/retention are acceptable and bounded; ordinary local update must not call those paths.

## Atomic commit and uncertainty

SQLite applies metadata header, affected work/root/index rows, Resource/reference/query-index changes, receipt/event/effect and operator receipt writes inside the existing Immediate transaction. redb applies the same logical set inside its existing writer gate and Immediate-durability transaction. The delta is not published outside that transaction and does not authorize callbacks. DeliveryStarted acknowledgement precedes callback; a commit-unknown cannot be manufactured into NotCommitted or an automatic new external attempt.

Failure before native commit discards every delta change. Commit failure/lost acknowledgement retains Unknown; redb's existing uncertain-writer fence remains. Invalidate optional caches and reread authoritative records/receipts after recovery. Do not blindly reapply a Claim after lost acknowledgement: it may spend another root attempt or reclaim another record. Stable Resource/operator receipt replay remains the recovery oracle where a receipt exists; work transitions without such a receipt recover through exact durable generation/state, not a new parallel receipt store. Any proposed local retry requires proof that the original transaction did not commit.

## Concrete acceptance additions to the maintained plan

1. Differential sequences covering all five updates, enqueue and operator control: compare full canonical state, errors, WorkResult and returned claim revision. Include two changed records preceding a claim and atomic prefix rollback on late overflow.
2. Current/reserved byte totals equal full serialization across key escaping and map empty/nonempty transitions; exact max_records/max_bytes boundaries; changing unrelated terminal history does not increase ordinary local reads/encodes/writes.
3. Root used/epoch/member/completed invariants after duplicates, Materialize, resolution-only claims, Retry, uncertainty and whole-root retention. Include valid zero-used roots with retained zero-attempt members and invalid orphan-root metadata. Future/old Action epochs and invocation mismatch reject without state change.
4. Native candidate ordering controls: lexicographic ID versus due order; delayed age expiry; equal age/due; not_before floor; expired leases; unresolved delivery; time overflow; many unrelated terminal and future Pending records. Measure inspected entries and affected writes, not just successful claims.
5. Both actual adapters: faults between each work/root/header/index/operator/native bundle write, before commit and committed lost ACK; reopen exact canonical archive equality; concurrent stale claims and exact receipt arbitration under current authority.
6. Restore generation and work revisions, native format refusal, copied predecessor conversion, real prior-reader/custom-codec replay, archive bounds and corrupt-summary/index detection. Preserve original archives/WAL controls.
7. Rerun unchanged 10000 retained-work seed and mixed load only after source attribution and interface review. Record native touches/encoded bytes and sync time separately. Run conformance, consumers and full verifier before integration.

The plan already covers these categories. Its phrase “bounded eligibility index” needs the explicit ID-order/prefix semantics and measured native-entry bound above; it must not imply a new per-Claim truncation. “Compute lifecycle reservation accounting incrementally” needs exact canonical envelope accounting and two totals. “Same logical delta” must include operator receipts, retry-epoch/root summaries, bundle completed-work and uncertainty behavior, not just standalone reaction_update. No checklist item or release requirement is closed by this note.

## Source identity

SHA-256 of the inspected contract sources and crosschecked plan at note creation:

```text
562f75c95d7929466de2dc039d77cc28e86b065b1035f6197a7f5f921ee7afe8  crates/rom/src/reaction_work/ledger.rs
244cc184d80a186cb849f45d4e40c06f5358ebb999114513c649341510a52e94  crates/rom/src/reaction_work/model.rs
f4079b3fc83189f63d08d9cfa960e5215a021a50562ae69117c49799a8ec8284  crates/rom/src/reaction_work/control.rs
86acfea1fd4d892a226d05e5581a1a99f903b0def9c3dc897bea8a95586a7cb3  crates/rom/src/reaction_work/retention.rs
0e7003c36e7d7d00cfe721c65789a191ea7189c7551843d87003dd35ccf7ea90  crates/rom/src/reaction_work/delivery_profile.rs
270a464e1a2c54cc975f89f92489e03dd3d3bfee08840efb764755cd146d900b  crates/rom/src/storage_state.rs
08c48a0b13a337e91a1b04bf57cf6cb6557a14e61e29e43142d56b6ef953aede  crates/rom/src/storage_state/work.rs
eab8f26f7236aa0762f15a3d8f71fb8e695e9184a98e5d362de90611cd643f78  crates/rom/src/storage_state/operator.rs
33dd703560d48d154a4b121e8b08da5851217088a37b8a734abe256873bf98a9  crates/rom/src/storage_state/retention.rs
ab7c3c496ec88f9c7629e5bb6f014f5f32944cc34709b8521c24605beb7eb89a  crates/rom/src/reactions/worker.rs
3511494b7141fdcb0ce357d5b601887f1cdadf80d28f4cb1b6d513e0f76a70a6  crates/rom/src/operator/control_execution.rs
3c76aff56c536bb7c5c08dc35bc4eba38cc7f90228be43d21c6a75b2a78970db  crates/rom-sqlite/src/persistence.rs
f3a3209a5507b1baa5652b8c0d1b0b451d644c028050d36ecd0fc02756296447  crates/rom-sqlite/src/operator.rs
ecbf73e2f328d3725ffe963e5a6a6cc925402c97bf83565a4471e60e3866ac37  crates/rom-redb/src/state.rs
285186b6e2ab56f1e87b333a441132776b1835f5271891a59a710826a89378a0  crates/rom-redb/src/storage.rs
864fbef370ccf55e5cbcdc40525afc59924701f82d9ce26859141f985b94b3df  crates/rom-redb/src/commit.rs
98df99c857e1a838d4dcc82581365a57fe068ae0cc30efd89573569e9105317b  crates/rom-redb/src/operator.rs
8314145074e496423eadf094c81c8c62b864cd3e5c85c8954878de3fd452f167  crates/rom-backup/src/model.rs
670f9ab7ca063c73024d296880be2c57dd501005817cb9650ab301ba22d37752  docs/superpowers/plans/2026-10-08-retained-work-latency.md
```
