# Disposable explicit compensation experiment

Question: can an author-declared compensating action recover useful business state
after a confirmed downstream failure without overwriting other work, implying
rollback, or compensating an outcome that is still unknown?

This is an explicitly authorized **native Rust spike**, not a core feature or a
production saga engine. The requested automated tests and scratch databases
intentionally override the prototype skill's default HTML/no-tests guidance.
Runtime, transactions, receipts, journal and durable work use the maintained ROM
crates. Payment, booking-provider and entitlement-provider outcomes are simulated
and labelled. There is no real payment/provider integration.

## Probe plan (written before implementation)

1. Register ordinary Inventory, Seating, Account and Workflow Resources. Forward
   actions reserve a token/seat or provision an account. Workflow outcomes are
   explicit domain facts, not transport-error guesses.
2. Compare three strategies after interleaved unrelated activity: leave partial
   work, deliberately restore a stale whole snapshot, or invoke a declared
   token-specific release/cancel/disable action. Retain audit history.
3. Author reactions from Workflow's confirmed-permanent-failure state to those
   targeted actions. Default/disabled compensation, transient failures and unknown
   outcomes schedule no compensating action. Simulated reconciliation can later
   establish a definite outcome.
4. Exercise stable mutation identities, duplicate workflow events, clean reopen,
   a real SQLite subprocess exit at the post-commit failpoint, current authority,
   retries and bounded terminal/manual cases. Interleave a target edit after work
   materialization to expose revision-conflict handling rather than overwrite it.
5. Exercise a deliberate compensation cycle under the runtime's causal depth/work
   budgets. Inspect durable stopped-work evidence and original committed state.
6. Explicitly demonstrate the architecture gap: a terminal notification failure
   does not call a generic compensation callback. The experiment's Workflow-state
   reaction requires application-authored failure classification and recording.

Primary pass condition: only the intended reservation/booking/account is changed,
committed history remains, and unresolved or unauthorized situations do not launch
an inferred inverse operation. Expected counterexamples (no compensation and stale
restore) are successful experimental observations, not production recommendations.

The one-command verification and machine-readable results are described below.
All databases live in a uniquely named `PROTOTYPE-compensation-wipe-me-*` scratch
directory. Clean reopen will be labelled separately from subprocess termination.

## Run and evidence

With Rust 1.99 and native SQLite build prerequisites installed, run from the
repository root on this prototype branch:

```sh
CARGO_TARGET_DIR=/var/tmp/rom-compensation-target ./prototypes/compensation/verify
```

The command checks formatting, runs Clippy with warnings denied, runs three native
integration tests, and atomically replaces `results.jsonl`. There are **40 cases**:
19 on SQLite, 19 on redb, and two SQLite child-process exits immediately after a
native commit. Each JSON line includes assertions, the action/state trace, and
work records. Counterexamples count as passing only when their expected damage is
observed. Databases are scratch files and are removed after each case.

| Strategy | Stock after A fails, B reserves 2, restock adds 5 | Seat / account result |
|---|---|---|
| No compensation | Total 15; A and B still reserve | A remains booked/enabled |
| Old snapshot at current revision | Total reset to 10; B's reservation erased | Other booking/current account audit content erased |
| Explicit targeted action | Total 15; only B reserves; available 13 | Only A cancelled/disabled; other work and audit content preserved |

The old-snapshot counterexample uses a valid current revision with old business
content. It does not bypass revision checks. ROM's immutable journal still retains
the facts even when the deliberately bad application update erases audit content.
Durable workflow context is created before the corresponding forward action.

## Measured cost under this fixed workload

Storage wrappers count calls, acknowledgments, changed-event acknowledgments,
work claims, retries and inspection reads. Counts cover each whole case including
probe trace/assertion reads and clean reopens; they are not latency benchmarks.
SQLite and redb produce the same stock strategy counts:

| Stock strategy | Bundle calls / acknowledged events | Work claims | Retry claims | Resource snapshot scans |
|---|---:|---:|---:|---:|
| No compensation | 6 / 6 | 8 | 0 | 0 |
| Old snapshot | 7 / 7 | 8 | 0 | 0 |
| Targeted | 7 / 7 | 9 | 0 | 0 |

Thus targeted recovery adds one domain event and one claimed action step to the
no-compensation baseline. The four compiled Workflow reactions account for eight
mapper claims (creation plus failure) even when three mappers return no targets.
A small real application should register only its required mapping; the maintained
demo will have one. Fanout is capped at 4, causal depth at 4, work per root at 64,
and attempts at 3. Each drain is limited to 16 batches of 32 steps. Inspection is
bounded by the durable ledger policy. No Resource snapshot scans are used.
Full counters for non-crash cases are in `cost`. Crash evidence instead asserts
durable journal facts across processes; in-memory counters cannot span killed
processes. These workload counts do not establish a universal performance optimum.

## Boundaries established by the cases

- Transient provider failure retries with a stable identity and does not compensate.
  Unknown acknowledgment holds the reservation until explicit reconciliation.
- Compensation is opt-in and uses the author's explicit confirmed failure fact.
  Each token-specific action is idempotent. Duplicate source delivery and reopen
  preserve identities. Payment/provider effects are simulated; the deduplicating
  receiver does not establish exactly-once external effects.
- Compensation failure retries at most three times, then retains stopped-work
  evidence for an explicit manual action. Revoked service authority blocks it.
  A concurrent target edit after action materialization causes a visible revision
  conflict, preserving the edit; no automatic rebasing is invented.
- A deliberate compensation cycle hits the causal depth limit. Earlier commits
  remain. No implicit transaction rollback or automatic inverse action occurs.
- The existing Reaction API has **no generic terminal-worker-failure hook**.
  A permanently failed notification does not trigger compensation. Workflow-state
  reactions are application composition, not a new core compensation facility.
- SQLite crash evidence uses a real child `process::exit` at the native post-commit
  observer (no Rust destructors). It proves process-restart behavior, not power-loss
  durability. redb cases use clean reopen, not process-kill evidence. The external
  mock receiver deliberately outlives ROM's clean reopen in its retry case.

The experiment follows the concurrency/idempotence caveats in Microsoft's
[compensating transaction pattern](https://learn.microsoft.com/en-us/azure/architecture/patterns/compensating-transaction)
and the cleanup-registration/uncertainty discussion in Temporal's
[compensating actions article](https://temporal.io/blog/compensating-actions-part-of-a-complete-breakfast-with-sagas).
