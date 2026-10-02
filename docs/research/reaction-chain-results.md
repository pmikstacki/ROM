# After-commit reaction chains: executable policy comparison

Date: 2026-10-02. Disposable deterministic Rust simulation on `codex/prototype-reaction-chains`. This follows the owner's decision: reactions run **after the upstream commit**, a failed downstream action retries under bounded policy, and the upstream transition stays committed. There is no implicit chain rollback. Resource remains the only domain entity; causal identities, receipts and delivery records are runtime/protocol values.

## Recommendation

ROM needs layered protections: durable delivery/action identity; semantic no-op suppression for actions whose contract permits it; conservative dependency/change filtering; separate hop and total-work budgets; bounded ready-work and byte admission; durable retry/quarantine records; and explicit ordering/recovery semantics. None of the single protections tested is sufficient. Reject blanket visited-resource suppression: a legitimate converging chain can revisit the same resource.

Use coalescing for explicitly latest-state projections, with authoritative freshness and revision checks. Preserve every required domain fact. Treat a monotone finite fixed-point engine as a separate pure-derivation capability, not as the default execution model for arbitrary resource actions or external effects. These are recommendations from executed counterexamples and primary-source research, not selected library dependencies or production guarantees.

## Reproduction and evidence boundaries

```sh
nixos-container run rom-dev -- bash -lc 'cd /workspace/ROM/.worktrees/reaction-chains && ./prototypes/reaction-chains/verify.sh'
```

Observed: **19 tests passed, zero failed; formatting and Clippy passed; regenerated CSV matched the retained file exactly**. Rust 1.99.0, two compiler jobs, isolated target directory. The [verifier](../../prototypes/reaction-chains/verify.sh), [lockfile](../../prototypes/reaction-chains/Cargo.lock), [implementation/assertions](../../prototypes/reaction-chains/src/lib.rs), [CSV generator](../../prototypes/reaction-chains/src/main.rs) and [110 result rows](../../prototypes/reaction-chains/results.csv) are retained. The crate has no external dependencies. An initial Clippy precedence warning was corrected with explicit parentheses before the complete verifier passed.

This is **not a database benchmark or durability test**. There is no SQLite, filesystem persistence, real crash, network, Tokio/Rayon workload or competing process in this simulation. A cloned store models committed resources/events/receipts, retry state and root budgets surviving restart; volatile ready work is returned for redelivery. Thus the crash-window assertion establishes what follows **if** that atomic durable contract holds. Existing actual restart evidence remains separate in [prior prototype results](prototype-results.md); it is not re-earned by this model.

The engine models committed events routed through declared rules into the same simulated mutation path. A root input creates the first committed event. Every subsequent mutation creates a new event identity. Each reaction delivery identity is `(event ID, rule index)` within this fixed test graph. A production identity must additionally cover stable declaration/version, scope and child-action ordinal rather than mutable array position.

## Counting model

`action_attempts` includes the root and each target evaluation after scheduler receipt deduplication, including failures/no-ops; receipt replays have their own `duplicates` count. `resource_writes` and `events` count actual modeled transitions, including equal-state writes in intentionally naive policies. `modeled_db_roundtrips` is receipt lookups + target resource loads + transaction charges; a state/event/receipt commit is one transaction charge. A durable no-op receipt costs a receipt-only transaction. Metadata, queue, checkpoint, authorization, network and storage-engine calls are excluded.

`redundant_modeled_roundtrips` counts charges on equal-state or replayed work. They are potential waste indicators, not a claim that every correctness read can be removed. For example, authoritative no-op detection and durable idempotency lookup can be necessary even when no resource write follows. Comparisons do not establish real I/O counts, latency, CPU cost, allocation cost or a universal speedup.

`ready_peak` bounds scheduled handles. `backlog_peak` records retained modeled durable obligations, which still allocate ordinary memory in this program. Bounding ready work alone does not bound journal/backlog storage. All scenarios have an experimental watchdog of 128 delivery steps; `watchdog=true` means pending work remained, not success or a mathematical proof of nontermination. The echo/oscillation rules themselves make their continued behavior clear.

## Executed comparisons

Ten policies: naive queued callbacks; stable-ID dedup alone; no-op suppression alone; change filters alone; hop budget eight alone; root work budget twelve alone; visited-resource suppression; ready queue capacity four alone; dedup/no-op/filters/queue without causal budgets; and the layered policy. Layered stress settings are sixteen hops, sixty-four child attempts per root, four ready handles and three attempts per failed delivery. These numbers are test values, not recommended production defaults. All policies use the same three-attempt failure rule to isolate loop protections.

Counts below include the initial upstream action:

| Scenario / policy | Attempts | Writes | Modeled DB calls | Observed outcome |
| --- | ---: | ---: | ---: | --- |
| A→B→C / naive | 3 | 3 | 6 | Correct acyclic result. |
| A→B→C / layered | 3 | 3 | 8 | Same result; dedup adds modeled lookups. |
| Equal-value A→B→A echo / naive | 129 | 129 | 258 | Watchdog; 254 calls on equal-state work. |
| Echo / dedup only | 129 | 129 | 386 | Watchdog, zero duplicates: each write generated a new event ID. |
| Echo / no-op only | 3 | 2 | 5 | One no-op terminates the echo. |
| Echo / filters only | 3 | 3 | 6 | Stops after one equal-state write emits an empty changed-field mask. |
| Echo / layered | 3 | 2 | 8 | One no-op; its receipt is retained. |
| Actual oscillation / no-op only | 129 | 129 | 258 | Watchdog; every transition really changes state. |
| Oscillation / dedup+no-op+filters+queue, no budgets | 129 | 129 | 386 | Watchdog despite all semantic filters and queue peak one. |
| Oscillation / layered | 17 | 17 | 51 | Hop boundary stops one delivery for inspection; this is interruption, not convergence. |
| Converging revisit / visited only | 2 | 2 | 4 | Wrong incomplete state A=1, B=2. |
| Converging revisit / layered | 6 | 5 | 17 | Correct A=4, B=4, then one no-op. |
| Irrelevant note change / no-op only | 2 | 1 | 3 | Unnecessary target read still occurs. |
| Irrelevant note change / layered | 1 | 1 | 2 | Dependency filter avoids the target attempt altogether. |
| Duplicate increment / naive | 3 | 3 | 6 | Target incremented twice. |
| Duplicate increment / layered | 2 | 2 | 6 | Target incremented once; replay lookup counted. |
| Modeled commit-before-ack restart / layered | 2 | 2 | 6 | Original receipt suppresses the second increment. |
| Downstream always transient-failing / layered | 4 | 1 | 8 | Three downstream failures, one quarantine, upstream remains committed. |
| Downstream succeeds on third attempt / layered | 4 | 2 | 9 | Two failures, final target transition, upstream unchanged. |
| Permanent downstream error / layered | 2 | 1 | 4 | One failure, immediate quarantine, no retry. |
| 64-way fan-out / naive or hop-only | 65 | 65 | 130 | Ready queue peak 64; hop bound does not constrain width. |
| 64-way fan-out / layered | 65 | 65 | 194 | Ready peak four, backlog peak 64; all targets eventually run. |
| 64-way fan-out / work-only twelve | 13 | 13 | 26 | Fifty-two deliveries cut, yet ready peak still 64. Work and admission limits solve different problems. |

The finite converging rule is `target = min(source + 1, 4)`. Starting A=1 gives B=2, A=3, B=4, A=4, then B=4 is a no-op. A small total budget of two also cuts this valid loop early. Every budget breach must therefore be reported as a policy stop, never silently relabeled successful convergence.

### Additional positive and negative assertions

- An intentionally wrong dependency declaration misses a real value change and leaves the target unchanged. Exact filters are only safe with complete dependency knowledge; unknown dependencies must invalidate conservatively.
- Duplicate delivery and the modeled restart window use an **increment** action, so no-op suppression alone cannot hide duplicate effects. It increments twice without stable identity and once with it. Another restart check proves the modeled root budget is retained rather than reset.
- Reordered latest-state snapshots `(revision 2, value 20), (revision 1, value 10)` leave the naive projection at 10; version-aware delivery keeps 20. Ordered domain history instead recovers revisions one then two, rejects missing revision one as a gap, and rejects a reused revision with conflicting content. Discarding an old-but-required domain fact is not equivalent to refreshing a projection.
- Three latest-state snapshots end at 30 with three projection assignments; coalescing ends at 30 with one assignment. Three distinct domain amounts 10, 20, 30 must total 60; unsafe coalescing totals 30. Stable duplicate domain IDs are removed without discarding different facts. These assignment counts are not resource database writes.
- Finite monotone reachability over a cyclic four-node graph reaches all four nodes in four frontier steps. Changing the seed to node three requires `{3}`; unioning the old result leaves obsolete facts. The special-case algorithm cannot handle retraction by pretending old derived facts remain valid.

## What related systems actually contribute

MobX's reaction runner has a `MAX_REACTION_ITERATIONS = 100` guard and detects failure to settle while draining its pending reactions. This supports the design lesson that reactive code needs a failure guard; it does not establish a correct ROM threshold, durable work budget, rollback or cross-process loop detector. MobX's separate reaction data/effect functions also show why precise dependency selection can avoid effects. Its docs recommend computed values for derived data and caution against relying on reaction execution order. [MobX runner source](https://github.com/mobxjs/mobx/blob/main/packages/mobx/src/core/reaction.ts), [MobX reactions](https://mobx.js.org/reactions.html).

RxJS `distinctUntilChanged` emits the first item and compares later selected keys against the last emitted key, using an equality comparator (default strict equality). That is local adjacent-value suppression, not global graph convergence, event-ID deduplication or durable idempotency. Inference for ROM: equality can suppress an equal-value echo, while a 0→1→0 sequence remains distinct on each step. A stable command identity and semantic equality are separate tests. [RxJS 7.x source](https://github.com/ReactiveX/rxjs/blob/7.x/src/internal/operators/distinctUntilChanged.ts).

Temporal exposes retry interval/backoff, maximum interval, maximum attempts and nonretryable error classification; its documented default maximum attempts is unlimited and it also discusses overall execution timeouts. ROM should deliberately select bounded retry duration/work and permanent-error classification rather than inherit an infinite policy. The model uses delays of one then two logical ticks and three total attempts; it does not execute Temporal, real timers, jitter, or its durability mechanisms. [Temporal retry policies](https://docs.temporal.io/encyclopedia/retry-policies).

Frank McSherry describes iterative fixed points, incremental differences and a case where correcting an input cannot be repaired merely by patching the final iterative result: prior derived values can feed the incorrect state back into the computation. This motivates separating pure incremental derivation from arbitrary side-effect chains. The four-node union example here is a much smaller special case, not an implementation or performance evaluation of differential dataflow. [Differential dataflow](https://www.frankmcsherry.org/differential/dataflow/2015/04/07/differential.html).

These are primary-source lessons, not recommendations to add MobX, RxJS, Temporal or differential-dataflow dependencies to ROM.

## Proposed layered ROM contract

1. **After-commit scheduling.** Commit upstream Resource, revision, facts and durable reaction obligation under the storage contract. A downstream action is a separate authorized transaction. Its failure leaves upstream committed. Publish downstream failure/progress explicitly; compensation, if wanted, is another named authorized action with its own possible failures.
2. **Stable identities and frozen meaning.** A delivery key contains event identity, reaction declaration/version, isolation scope and child ordinal. Keep the resolved action's canonical input/expected-revision meaning stable across a retry; reject conflicting identity reuse. Re-running nondeterministic callback code under the same key must not silently change its command. Persist outcomes and progress through the commit/ack window.
3. **Semantic no-op policy.** Compare canonical state under the authoritative action transaction before incrementing revision or emitting state-change facts. Apply this only where the action's definition permits it. Equal Resource state does not prove an explicit domain event, billing effect, notification or audit obligation is redundant. A no-op can still need a persisted action receipt.
4. **Conservative dependency filtering.** Route by declared source kinds and changed fields; include referenced reads, policy and configuration dependencies where relevant. Unknown/dynamic dependencies get broad invalidation. A field filter should reduce scheduling, not become an unverified correctness assumption hidden in a callback.
5. **Causal limits with honest terminal states.** Propagate root and parent identities, depth and accumulated work. Charge fan-out and retries against root budgets; persist accounting across restart and arbitrate it across workers. Also limit elapsed age and bytes. On breach, retain an inspectable stopped obligation and causal trail. Do not reset causality because a callback created another action or crossed a transport.
6. **Bound distinct queues.** Limit ready/in-flight count and bytes independently from durable backlog, journal retention and retry timers. Pull work incrementally from durable obligations. If durable capacity is exhausted, reject new obligations before promising acceptance or use an explicit recovery policy; never silently discard required facts. A single queue length or hop limit is insufficient.
7. **Bounded retry and isolation.** Classify transient, conflict, permanent, denied and unresolved outcomes. Reconcile uncertain commit by identity before retrying effects. Retry transient work with bounded attempts, elapsed age, capped backoff and production jitter; quarantine exhausted/permanent work. Avoid one poison delivery blocking unrelated partitions indefinitely while retaining the required ordering contract.
8. **Distinct projection/history profiles.** Latest-state projection may coalesce and refresh from authoritative state, with current authorization and stale-version protection. Domain-history consumers preserve scoped order, stable identities, gap detection and explicit processing checkpoints. Finite monotone pure derivations may opt into a fixed-point evaluator only under a declared lattice/order, convergence argument and retraction policy.

This policy retains the existing [committed-fact and recoverable-consumption requirements](../../openspec/changes/establish-rom/specs/event-reactivity/spec.md) and [shared mutation/idempotency rules](../../openspec/changes/establish-rom/specs/action-runtime/spec.md). It adds no second application domain entity or implicit transaction across the causal chain.

## Remaining risks, owner decisions and next tests

The minimal next integrated gate is one persisted after-commit A→B reaction with stable IDs, no-op semantics, bounded retry/quarantine and core admission. Add the converging/oscillating graphs and the commit-before-ack kill/restart vectors against the actual chosen adapter. Do not present this model's `Store::clone` as evidence that SQL atomicity, process recovery or broker acknowledgement timing works.

Before production claims, prove atomic state/event/receipt/obligation persistence; same-key conflicting command rejection; nonretryable policy failures; restart after every persistence boundary; unresolved commit resolution; CAS/expected-revision behavior under concurrent writers; stale-owner fencing; root-budget races across workers; retry counters and deadlines surviving restart; notification/effect outbox coordination; receipt-retention expiry; bounded backlog **bytes** and retention pressure; cancellation/shutdown while CPU and commit are active; policy revocation before callback execution and disclosure; poison-event partition progress; and descriptor/version migration of pending reactions.

Owner decisions remain: which actions permit no-op suppression, which reactions derive data versus cause required effects, ordering partition, supported convergence/monotonicity declarations, root hop/work/age/byte quotas, retry horizon and jitter, quarantine/operator-resume behavior, compensation requirements, persistent budget storage and receipt retention. A manual resume must deliberately preserve or revise budget/identity semantics; blindly resetting them can recreate the original storm.

A bounded chain is not necessarily a correct chain; a quiescent queue may conceal a wrongly filtered dependency; a repeated resource is not necessarily a loop error; and an empty ready queue can coexist with a large durable backlog. The executable counterexamples make each of those distinctions observable.
