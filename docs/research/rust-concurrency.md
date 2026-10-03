# Rust concurrency and parallelism for ROM

Research date: 2026-10-02. Status: **Tokio plus Rayon is the selected execution foundation**; release pins and implementation remain pending. Sources are public first-party documentation. The five isolated Tokio probes below ran successfully. Database, Rayon, Loom, and property-testing experiments are proposed, not reported as completed. This report deliberately separates the latest documentation from the older release used in the probes.

## Recommendation

Use **Tokio for asynchronous execution and I/O, and Rayon for CPU parallelism**. This explicit selection supersedes the [historical Bevy runtime design](bevy-runtime.md). ROM owns the domain model, action semantics, and persistence guarantees. See [the comparison and integration decision](runtime-comparison-bevy-tokio.md) for the rationale, candidate releases, and acceptance probes.

The host owns a shared Tokio runtime and an explicitly sized Rayon pool, with a combined thread budget. Bound admission before spawning work and treat queues as scheduling tools. Persist the authoritative state, events, action outcomes, and reaction progress. Keep small synchronous field checks synchronous; use Rayon for substantial CPU jobs. Retain CPU permits inside submitted closures until actual completion, even if their async callers cancel.

This recommendation serves the existing [action contract](../../openspec/changes/establish-rom/specs/action-runtime/spec.md) and [event contract](../../openspec/changes/establish-rom/specs/event-reactivity/spec.md): stale revisions conflict, retries have scoped identity, events describe committed transitions, and reactions recover after restart. Resource remains the sole domain entity; scheduler tasks and delivery records are supporting machinery. See also [framework and crate research](frameworks-and-rust-crates.md).

## Choose the primitive by its lifetime and work

| Primitive | Verified behavior | Proposed ROM use and boundary |
| --- | --- | --- |
| Tokio tasks and `JoinSet` | Spawned tasks run independently; the set yields completion order. Dropping the set aborts its tasks. `abort_all` requests cancellation; joining is still necessary to observe completion. | Supervised action/reaction workers with explicit admission limits. Consume every task result and distinguish domain failures, panics, and cancellation. Neither task IDs nor completion order are durable event identities/order. |
| `futures` combinators | `buffer_unordered(n)` buffers at most `n` futures and yields completion order; `buffered(n)` returns results in input order. | Small, request-scoped concurrent reads. Use a nonzero configured bound. These combinators do not create a CPU thread pool. Returning results in order does not serialize side effects. |
| `FuturesUnordered` | Futures are driven when the collection is polled; inserting one does not poll it immediately. | Dynamic scoped fan-out where borrowing is useful. Limit the collection explicitly; it is not an admission controller. Buffering already-spawned handles does not bound their tasks. |
| Tokio `spawn_blocking` | Runs work on blocking threads, with queuing after the configured thread limit. Already-started blocking tasks cannot be aborted. | Occasional finite blocking adapter work, behind separate admission control. Hold its permit inside the closure until actual completion, including when the async waiter disappears. |
| Rayon | A custom pool can have a specified maximum thread count. Parallel work runs in that pool; `install` synchronously returns its result. | Sustained pure CPU work, such as a demonstrated expensive codec. Bound submissions separately from threads. Bridge results asynchronously; calling `install` directly on an async worker blocks that worker. |

Sources: [Tokio JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html), [futures stream combinators](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html), [FuturesUnordered](https://docs.rs/futures/latest/futures/stream/struct.FuturesUnordered.html), [spawn_blocking](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html), [Rayon pool configuration](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html), [Rayon ThreadPool](https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html).

A `JoinSet` supplies useful ownership machinery, not a complete structured-concurrency policy. Child work outside the set needs its own ownership. Prefer a `join_next` loop to inspect individual failures. `JoinSet::shutdown` aborts and waits while ignoring panics, so use explicit result handling when failures must be recorded. This is documented behavior, not an instruction to abort a commit on graceful shutdown. [JoinSet lifecycle methods](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html).

Native plugins remain trusted in-process Rust code. Cooperative cancellation cannot enforce a hard execution deadline on a plugin that never yields or checks its stop signal. Future isolation is a separate host-capability design, not a reason to start with an actor or WASM framework.

## Queues and backpressure

Tokio's bounded `mpsc` waits for capacity; `try_send` can instead expose overload immediately. A successful send does not establish that the receiver consumed the item. Cancelling `send(value)` in `select!` drops the unsent value. If the caller must be able to recover a value, reserve capacity before moving it. Cancelling a reservation loses its queue position. [Sender semantics](https://docs.rs/tokio/latest/tokio/sync/mpsc/struct.Sender.html).

Proposed ROM limits must cover queued items, active work, payload sizes, database connections, CPU submissions, and retries. A bounded channel does not bound the number of blocked producers retaining payloads. Similarly, spawning unlimited tasks that each wait for a semaphore bounds execution but leaves unlimited waiting tasks. Acquire admission before spawning, or let a bounded worker supervisor pull work only when it has capacity. Apply ingress size/concurrency limits before expensive decoding. [Tokio backpressure guidance](https://tokio.rs/tokio/tutorial/channels), [Semaphore admission example](https://docs.rs/tokio/latest/tokio/sync/struct.Semaphore.html).

Reserve independent capacity for action traffic and reactions so a retry storm cannot occupy every connection. Avoid a reaction holding a permit while synchronously waiting for a child action that requires the same exhausted pool. Claim durable work only when execution capacity is available, or explicitly account for queue time in lease handling. Numeric limits are deployment settings to determine through measurement, not universal crate defaults.

| Channel | Documented semantics | Appropriate ROM interpretation |
| --- | --- | --- |
| Bounded `mpsc` | Multiple producers, one receiver, finite capacity. | Admission to one supervisor; volatile work or IDs referencing durable work. |
| `oneshot` | One result between sender and receiver. | Reply from a worker or CPU bridge; disappearance of the reply receiver must not erase a committed outcome. |
| `watch` | Stores only the latest value. | Configuration or reconciliation hints; intermediate transitions may be skipped. |
| `broadcast` | Bounded retention; a slow receiver gets `Lagged` after old values are removed. | Live notification hints with reload/replay after lag. Never the only source of durable reaction input. |

Sources: [Tokio synchronization module](https://docs.rs/tokio/latest/tokio/sync/index.html), [watch](https://docs.rs/tokio/latest/tokio/sync/watch/index.html), [broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html).

## Cancellation must respect the commit boundary

Tokio describes graceful shutdown as detecting shutdown, notifying components, and waiting for them. `tokio-util::CancellationToken` signals a cooperative request; child tokens allow subordinate cancellation without cancelling their parent. A cancellation signal is not proof of rollback. [Graceful shutdown](https://tokio.rs/tokio/topics/shutdown), [CancellationToken](https://docs.rs/tokio-util/latest/tokio_util/sync/struct.CancellationToken.html).

Proposed ROM lifecycle:

1. Stop accepting new actions and claiming new reaction deliveries.
2. Cancel idle waits and work that has not entered a protected commit phase.
3. Allow admitted commits a bounded grace period and observe their outcomes. Disconnecting an HTTP request should not by itself drop the supervisor that owns this phase.
4. Drain task results. On forced shutdown, recover unfinished work from persistence at restart.
5. Return or document an indeterminate outcome when the connection fails around commit; resolve it by looking up or retrying the same scoped action identity and identical input. Do not report a confirmed rollback merely because a timeout elapsed.

SQLx documents rollback when a live transaction is dropped without commit or rollback. That API behavior does not prove the outcome of a `COMMIT` whose response was lost. ROM must test each adapter's cancellation and connection-reuse behavior. Persist an action's idempotency result atomically with its transition and events. [SQLx transaction lifecycle](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html).

Timeouts on blocking work do not stop the work. Prefer cooperative chunk boundaries for expensive native operations and retain capacity until execution finishes. Tokio warns that started blocking work can keep runtime shutdown waiting. A shutdown timeout stops the wait, not execution. [Blocking-task shutdown](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html).

## Resource concurrency and recoverable reactions

These are proposed ROM adapter obligations, not guarantees supplied by any Rust executor:

- Use an atomic revision predicate, conceptually `UPDATE ... WHERE resource_id = ? AND revision = ?`, and inspect affected rows. Insert the resulting events and scoped idempotency outcome in that same transaction. A zero-row update must become an appropriate conflict/not-found outcome, never silent success.
- Test from separate connections and processes. A per-resource in-memory mutex can reduce local contention but cannot enforce correctness across hosts. Do not automatically rerun a stale action whose validation or external observations may have changed.
- PostgreSQL Read Committed rechecks an update's predicate after a concurrent updater commits, supporting this single-row comparison pattern. Cross-resource invariants need separately specified locking/isolation and transaction retry behavior. [PostgreSQL isolation semantics](https://www.postgresql.org/docs/current/transaction-iso.html).
- Parallelize independent resource work while preserving revision order in persisted resource history. Decide separately whether each reaction requires sequential per-resource delivery or can reconcile latest state. Completion order from tasks is not journal order.
- Persist delivery identity, consumer identity/version, attempt state, and acknowledgement/checkpoint. If claims use leases, make completion conditional on the current claim generation so a stale worker cannot acknowledge a newer worker's claim. A lease alone does not stop an old worker's external effect; use destination idempotency or enforceable fencing where available.
- Derive stable action identities for retried reactions, including reaction version and effect identity when one event produces several actions. If reaction action commit and acknowledgement cannot share a transaction, retry must find the original action outcome before acknowledging. External effects need their own idempotency/outbox contract.
- Bound retries, delay/backoff, and cascade depth or causation budgets. A no-op reconciliation must have a deliberate event policy. Preserve failed work for inspection instead of silently dropping it after a retry limit.

Neither Loom nor a successful local stress loop proves database isolation, durable delivery, broker behavior, or exactly-once external effects.

## Smallest stack and test strategy

The selected execution stack is **Tokio** and **Rayon**, with `JoinSet` for async supervision and **tokio-util** for cooperative lifecycle cancellation when needed. The embedding application owns runtime/pool creation, process signals, budgets, and shutdown; share handles instead of creating a pool per resource or action. Reuse `tracing` for queue wait, action duration, conflicts, claim retries, commit uncertainty, and shutdown outcomes without logging resource contents.

Add **futures-util** only when stream combinators simplify a concrete bounded operation. Rayon has a separate CPU budget and bounded submission policy; it is the selected CPU executor, not another default pool for every workload. Avoid additional channel/actor crates until the selected primitives fail a demonstrated requirement. Reuse their scheduling instead of building a generic scheduler.

Use **proptest** as a development dependency for generated action sequences, presence-aware patches, codec round trips, duplicate inputs, and crash/retry model transitions. It generates test inputs and shrinks failures. Keep a small reference state machine and compare observable outcomes; random inputs alone do not explore thread schedules. [Proptest documentation](https://proptest-rs.github.io/proptest/proptest/index.html).

Use **Loom** only for small custom synchronization mechanisms, such as a shared claim-generation or admission/close state machine. It explores modeled interleavings and requires its replacement synchronization types. It cannot see ordinary uninstrumented concurrency. State-space limits and relaxed-memory limitations must be recorded. It does not model a production SQL server or arbitrary Tokio runtime simply because a test calls `loom::model`. Prefer standard primitives over inventing locks that require this work. [Loom usage and limitations](https://docs.rs/loom/latest/loom/).

Before adopting releases, pin a supported Rust toolchain, resolve compatible crate versions/features, and review licenses and maintenance. The existing Rust 1.82 container is a bootstrap toolchain, not the project's minimum supported Rust version. No combined latest-stack build was performed.

## Experiment matrix

| Experiment | Concrete setup | Required observation | Status |
| --- | --- | --- | --- |
| Admission and overload | Capacity 1 channel; then 100 yielding jobs with supervisor bound 4. | Overflow is explicit; all admitted jobs complete; tracked tasks never exceed 4. | Isolated Tokio probe passed. |
| Cancellation loses unsent value | Fill a channel; race pending send against a ready cancellation branch; count destructor calls. | Unsent payload drops exactly once; original queued payload remains. | Isolated Tokio probe passed. |
| Blocking cancellation | Wait for a blocking closure's start signal, call abort, then release it. | Closure still completes and returns its result. | Isolated Tokio probe passed. |
| Live subscriber lag | Capacity 2 broadcast channel; send 3 items before receiving. | Receiver reports one missed item, then gets retained items. | Isolated Tokio probe passed. |
| CPU isolation | Compare inline CPU work, bounded `spawn_blocking`, and the selected custom Rayon pool; 1/2/4 CPU workers; same deterministic inputs alongside I/O timers. | Correct outputs; record throughput, p95/p99 timer lateness, CPU and memory to tune job granularity and pool limits. | Proposed. |
| Revision race | Two independent adapter connections read one revision; barrier before competing updates; repeat with two processes. | One winner, one conflict, one revision increment, and exactly the winner's event set. | Proposed; needs adapter/database. |
| Atomicity and uncertain commit | Inject failure after state write, after event insertion, before commit, and after server commit before client response. | State/events/outcome remain atomic; same-identity retry resolves uncertainty without a second transition. | Proposed; needs transaction fault injection. |
| Reaction restart and stale lease | Stop worker before action, after action commit, and before ack; expire/reclaim lease while old worker resumes. | Recoverable delivery, deduplicated action, rejected stale ack, documented external-effect limits. | Proposed; needs durable worker. |
| Ordering and fairness | Hot resource plus many cold resources; parallel publishers and slow reaction; one long transaction allocates an event ID before a short transaction commits. | Ordered resource history; no skipped committed event from cursor advancement; bounded cold-resource latency. | Proposed. |
| Shutdown and overload cascade | Fill queues, cancel callers, block one plugin, request graceful shutdown; retrying reactions produce child actions. | No lost durable work, no permit leak, no same-pool deadlock, explicit unfinished/uncertain outcomes. | Proposed. |
| Model exploration | Generated retry/crash sequences against reference model; Loom model only for custom synchronization. | Recorded seeds/minimal failures and explicit model bounds; no unsupported claim of full-system proof. | Proposed. |

Performance experiments need fixed inputs, recorded hardware and runtime settings, a warmup, repeated runs, and correctness assertions. No performance conclusion follows from the semantic probes below.

## Executed probe record

Ran in native `rom-dev`, entirely under `/tmp/rom-concurrency-probe`, using Rust 1.82.0 and Cargo 1.82.0. Pinned Tokio 1.41.1 to test on the bootstrap compiler; current documentation was also reviewed but its newest releases were not built. `cargo run --quiet` exited 0 and printed all five `PASS` lines. There was no database, broker, ROM implementation, timing benchmark, or host/container configuration change. The generated lockfile remains in that temporary directory; this appendix preserves the source and exact direct dependency, not a permanent transitive lockfile.

To repeat, create a disposable Cargo binary with the following manifest and source, then run `cargo run --quiet`. Dependency resolution may differ without the original lockfile.

```toml
[package]
name = "rom-concurrency-probe"
version = "0.0.0"
edition = "2021"

[dependencies]
tokio = { version = "=1.41.1", features = ["rt-multi-thread", "macros", "sync", "time"] }
```

```rust
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use tokio::{sync::{mpsc, oneshot, broadcast}, task::JoinSet};

struct MarkDrop(Arc<AtomicUsize>);
impl Drop for MarkDrop {
    fn drop(&mut self) { self.0.fetch_add(1, Ordering::SeqCst); }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let (tx, mut rx) = mpsc::channel(1);
    tx.send(1).await.unwrap();
    assert!(matches!(tx.try_send(2), Err(mpsc::error::TrySendError::Full(2))));
    assert_eq!(rx.recv().await, Some(1));
    tx.send(2).await.unwrap();
    assert_eq!(rx.recv().await, Some(2));
    println!("PASS bounded mpsc refuses extra item and resumes after drain");

    let drops = Arc::new(AtomicUsize::new(0));
    let (tx, mut rx) = mpsc::channel(1);
    tx.send(MarkDrop(drops.clone())).await.unwrap_or_else(|_| panic!());
    tokio::select! {
        biased;
        _ = tx.send(MarkDrop(drops.clone())) => panic!("full channel accepted send"),
        _ = std::future::ready(()) => {}
    }
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    drop(rx.recv().await.unwrap());
    assert_eq!(drops.load(Ordering::SeqCst), 2);
    println!("PASS cancelled send drops the unsent value");

    let mut set = JoinSet::new();
    let mut completed = 0;
    let mut max_tracked = 0;
    for i in 0..100 {
        if set.len() == 4 {
            set.join_next().await.unwrap().unwrap();
            completed += 1;
        }
        set.spawn(async move { tokio::task::yield_now().await; i });
        max_tracked = max_tracked.max(set.len());
    }
    while let Some(result) = set.join_next().await { result.unwrap(); completed += 1; }
    assert_eq!(completed, 100);
    assert_eq!(max_tracked, 4);
    println!("PASS admission before spawn: 100 completed, at most 4 tracked tasks");

    let (started_tx, started_rx) = oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let task = tokio::task::spawn_blocking(move || {
        started_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        7
    });
    started_rx.await.unwrap();
    task.abort();
    release_tx.send(()).unwrap();
    assert_eq!(task.await.unwrap(), 7);
    println!("PASS already-started spawn_blocking survives abort");

    let (tx, mut rx) = broadcast::channel(2);
    tx.send(1).unwrap(); tx.send(2).unwrap(); tx.send(3).unwrap();
    assert!(matches!(rx.recv().await, Err(broadcast::error::RecvError::Lagged(1))));
    assert_eq!(rx.recv().await.unwrap(), 2);
    assert_eq!(rx.recv().await.unwrap(), 3);
    println!("PASS broadcast reports lag and loses oldest value");
}
```
