# Compensation: executed domain cases and recovery evidence

Date: 2026-10-03. Source: immutable prototype commit
`a3e06caa23c155cba1dd7df8591236251b835c3f`, branch
`codex/prototype-compensation`, directory `prototypes/compensation`.
The parent coordinator independently reran its verifier in native `rom-dev`:
formatting, Clippy with warnings denied and three integration tests passed,
covering **40 cases**. The verifier also generated 40 machine-readable result
records. Payments, booking and entitlement providers are synthetic; ROM Runtime,
SQLite/redb transactions, receipts, events and work ledgers are real.

## Result and recommendation

Explicit token-specific compensation preserves concurrent unrelated work.
Restoring an old payload at the **current valid revision** can erase that work;
revision checks cannot recognize an application's incorrect business intent.
Leaving prior commits untouched is correct as a default, but does not by itself
release a resource held by a failed business workflow.

| Case | No compensation | Old snapshot replacement | Targeted compensation |
| --- | --- | --- | --- |
| Stock: A reserves, B reserves, stock is replenished, A fails | A and B remain reserved | B and replenishment are lost | Only A is released |
| Seat: A fails after another booking | A remains booked | Other booking can be lost | Only A's seat is released |
| Account: entitlement rejected after unrelated account changes | Account stays enabled | Current unrelated content can be lost | Specific account is disabled; unrelated content remains |

The immutable event journal remains in every strategy, including the deliberately
bad update. Compensation is a new domain action/event, not deletion of history or
a cross-Resource transaction rollback.

## Coverage and costs

Nineteen cases run on each adapter, plus two real SQLite child-process exits.
The suite covers strategy comparison, transient retry, unknown-outcome hold,
compensation retry and terminal/manual state, revoked service authority, target
revision conflict after work materialization, duplicates, clean reopen, lost
external acknowledgements with a synthetic deduplicating receiver, and bounded
cycles. Workflow context is committed before the forward operation.

Under the fixed stock workload, both adapters have the same counts:

| Strategy | Bundle calls / acknowledged events | Work claims | Retry claims | Resource snapshot scans |
| --- | ---: | ---: | ---: | ---: |
| No compensation | 6 / 6 | 8 | 0 | 0 |
| Old snapshot | 7 / 7 | 8 | 0 | 0 |
| Targeted action | 7 / 7 | 9 | 0 | 0 |

The targeted action adds one event and one claimed action step to the baseline.
Counts include fixture inspection; the fixture registers four Workflow mappers,
which account for eight mapper claims on creation/failure. The maintained demo
will register only its required mapper. These are operation counts, not latency,
allocation or production throughput measurements. They do not establish a
universal optimum.

Fanout is bounded at 4, causal depth at 4, work per root at 64 and attempts at 3.
Draining is bounded to 16 batches of 32 steps. A concurrent target edit yields a
visible revision conflict; the test does not invent automatic rebasing.

## Evidence boundaries

- SQLite subprocess tests exit immediately after the failure commit and after
  the compensating commit, then inspect recovery from another process. This
  avoids Rust destructors but does not simulate power loss.
- redb tests reopen cleanly; no redb process-kill claim is made.
- Unknown provider outcome preserves the reservation until a synthetic explicit
  reconciliation result exists. No real payment service was contacted.
- The current Reaction API does not turn every terminal technical worker failure
  into a domain Resource event. A stopped notification does not automatically
  call compensation. The Workflow Resource explicitly records a confirmed
  business outcome and its author-declared reaction chooses the next action.
- Counterexample cases pass only when they expose the expected damage. They are
  not recommended application behavior.

## Reproduction and promotion

Check out the prototype commit separately; from that checkout in `rom-dev`:

```sh
CARGO_TARGET_DIR=/var/tmp/rom-compensation-target ./prototypes/compensation/verify
```

The command writes `prototypes/compensation/results.jsonl`; scratch databases are
removed after each case. Its README explains the full case matrix and trace
format. The parent run log was `/var/tmp/rom-compensation-independent.log` in
the container.

Promote only a small example composed from maintained Resources/actions/reactions
into the demo, with both-adapter tests. A general saga engine or automatic
terminal-failure hook would require a separate contract and is not established by
this result. The [research rationale](compensating-actions-research.md) explains
the source-grounded concurrency and uncertainty assumptions.
