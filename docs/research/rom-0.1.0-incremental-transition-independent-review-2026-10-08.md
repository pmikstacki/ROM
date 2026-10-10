# Independent incremental transition review

Date: 2026-10-08. Scope: shared engine, in-memory bridge, tests, and supplied raw logs.
No native execution or implementation edit occurred during this review.
This is not approval of native index performance or production persistence integration.

## Current blocker and transaction boundary

The inspected engine removes unchanged record/root entries before constructing WorkDelta.
The bridge checks only entries retained in that write set.
An exact duplicate enqueue can therefore lose its record-read precondition and accept a foreign equal-byte image.
The maintained `unchanged_write_set_still_checks_exact_record_read_preconditions` case targets that condition.
Its corrected source and GREEN evidence were still pending during this inspection.
Do not claim this issue resolved based on the earlier four-test bridge gate.

Keep complete record/root read preconditions separately from filtered writes, including confirmed absence for attempted insertion.
Capture them before filtering. Validate all of them before publishing any bridge change.
Preserve the no-write result for exact duplicate enqueue and repeated DeliveryStarted.

Returned-record preconditions alone do not bind a candidate-search predicate.
For example, consider equal-sized valid records whose due values are 11 and 10, with Claim time 10 and no age expiry.
The first image returns Idle without a candidate; the second has eligible work.
Headers, accounting and root summaries can match, so a foreign Idle delta has no record precondition to reject that difference.
This is a source-derived counterexample, not an executed native regression.

The delta contract already requires application in its original coherent transaction.
Enforce that boundary through transaction-owned application or an opaque snapshot identity/version before claiming general stale/foreign-image rejection.
Alternatively, capture the complete selection predicate dependencies; that can be more expensive than a transaction identity.
The bridge's limited record checks must not be presented as portable optimistic concurrency for arbitrary images.

## Semantic source review

The overlay caches touched complete records and root summaries. PendingWork changes are rejected as IdentityMismatch.
New roots start with zero used attempts and gain a member only when a new record is inserted.
Replacement updates subtract the old attempts/completion contribution before adding the new contribution.
Exact JSON accounting uses the previously reviewed current/reserved contribution helper.

Finalization increments each changed existing record's revision once, then refreshes its byte contribution.
New records retain revision zero. Repeated identical DeliveryStarted produces Changed with no actual record replacement.
DeliveryFinished composes observed delivery and Finish before finalization, so it does not consume two revisions.
Materialize validates a live Source claim, checks frozen parent/child attributes, enqueues children, and marks the parent Done in one overlay.

The five update branches preserve the reviewed current stop-reason precedence, retry multiplication, immutable eligibility floor, and checked time arithmetic.
An absent policy returns Idle before matching a WorkUpdate variant.
The engine validates retry headers before transition and validates touched Action invocation epochs before returning a delta.
Unrelated records rely on complete import validation and maintained native invariants; this is not a fresh full archive check on every update.

Claim visits the reader's relevant candidates in ascending ID order.
Delayed age-expired records become Stopped; unresolved deliveries become AwaitingReconciliation before selection continues.
These changes remain in the same candidate as the final lease. A late overflow prevents publication of the entire prefix.
Resolution-only claims increment generation but do not add attempts or root usage.
The returned WorkClaim is replaced with its finalized record revision.

`candidate` rejects non-increasing returned IDs and the update rejects returned records that are not actually eligible.
It cannot detect a trusted reader that silently skips a lower eligible ID.
Correct native ordered-seek behavior therefore needs adapter conformance tests, not only engine tests.

## Bridge source review

WorkImage import runs complete archive and retry-epoch validation before constructing summaries and active IDs.
Canonical reconstruction is explicitly separate from ordinary transitions.
The bridge uses exact preconditions before mutation, but the current no-op filtering limitation remains above.
After those checks, publication updates records, roots, accounting, and active membership without another fallible Result path.

Its active-ID set contains Pending and Leased records only.
Canonical import with 499 Done records and one Pending record leaves one active entry while retaining all 500 canonical records.
The seek can still inspect every future active record. No constant-time selection or native index bound is established.
The engine fixture's read counters count keyed reads and returned candidates, not every entry examined by its full-map search.
The unrelated-terminal fixture therefore proves no unrelated record output/keyed decoding through that seam; it does not measure every search entry.

## Recorded tests

`/var/tmp/rom-010-work-incremental-edge-characterization.log` reports six passing focused engine tests, zero failures, and zero ignored tests.
They cover enqueue identity, all five updates, delivery profiles/outcomes, composed revisions, epoch errors, policy/capacity precedence, and late-prefix rollback.
They also cover empty policy and keyed read/output behavior with 2000 unrelated terminal records.
The earlier three-test five-update log contains an unused-assignment warning; do not label it warning-free Clippy evidence.

The native `rom-010-work-bridge-publication-red.log` reports one PASS and three intended unimplemented-publication failures.
The following `rom-010-work-bridge-publication-green.log` reports four PASS with zero failures or ignored cases.
These logs reside under `/var/lib/nixos-containers/rom-dev/var/tmp/` on the host.
The later no-op read-precondition case is outside that four-test gate.

The frozen reference comparison supplies independent transition code but shares current model serialization.
It does not yet cover the full bundle, operator, retention, restore, archive, native-fault, or complete workload matrix.
No production adapter invokes this engine yet in the inspected scope.

## Required closure

1. Preserve read preconditions outside filtered writes and verify the maintained no-op foreign-image RED/GREEN.
2. State and enforce original-snapshot application, including candidate predicate and negative-read dependencies.
3. Add native reader tests for lowest relevant ID, candidate completeness, bounded examined entries, and atomic prefix publication.
4. Test actual bundle completion/enqueue/receipt arbitration, operator controls, maintenance, and native commit uncertainty after integration.
5. Capture the final source inventory and run affected checks, full verification, and the unchanged retained-work workload.

This source review does not establish final-source completion or a measured speedup.
