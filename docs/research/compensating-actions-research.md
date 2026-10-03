# Explicit compensation for reactive Resource chains

Research date: 2026-10-03. Maintained API inspected at `64240cf`.
This document records source-grounded design and the experiment contract. It does
not report the in-progress compensation probe as passed.

## Decision and practical use

The owner selected forward recovery: earlier commits remain, and ROM retries the
failed step under a bounded policy. Compensation is an optional, author-declared
action when the application needs a different business outcome. It changes
Resources through the same authorization, validation, revision and durable
receipt path as any other action. It does not erase history.

Microsoft's compensating-transaction guidance treats recovery as domain-specific
work that can itself fail and need resumption or manual intervention. It warns
against restoring old state over concurrent work; the appropriate result may
differ from the original state. These properties support explicit targeted
actions rather than generic snapshot rollback in ROM.
[Microsoft guidance](https://learn.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction)

Temporal's example highlights a second boundary: an operation may have performed
its effect before a timeout or lost response. Registering recovery only after a
successful reply can miss that effect. Its conditional compensation handles the
possibility that an effect exists or does not. For ROM we infer a stricter rule
for ambiguous external operations: reconcile by stable operation identity before
deciding on a domain transition. A timeout alone does not establish rejection.
[Temporal example](https://temporal.io/blog/compensating-actions-part-of-a-complete-breakfast-with-sagas)

These sources motivate the tests; neither proves ROM's implementation.

| Domain case | Confirmed terminal condition | Targeted action | Information that remains |
| --- | --- | --- | --- |
| Stock reservation | Payment explicitly rejected | Release this reservation ID | Other reservations, original attempt and failure |
| Seat booking | Required downstream booking rejected | Release the held seat | Independent bookings and audit history |
| Account provisioning | Entitlement creation definitively rejected | Disable the account created by this workflow | Existing unrelated accounts and provisioning history |

All payment, booking and entitlement outcomes in this experiment are synthetic.
An irreversible effect, such as a sent message, needs a different forward action
or human handling; there is no universal inverse function.

## What the existing API can demonstrate

[`Reaction::new`](../../crates/rom/src/reactions.rs) maps a committed source
snapshot to target action invocations. An ordinary Workflow Resource can record
`pending`, `transient`, `unknown`, `confirmed_permanent` and `completed` outcomes.
The author can map a confirmed business rejection to an explicit release action.
This uses the existing Resource mechanism without a second domain entity model.

The present API has no generic terminal-failure hook for every technical reaction
or notification failure. A stopped work item is not automatically a new domain
Resource event. The experiment must expose that limitation rather than pretend
its synthetic Workflow outcome was emitted automatically by core.

The same distinction applies to external effects. A durable ROM receipt can
deduplicate a Resource mutation; it cannot alone prove that an external provider
performed an operation once. Provider reconciliation or deduplication remains a
separate adapter contract.

## Experiment and falsification plan

Use actual maintained Runtime, SQLite and redb APIs in an isolated Rust crate.
Compare three application policies on identical scenarios:

1. No compensation: preserve the prior reservation after confirmed failure.
2. Unsafe restoration: use a **valid current revision** but replace data with an
   old snapshot. Demonstrate loss of a concurrent unrelated change. Stale-CAS
   rejection is not evidence that this domain mistake is impossible.
3. Targeted action: release only the identified reservation or disable only the
   identified provisioned account, retaining unrelated concurrent work.

Assert durable Resource states, revision/event counts, receipts and work status,
not merely successful return values. Cover duplicate delivery, transient retry,
unknown-outcome hold, authority revocation, compensation failure and bounded
manual intervention. Reopen stores and exercise a real process-exit failpoint
where possible. Label graceful reopen, process exit and synthetic provider
failure separately; none alone establishes power-loss durability.

Negative controls should make their associated assertions fail when the safety
condition is removed. The final report must publish the commands, pinned source,
raw outcomes, rejected variants and limits. Standalone evidence may justify a
small maintained demo using the same Resource/action/reaction API. It does not
justify adding an unreviewed general saga engine to core.

## Initial recommendation

Keep compensation optional and explicit. Use stable domain operation IDs and
targeted actions. Keep `unknown` visibly distinct from confirmed rejection and
from confirmed success. Persist progress and use normal ROM work supervision;
never make request disconnection own a recovery chain. Evaluate any future
technical-failure hook separately for authorization, causal identity, bounded
retries and its own failure path.
