# ROM 0.1.0: durable Work batching and group commit

Date: 2026-10-08. Status: source review and primary-source research; no implementation or benchmark admission.

## Recommendation

Keep the generic worker's durable barriers for this release. Do not prefetch Claims or defer Materialize as a transparent optimization.
Those changes alter callback order, lease timing, root budgets and observable Work state.
The mapper contract does not require purity. An ordinary Rust function can use global state or application services.

A bounded native transaction can apply an ordered list of pure Work transitions with fewer durable commits.
However, the current synchronous worker has no accumulation window that preserves its existing barriers.
Work batching alone also cannot establish the unchanged 10,000-Resource, 120-second seed on the measured storage envelope.

The next practical step is a durable storage-envelope investigation with the unchanged test and FULL/Immediate persistence.
Keep the failed workload gate explicit. A new supported environment needs its own successful execution; it does not erase these failures.

## Evidence and scope

The coordinator supplied the latest stage totals for build `907302a3...` and its 387-input source closure.
This researcher did not inspect that run's raw files. The abbreviated build identifier is not a complete independently verified identity.
The run created 2,200 Resources and drained 2,100 before deadline failure.
The [native-10 diagnosis](rom-0.1.0-native10-seed-diagnosis-2026-10-08.md) preserves earlier source reviews and terminal evidence.
Its original SQLite run created 2,500 Resources; redb created 3,800. The matched untraced host run fully drained 3,200 and failed.

| Coordinator-supplied category | Calls | Inclusive wall seconds |
| --- | ---: | ---: |
| Resource native commit | 2,200 | 43.248034681 |
| Work native commit | 4,306 | 62.402274084 |
| Commit metadata decode | Not supplied here | 1.429765674 |
| Retry-epoch metadata reads | Not supplied here | 6.376582435 |
| Commit shared preparation | Not supplied here | 2.588525454 |
| Work shared preparation | Not supplied here | 0.405251569 |

These intervals are wall time, not CPU cost. Observer overhead and host variation remain unmeasured.
The Work total covers multiple transition types and cannot establish an individual Claim or Materialize sync cost.
No change, native Cargo command, performance probe, process stop or cleanup was performed by this researcher.
Only this report was added.

The reviewed checkout has HEAD `1a2b7934139f3d7bb4cb04aaf4a90bd9405ab470`.
HEAD alone does not identify dirty source. These focused hashes identify inspected worker and transition inputs:

```text
ab7c3c496ec88f9c7629e5bb6f014f5f32944cc34709b8521c24605beb7eb89a  crates/rom/src/reactions/worker.rs
f476a49882ad3c324e6fe47f74f760fb064cd98056b4d375e9b5608fcbfbd800  crates/rom/src/reactions/claims.rs
98ac2033bf8956308e4516e2a7fccb0b978fa1e6fd4a1834dead00a2b20a96bf  crates/rom/src/reaction_work/incremental/engine.rs
f7f2bfae15f9464f96f0ba4ec24cfa7832098494562556351b8580bd4afefe3b  Cargo.lock
```

## Existing barriers and contracts

[`worker.rs`](../../crates/rom/src/reactions/worker.rs) processes one Claim, completes its callback path, then samples time for the next Claim.
The finite worker retains admission and supervised execution after caller cancellation.
It checks runtime closure between steps and returns only the number of processed Claims.
`WorkResult::Changed`, including Claim prefix maintenance without a returned claim, ends the current worker loop.

[`claims.rs`](../../crates/rom/src/reactions/claims.rs) checks the definition and current authority before a mapper runs.
It loads and authorizes the source before and after that mapper.
The mapper runs in the shared pool outside native transactions and the core gate.
Materialization resolves current target revisions after mapping, then writes children and parent completion atomically.
Each child ID derives from the existing root and path. Application-defined mapper results must not be reordered or regenerated speculatively.

An action Claim takes the command evaluator path in [`command.rs`](../../crates/rom/src/execution/command.rs).
Receipt replay and current authorization precede execution. A successful target bundle also completes its Claim atomically.
Do not extract that completion into a later Work batch: Resource, event, receipt, effects and Claim completion form one persistence boundary.
`Error::Unknown` can mean committed and causes invalidation. It is not rollback or permission to invoke the action again immediately.

[`channels/execution.rs`](../../crates/rom/src/channels/execution.rs) durably writes `DeliveryStarted` before invoking a provider.
This records an unknown delivery outcome for restart recovery and reconciliation.
The provider runs outside the gate. Delivery completion retains timeout, panic, reconciliation and stale-Claim handling.
Do not postpone `DeliveryStarted` until a batch closes or run a provider inside a writer transaction.

[`model.rs`](../../crates/rom/src/reaction_work/model.rs) exposes ClaimKey generation fencing, lease expiry, lifecycle revision and original budgets.
[`incremental/update.rs`](../../crates/rom/src/reaction_work/incremental/update.rs) uses ordered candidates and updates root usage during each Claim.
Claims can stop aged work or require reconciliation before selecting a later candidate.
Lease validation requires `until > now`; its exact boundary and `not_before` eligibility floor must remain intact.

## Why straightforward worker batching changes behavior

| Proposal | Concrete difference from the existing worker |
| --- | --- |
| Claim 32 items, then process them | A mapper can add a lower-ID child that the original loop would select before a previously pending item. |
| Claim several items from one root | Later attempts consume root usage before earlier processing can fail, stop, retry or add children. |
| Start all leases together | A slow earlier callback can exhaust a later item's lease before its callback starts. |
| Map several sources, then materialize | The next mapper runs before prior children and parent completion become visible. Global mapper state can observe this. |
| Resolve target revisions before the batch | Earlier action effects or concurrent mutations can change the expected revisions that the original loop would freeze. |
| Finish several callbacks together | A callback or storage failure loses the original committed prefix; visibility and restart behavior change. |
| Materialize and Claim next in one transaction | Time is sampled before the original post-commit point; an unknown acknowledgement can leave an extra unreturned lease. |

A zero-child mapper is not proof that callbacks are pure or that other reaction definitions have the same property.
Even a batch that preserves final rows can change intermediate Work revisions, callback count, ordering and unknown-outcome recovery.
Holding the writer transaction across callbacks would add lock duration and reentrancy hazards. It is excluded by the task's contract.

## Primary-source findings

SQLite WAL uses one writer at a time. With `synchronous=FULL`, the writer synchronizes WAL at each transaction commit.
This explains why changing the number of durable transactions can matter, but does not measure ROM's device latency.
See the official [WAL documentation](https://www.sqlite.org/wal.html).

SQLite recommends placing multiple writes in one transaction to share transaction overhead.
That recommendation concerns a common transaction boundary; it does not prove equivalence to several externally acknowledged transactions.
See [SQLite FAQ, question 19](https://www.sqlite.org/faq.html#q19).

The pinned redb version is 4.3.0. `Durability::Immediate` guarantees persistence when commit returns.
`Durability::None` requires a later Immediate commit for persistence. This report does not propose that mode.
See the versioned [redb durability contract](https://docs.rs/redb/4.3.0/redb/enum.Durability.html).

PostgreSQL describes synchronous group commit as several ready transactions sharing a WAL flush before success returns.
Its commit delay creates a window for other transactions to join. This differs from returning success before persistence.
An isolated sequential caller cannot supply another ready transaction while it waits for its current commit.
The last statement is an inference from the mechanism and ROM's call order, not a PostgreSQL throughput result.
See [PostgreSQL asynchronous and synchronous commit](https://www.postgresql.org/docs/current/wal-async-commit.html).

PostgreSQL permits `SKIP LOCKED` for queue consumers but explicitly describes its view as inconsistent.
That queue technique does not establish ROM's canonical ID ordering or Claim behavior.
See the [SELECT locking-clause contract](https://www.postgresql.org/docs/current/sql-select.html#SQL-FOR-UPDATE-SHARE).

RocksDB supports ordered atomic `WriteBatch` edits and synchronous writes that wait for persistent storage.
These are useful examples of distinct batch and durability contracts. They do not justify a new ROM adapter for this release.
See the official [RocksDB basic operations](https://github.com/facebook/rocksdb/wiki/Basic-Operations#atomic-updates).

## Ranked strategies

The ranking favors a small semantic surface. None has a measured ROM prototype speedup.

| Rank | Strategy | Implementation cost | Potential under the measurements | Correctness and applicability |
| --- | --- | --- | --- | --- |
| 1 | Bounded transaction batch of already prepared, pure WorkUpdate values | Low to medium for storage; high if used to redesign the worker | Can share Work commit cost when callers already possess independent updates. No demonstrated accumulation window in the unchanged seed. | Keep the generic loop. Expose a distinct atomic batch contract or trusted adapter capability; do not claim equivalence to sequential public calls. |
| 2 | Adapter writer queue groups independently ready calls before one FULL/Immediate commit | High across concurrency, read visibility, rollback and supervision | Useful only with multiple ready callers. Current core gates and sequential create-await calls prevent a useful group on this seed. | Optional adapter path requires core authorization and arbitration design first. Do not release the authority gate merely to collect writes. |
| 3 | Explicit application mutation batch with one durable response | High for API design; potentially highest reduction of primary commit count | Could share both primary and Work enqueue transaction overhead. The unchanged sequential seed does not request such a batch. | A new ergonomic API and failure contract, not a transparent fix. Each item retains identity, authorization, events and revision checks. |

For strategy 1, start with native `prepare_update` followed by native delta application for each logical update inside one transaction.
Each subsequent preparation must read the transaction after earlier application. Both adapters invalidate their ReadFence after application.
Reject a delta from a previous transaction or a fence invalidated by earlier publication.
Never concatenate opaque deltas prepared against one stale snapshot.

The shared engine's `versioned` set increments an existing record once per WorkEdit publication.
A single enlarged WorkEdit would coalesce revisions across several top-level updates.
The independent [`reference/ledger.rs`](../../crates/rom/src/reaction_work/reference/ledger.rs) oracle increments once per changed record per `apply`.
To preserve logical revision counts, prepare/apply separate logical deltas or introduce explicit step checkpoints with corresponding oracle tests.
`DeliveryFinished` internally invokes Finish but remains one top-level transition; do not give it an extra revision.

An all-or-nothing batch must reject every update when any step fails. That is a new contract compared with a committed sequential prefix.
A per-item result queue must isolate rejected candidates without losing earlier valid candidates, then report success only after durable commit.
SQLite savepoints can isolate candidate edits, but redb requires an equivalent rollback design. Shared transition logic must remain adapter-free.
All batching needs fixed count/byte limits, bounded waiting and tracked shutdown ownership.
Preserve public API paths. Do not add a mandatory batch method to custom adapters without a compatibility design.

For strategy 3, prefer one bounded list of ordinary invocations over a user-managed transaction handle.
Document all-or-nothing versus partial results, duplicate identities, intra-batch dependencies, stale revisions and unknown outcomes before implementation.
Run application callbacks outside native transactions. Revalidate proposals and current authority at publication.
Do not re-run an impure handler merely because batch arbitration changed its expected revision.

## Minimal test and fault cases

1. Compare logical batch outcomes with separate oracle `apply` calls, including every WorkState, Claim prefix effect, returned WorkResult and revision.
2. Repeat updates to the same ID. Cover Claim then Finish and DeliveryStarted then DeliveryFinished. Verify per-step revisions and nested Finish behavior.
3. Use two roots and one shared root at its final work budget. Verify attempts, root usage, generation, stop-reason precedence and child IDs.
4. Have a mapper insert a child whose ID sorts before the next pending source. Record the callback trace and confirm the existing generic ordering.
5. Use a mapper that increments global state and queries durable Work through a trusted application handle. Detect speculative or reordered callbacks.
6. Advance a controlled clock across eligibility, age, retry and lease boundaries. Include a callback longer than the remaining lease.
7. Revoke authority or change the source while a mapper runs. Reject publication without changing Work, Resource, receipt or events.
8. Fail native publication after each delta write and immediately before commit. Verify the documented atomic batch or per-item rejection contract after restart.
9. Lose acknowledgement after commit. Return Unknown, recover durable state and prevent duplicate action effects through existing receipts.
10. Crash after DeliveryStarted, during provider execution and after DeliveryFinished commit. Preserve reconciliation and delivery-profile behavior.
11. Cancel the caller after admission and request shutdown during queue wait. Accepted work must remain tracked; no writer/provider callback may escape supervision.
12. Test count/byte overflow, stale fences, generation overflow, revision overflow, corrupt touched records and unavailable redb after uncertain commit.

These are proposed cases, not executed tests. A future change needs affected conformance checks and the full local verifier before integration.

## Throughput envelope and unresolved assumptions

The supplied primary native commit mean is 19.658197582 milliseconds per Resource.
At the same mean, 10,000 sequential primary commits alone require approximately 196.581975823 seconds.
Even removing every measured Work commit and all other work would exceed 120 seconds under that constant-cost assumption.
This is a conditional extrapolation, not a hardware lower bound or measured 10,000-row execution.

The target allows 12 milliseconds per Resource for all creation and reaction work combined.
Primary commits therefore need more than a 1.638-fold improvement over the supplied mean, even before reaction costs.
Work native commits average 14.491935458 milliseconds across their mixed call population.
The measured Resource and Work native intervals total 105.650308765 seconds; do not interpret them as an exclusive CPU decomposition.

A durable environment with lower sync latency might satisfy the unchanged contract.
The existing matched host run improves progress but still fails; moving off the loop volume alone has not established admission.
An honest supported envelope must identify filesystem, backing storage, durability, load, resource budgets and source/build identity.
Validate it with the unchanged workload and original deadlines. Keep every failed store and evidence set.

Unproved assumptions include stable per-commit cost, disabled-observer cost, concurrent grouping capacity and any mapper purity.
No source review establishes that a host cache acknowledges only power-loss-safe writes.
No report may replace the existing failed test with fewer Resources, longer deadlines, shorter history, weaker durability or concurrent application batching.

Technical vocabulary follows the project writing rules: Resource, Work, ClaimKey, commit, event, receipt, adapter and unknown outcome.
The prose uses STE guidance without a claim of certification. Full dictionary compliance was not assessed.
