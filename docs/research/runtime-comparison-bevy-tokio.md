# Execution foundation: Bevy versus Tokio and Rayon

Research date: 2026-10-02. **Decision: Tokio plus Rayon, explicitly selected by the user after reopening the Bevy choice for comparison.** Bevy research is retained as historical evidence. This note records the rationale and integration constraints. No production runtime was implemented. No comparative performance benchmark was run.

Evidence labels: **documented** means verified against primary sources; **tested** means an executed local probe; **judgment** means an architectural inference for ROM, not an upstream guarantee.

## Why this choice fits ROM

**Judgment:** Tokio plus Rayon is the better fit for ROM's currently specified backend workload and preference for reuse and simplicity. Most execution involves awaiting persistence, handling concurrent actions, recovering pending reactions, and performing occasional substantial CPU work. Tokio supplies I/O, timers, and task scheduling together. Rayon supplies a CPU pool. Bevy adds useful ECS access analysis and system scheduling, but those mechanisms do not enforce ROM's database transactions or durable reaction semantics. [Tokio runtime services](https://docs.rs/tokio/latest/tokio/runtime/index.html), [Rayon pool](https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html), [Bevy scheduling](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/index.html).

This is a fit assessment, not a claim that Tokio is universally faster or that Bevy cannot run a backend. Bevy would become more compelling if ROM's demonstrated workload required many interacting in-memory systems with explicit component access and schedule dependencies, or if the embedding application already centered on a Bevy world. Those are not present requirements.

## Comparative evidence

| Concern | Headless Bevy | Direct Tokio plus Rayon | ROM consequence |
| --- | --- | --- | --- |
| Durable action transaction | Systems coordinate execution; ECS access checks concern in-memory data. | Async jobs can await the persistence adapter directly. | Neither supplies atomic state/revision/receipt/events. Database concurrency and idempotency remain ROM contracts. Tokio needs fewer execution handoffs for the expected I/O path. |
| Recoverable reactions | Messages and observers can wake workers, but are transient. | Channels and tasks can wake workers, but are transient. | Both require durable pending work, acknowledgement, retries, and restart recovery. No correctness advantage from either notification mechanism. |
| Same-resource concurrency | Non-conflicting ECS systems can still write the same database row. | Independent tasks can also write the same row. | Atomic expected-revision checks are mandatory in both. A local mutex or ECS schedule is not distributed concurrency control. |
| Independent resources | Scheduler parallelizes systems with non-conflicting declared access. | Async tasks overlap I/O; Rayon parallelizes pure CPU work. | Current requirements need independent actions, not necessarily a graph of ECS systems. Avoid global locks in either design. |
| Fields and extensions | `Plugin` configures an `App` with lifecycle hooks. | Explicit Rust registries and domain traits are independent of the executor. | Bevy plugin registration does not define canonical field encoding, validation, migration, authorization, or adapter capability contracts. These stay in ROM either way. |
| Headless embedding | `App::update` and a custom runner permit headless execution; the App has thread-affinity constraints. | Host can supply a configured runtime/handle; tasks wake on I/O and timers. | A demand-driven Bevy service adds runner/lifecycle/wakeup integration. Tokio already supplies the needed event loop. |
| HTTP and database | Can bridge to compatible I/O runtimes. | Axum targets Tokio; SQLx supports a Tokio runtime feature. | The likely HTTP/database stack can share Tokio instead of bridging Bevy and Tokio. HTTP remains a separate adapter. |
| Cancellation and bounds | Retained task handles cancel on drop; ROM still needs admission, supervision, and completion policy. | `JoinSet`, bounded channels, semaphores, and lifecycle tokens provide reusable primitives. | Both need a commit-safe cancellation policy. CPU work must retain capacity until it actually stops. |
| Testing and diagnosis | Manual updates and schedule ambiguity diagnostics are useful. | Paused Tokio time helps timer/retry tests; task results expose errors and panics. | Test domain behavior independently; run real database crash/race tests for both. Neither proves durability from an in-memory test. |
| Operational burden | Pool initialization, schedule phases, update progress, and possibly an I/O bridge. | Shared async runtime and shared CPU pool, each with explicit budgets. | Fewer execution concepts for this backend. Neither removes the need for supervision or deployment measurements. |

Documented bases: [Bevy task pools](https://docs.rs/bevy_tasks/latest/bevy_tasks/), [Bevy messages](https://docs.rs/bevy_ecs/latest/bevy_ecs/message/struct.Messages.html), [Bevy observers](https://docs.rs/bevy_ecs/latest/bevy_ecs/observer/struct.Observer.html), [Bevy Plugin](https://docs.rs/bevy_app/latest/bevy_app/trait.Plugin.html), [Bevy App](https://docs.rs/bevy_app/latest/bevy_app/struct.App.html), [Axum compatibility](https://docs.rs/axum/latest/axum/#compatibility), [SQLx runtime support](https://docs.rs/sqlx/latest/sqlx/#runtime-support), [Bevy Task cancellation](https://docs.rs/bevy_tasks/latest/bevy_tasks/struct.Task.html), [Tokio JoinSet](https://docs.rs/tokio/latest/tokio/task/struct.JoinSet.html), [bounded Tokio channels](https://tokio.rs/tokio/tutorial/channels), [Bevy ambiguity diagnostics](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/struct.ScheduleBuildSettings.html), [Tokio paused time](https://docs.rs/tokio/latest/tokio/time/fn.pause.html). The ROM-consequence column is judgment.

**ECS does not require a duplicated domain model**, but mapping every resource and field into ECS components would introduce a second representation and synchronization rules without a demonstrated query/scheduling benefit. Bevy could hold only jobs and queues instead. In that narrower use, much of its ECS advantage is unused. Stable ROM IDs, typed field registries, and database revisions remain necessary regardless. Bevy itself warns against treating serialized `Entity` handles as stable long-term identities. [Entity stability](https://docs.rs/bevy_ecs/latest/bevy_ecs/entity/struct.Entity.html).

Tokio does not infer task dependencies or detect conflicting domain accesses the way an ECS scheduler detects component conflicts. If ROM later needs a substantial declarative computation graph, evaluate an existing graph/scheduling facility against that requirement. Do not build that facility speculatively now.

## Concrete Tokio and Rayon integration

These are the selected architectural rules, still to be implemented:

1. **Host ownership.** The embedding application owns one shared Tokio runtime and one explicitly sized Rayon pool per intended host boundary. ROM receives handles/services. A standalone host may construct them. Do not create runtimes or pools per action, resource, reaction, or field plugin; do not mutate Rayon's global pool configuration from the library. Budget Tokio workers, Rayon workers, blocking threads, and other libraries together.
2. **Work placement.** Await database/network/timer operations on Tokio. Execute tiny bounded validation inline. Send substantial pure CPU jobs to Rayon. Use bounded `spawn_blocking` only for finite synchronous/blocking APIs, rather than routing CPU parallelism through a second default CPU pool. Never call synchronous `Rayon::install` or wait for Rayon completion directly on an async worker.
3. **Admission before execution.** Bound ingress count/bytes, async jobs, CPU submissions, and completion retention. Acquire a CPU permit before `ThreadPool::spawn`; move it into the closure. A thread limit bounds simultaneous CPU execution, not the queued job count. An unlimited population of tasks waiting for permits is also an unbounded queue.
4. **Result bridge.** Submit an owned job to the shared Rayon pool, compute a ROM result, and send it through a one-result channel. A dropped receiver is an ordinary abandoned reply, not a reason to panic. The supervisor retains job identity and tracks actual completion for shutdown. Report CPU panics deliberately; Rayon's unhandled spawned-task panic policy is not an application error-return contract. Unwinding recovery cannot catch an aborting panic.
5. **Cancellation ownership.** CPU admission remains held until the closure exits, including cancellation of its async waiter. The cancellation signal may be checked before work starts and between chunks, but it does not forcibly interrupt a running closure. The same rule applies to started `spawn_blocking` jobs. Do not allow caller disconnect to silently cancel an owned commit phase.
6. **Durable recovery.** Commit state, revision, scoped action receipt, and events atomically. Retry an uncertain outcome with the same identity/input; recover reactions from durable records. Shutdown stops admission, drains owned work under an explicit deadline, and leaves unfinished durable work discoverable. Executors provide scheduling, not these domain guarantees.

Documented support: Tokio explicitly recommends bounding CPU/blocking work and notes that started `spawn_blocking` work cannot be aborted. Rayon supports explicit pool thread counts and externally submitted owned tasks; its synchronous APIs wait for results. [Tokio blocking work](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html), [Rayon pool configuration and panic handler](https://docs.rs/rayon/latest/rayon/struct.ThreadPoolBuilder.html), [Rayon submission and synchronous execution](https://docs.rs/rayon/latest/rayon/struct.ThreadPool.html).

Reuse Tokio's scheduler, timers, bounded `mpsc`, semaphores, `JoinSet`, and Rayon's pool. Add `tokio-util::CancellationToken` for cooperative lifecycle signals where needed. ROM needs a small worker lifecycle and persistence-driven reaction loop, not a custom general-purpose scheduler, actor framework, or ECS compatibility layer. [Tokio shutdown pattern](https://tokio.rs/tokio/topics/shutdown).

## Candidate versions and toolchain

| Candidate examined | Manifest requirement | Status |
| --- | --- | --- |
| Tokio 1.53.1 | Rust 1.71, edition 2021; MIT. | Current source reviewed; not compiled in this research. |
| Rayon 1.12.0 / rayon-core 1.13.0 | Workspace Rust 1.80, edition 2021; MIT OR Apache-2.0. | Current source reviewed; combined bridge not compiled. |
| Bevy 0.19.1 | `bevy_ecs` requires Rust 1.95.0, edition 2024; MIT OR Apache-2.0. | Historical candidate; no compile probe on installed Rust 1.82. |

Sources: [Tokio release manifest](https://github.com/tokio-rs/tokio/blob/tokio-1.53.1/tokio/Cargo.toml), [Rayon release manifest](https://github.com/rayon-rs/rayon/blob/v1.12.0/Cargo.toml), [rayon-core manifest](https://github.com/rayon-rs/rayon/blob/v1.12.0/rayon-core/Cargo.toml), [Bevy ECS manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/Cargo.toml).

These are candidate pins, not an installed or audited full dependency set. Direct-crate compiler requirements do not prove that current transitive dependencies and chosen adapters resolve/build on Rust 1.82. Select a maintained toolchain, resolve and retain a lockfile for executable probes, inspect features/licenses, then compile the combination. No toolchain or container packages were changed.

## Evidence and acceptance probes

**Tested:** the earlier isolated Tokio 1.41.1 probe on Rust 1.82 passed five semantic assertions: bounded-channel overflow/drain, unsent-value destruction on cancellation, 100 jobs with at most four tracked tasks, a started blocking task surviving abort, and broadcast lag. Source and limitations are in [the executed probe record](rust-concurrency.md#executed-probe-record). This is neither a Tokio/Rayon integration test nor a comparison against Bevy.

**Unrun acceptance probes for the selected stack:**

| Probe | Falsifiable acceptance condition |
| --- | --- |
| CPU cancellation and capacity | With CPU admission bound 2 and two blocked closures, cancel both async waiters. A third CPU job must remain unadmitted until an actual closure exits. |
| Overload and pool ownership | With ingress bound 32, async bound 8, and CPU bound 2, flood fixed-size requests. Observed retained work respects declared bounds; rejection/backpressure is explicit; creating additional ROM handles creates no additional pools. |
| CPU result failure | Drop a reply receiver and inject an unwinding CPU panic separately. No lost permit, supervisor deadlock, or panic caused by abandoned reply; panic outcome follows the documented host policy. |
| Async responsiveness | Run deterministic CPU jobs alongside database requests and timers. Verify correct outputs and record p95/p99 request latency and timer lateness against a predeclared workload/SLO; tune Rayon granularity and budgets. No performance result is claimed yet. |
| Transaction race and recovery | Two connections/processes compete on one revision; inject crashes around commit and reaction acknowledgement. Require one valid transition, atomic state/events/receipt, recoverable delivery, and no duplicate action on same-identity retry. |
| No executor leakage | An example application registers a custom field and executes/reads a resource using ROM domain types. Changing executor internals requires no field schema, stable identity, or persistence migration. |

No new benchmark work or runtime implementation was performed for this decision. The chosen foundation reduces integration work; the probes establish whether its eventual implementation meets ROM's promises.
