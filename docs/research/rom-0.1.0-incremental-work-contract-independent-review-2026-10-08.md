# Independent incremental work contract review

Date: 2026-10-08. Scope: proposed contract, retained-work plan, and current shared model source.
No native execution or product edit occurred. Actual storage attribution probes remain separate evidence.

## Result and required correction

The proposed transaction-local reader, affected-record overlay, and shared delta are a suitable minimum implementation seam.
Keep policy and exact accounting in core. Native adapters must only read coherent records and atomically persist validated changes.
Implementing work tables alone cannot remove scans in the surrounding StorageState, bundle, or operator paths.

One proposal sentence contradicts the current archive contract.
It says empty retained root metadata is logically permissible.
`reaction_work/ledger.rs:43` builds root usage only from retained members; `ledger.rs:47` requires that map to equal all stored roots.
A root entry without any retained member therefore fails archive validation, including an entry whose used count is zero.
A zero-used root WITH retained zero-attempt members is valid and must remain distinct.
Correct the sentence before freezing the import and native-summary contract. Do not loosen archive validation during optimization.

## Smallest implementable seam

Use one named core support module with a coherent reader and private transition overlay.
The reader supplies the header, exact record, root summary, and next relevant ID. Each native reader belongs to one transaction.
The result contains complete affected records, affected root summaries, exact header accounting, index changes, and the existing WorkResult.
Keep records and frozen PendingWork intact. Do not expose serialized deltas as executable authority.

The first shared implementation must handle enqueue plus all five WorkUpdate variants.
Resource bundle completion and reaction enqueue must compose through the same overlay before native publication.
Operator control must share the same transition and accounting helpers, although its receipt arbitration remains a distinct operation.
Do not create separate native retry or delivery-policy implementations.

Retain a test-only frozen copy of the old transition implementation as the differential oracle.
Comparing two wrappers of the new engine would not independently test semantic preservation.
Do not replace the old public WorkLedger paths until their bridge passes canonical differential checks.

## Claim selection and atomic prefix

Current Claim visits BTreeMap records in ascending ID order.
It can stop several delayed, age-expired records and move several uncertain deliveries to reconciliation before returning one lease.
Those preceding changes belong to the same atomic transition as the returned claim.
If a later checked generation, attempt, root count, lease time, revision, or size operation fails, none of that prefix publishes.

The candidate seek must return the smallest relevant ID after the previous ID, within the same coherent transaction.
Relevant means due Pending, age-expired delayed Pending, or expired Leased work.
Done, Stopped, and AwaitingReconciliation records do not participate.
A due-ordered queue changes existing fairness. A fixed prefix-write cap changes existing atomic behavior.
Neither is a transparent performance correction.

Use bounded seek pages for native reads, without truncating the semantic prefix.
One claim can legitimately affect many records, up to frozen max_records.
Measure native entries examined as well as records decoded and writes applied.
A simple time/ID index can still scan many due candidates to find the lowest ID.
Do not promise constant-time selection before the native query proves its examined-entry bound.

At equal due/age time, preserve eligible claim logic rather than the future-due age-stop branch.
An overflowing started_at-plus-age calculation means no representable age expiry; saturation would create an incorrect event.
Preserve generation changes on resolution-only claims, while attempts and root usage remain unchanged for those claims.
Return the finalized claim record after its one checked revision increment.

## Exact accounting and shared invariants

Maintain two independent totals: canonical current JSON bytes and canonical lifecycle-reserved JSON bytes.
The current reserved transformation uses maximum numeric widths, its specific Leased shape, and Retryable delivery.
Implement that transformation once. Do not substitute a different theoretical maximum representation.
Its current behavior is the compatibility oracle.

Map entries include escaped keys, a colon, encoded values, and comma accounting.
Envelope and serialized limits also contribute. Empty/nonempty map changes need exact accounting.
Replacement deltas require checked subtraction and addition; saturation cannot preserve bounds or expose corruption.
Terminal records retain lifecycle reservation and their consumed root attempt budget.

New records start at revision zero. Changed existing records receive one revision increment per finalized transition.
DeliveryFinished invokes Finish internally. Materialize enqueues children before completing its parent.
An overlay must not increment a revision separately for each internal step.
Exact duplicate enqueue is a no-op; changed frozen PendingWork is IdentityMismatch and rolls back the entire candidate.

Validate immutable invocation epochs at admission and import. Maintain root epoch coherence under each local write.
Global retry-floor advancement remains a full maintenance operation with dependency validation.
Derived summaries and indexes must match canonical records at import, restore, and archive boundaries.
They are durable invariants, not unchecked hints.

## Publication and uncertainty

Native publication includes work/root/header/index changes and existing Resource, receipt, event, effect, and operator receipt changes where applicable.
Preserve the existing transaction and durability settings. An observer delta is not callback permission.
DeliveryStarted must commit before external delivery. Unknown acknowledgement remains Unknown.
Do not blindly retry Claim after lost acknowledgement; it can consume another attempt or select another record.
Recover through durable state and exact generation where no existing receipt supplies a replay oracle.

Archive reconstruction remains a coherent full scan and uses unchanged canonical serialization.
Native format conversion needs explicit refusal and fresh destinations. Preserve predecessor archives and old Runtime/custom-codec evidence.
The archive format and native storage format are separate decisions.
The plan correctly requires current receipt authority, restore fencing, retention closure, and application rollback proof.

## Implementation tasks after attribution

1. Correct the empty-root statement and freeze reader/delta invariants with separate file ownership.
2. Preserve the old model in a test-only oracle. Add deterministic differential sequences before replacing transition internals.
3. Implement exact JSON contribution and reservation helpers. Test escaped keys, optional fields, all outcomes, limits, and overflow rollback.
4. Implement the coherent overlay and all five updates, enqueue, bundle composition, and operator transitions.
5. Add in-memory bridge checks. Compare complete canonical state, Result, returned claim revision, and exact totals with the old oracle.
6. Add SQLite and redb native records and ID-preserving candidate seeks under separate ownership.
7. Test failures between every affected write, before commit, and after committed acknowledgement loss; reopen and compare canonical state.
8. Implement explicit conversion, summary reconstruction, archive/restore/retention validation, and actual predecessor replay checks.
9. Repeat the unchanged 10000 retained-work profile and finite mixed load. Rerun diagnostics and package/full verification after source changes.

A successful local benchmark cannot replace these semantic checks.
The attribution probe establishes boundary costs; internal codec and native-touch claims require additional measurement.
No measured speedup, final-source acceptance, or release readiness follows from this source review.

## Proposal correction

The proposal author corrected the root invariant during review.
The current note preserves zero-used roots with retained members and rejects orphan root metadata.
It adds explicit positive and negative controls for this distinction.
The corrected note's SHA-256 is `204e3aa94935863e81279cfa18510bdc403a5516f341d986d284a99aca92f7cc`.
The identified proposal contradiction is resolved. The implementation and measurement tasks remain open.
