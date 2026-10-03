# Bevy as ROM's execution engine

Research date: 2026-10-02. Status: **superseded historical research**. The user subsequently selected **Tokio plus Rayon**; see [the runtime comparison and decision](runtime-comparison-bevy-tokio.md) and [current concurrency guidance](rust-concurrency.md). The Bevy proposals below are retained for reference, not current implementation instructions. Reviewed official Bevy documentation identifying release 0.19.1 and that release's source manifests. No Bevy compile or runtime probe was performed. Bevy needs a compiler newer than the installed toolchain.

## Historical proposal and boundary

The historical proposal was to build ROM on a headless Bevy application and scheduler. Bevy would execute work. ROM would define every operation and semantic rule concerning domain resources. Applications would register kinds and extensions, submit actions, read resources, and consume committed changes through ROM's API. They would not need a Bevy `World`, `Entity`, `Resource`, system, or component.

That proposal temporarily replaced an earlier Tokio-first recommendation. It would have limited Tokio to adapters requiring its I/O driver and used Bevy for ROM scheduling and CPU pools. The subsequent explicit selection of Tokio plus Rayon superseded that proposal. [Rust concurrency research](rust-concurrency.md) now describes the selected foundation.

The remaining sections preserve the unimplemented Bevy design and its research findings. Its proposed runtime boundary was an internal module behind the main library, without a public generic executor framework or required ECS entity per domain resource. Any ECS projection would have been optional and reconstructible; persisted ROM state remained authoritative.

## Minimum headless building blocks

| Building block | Official capability | ROM decision |
| --- | --- | --- |
| `bevy_app` | `App` provides lifecycle, plugins, schedules, `update`, and a replaceable runner. | Internal host for ROM scheduling and lifecycle. |
| `bevy_ecs` | Systems, scheduling, data-access conflict detection, and parallel execution of non-conflicting systems. | Internal work coordination; public domain types stay independent. |
| `bevy_tasks` | Compute, asynchronous compute, and I/O-oriented pools. | Explicitly bounded job submission; retain and supervise task handles. Direct dependency only when its API is used. |
| `TaskPoolPlugin` | Initializes Bevy's default task pools and registers main-thread pool ticking. | Explicit pool configuration owned by the embedding host. |
| `ScheduleRunnerPlugin` | Headless run-once or repeated scheduling. | Useful for a first smoke test; use a custom demand-driven runner for the eventual service loop. |

Sources: [App](https://docs.rs/bevy_app/latest/bevy_app/struct.App.html), [ECS scheduling](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/index.html), [Bevy tasks](https://docs.rs/bevy_tasks/latest/bevy_tasks/), [TaskPoolPlugin source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_app/src/task_pool_plugin.rs), [ScheduleRunnerPlugin](https://docs.rs/bevy_app/latest/bevy_app/struct.ScheduleRunnerPlugin.html).

Proposed manifest for a future isolated compatibility probe, **not a verified dependency set**:

```toml
[dependencies]
bevy_app = { version = "=0.19.1", default-features = false, features = ["std"] }
bevy_ecs = { version = "=0.19.1", default-features = false, features = ["std", "multi_threaded"] }
# Include directly only when using task-pool APIs:
bevy_tasks = { version = "=0.19.1", default-features = false, features = ["multi_threaded"] }
```

The manifests expose these features. ECS `multi_threaded` enables the task crate's multithreading; that task feature enables `async_executor`. Disabling defaults avoids opting into reflection/backtrace features merely to schedule work. No renderer, window, assets, audio, scene system, or full `bevy` crate is needed for this proposed composition. Cargo feature unification must still be inspected in the resolved dependency graph. [bevy_app manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_app/Cargo.toml), [bevy_ecs manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/Cargo.toml), [bevy_tasks manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_tasks/Cargo.toml).

`MinimalPlugins` is a headless convenience group, but includes time and frame-count support as well as pools and a runner. Its default loop runs as fast as possible. Prefer explicit plugins for ROM; domain retries and leases should have deliberately specified clock/deadline semantics rather than inherit game-frame assumptions. [MinimalPlugins](https://docs.rs/bevy/latest/bevy/struct.MinimalPlugins.html).

## Keep the vocabularies separate

| ROM concept | Bevy concept | Mapping rule |
| --- | --- | --- |
| Resource: the sole domain entity, with durable ID, kind, fields, revision, and allowed actions. | `Resource`: a unique value of a Rust type in a `World`. | They are different concepts. Bevy resources may hold internal queues, registries, or scheduler configuration. They do not define ROM resource semantics. |
| Stable domain identity. | `Entity`: opaque ECS handle with generation and allocation behavior. | Never expose/persist the handle as a ROM identifier. An optional projection maps stable ROM ID to a local handle. |
| Resource field type and kind schema. | Rust component type and archetype. | No required one-to-one mapping; domain validation, encoding, schema versions, and migrations belong to ROM. |
| Action and durable committed event. | System, `Commands`, `Message`, observer `Event`. | Internal execution/notification mechanisms may carry ROM IDs or outcomes; none constitutes a durable action commit. |

Bevy documents its singleton resource semantics and explicitly warns that serialized `Entity` values have no long-term wire-format compatibility guarantee. It recommends a secondary identifier rather than synchronizing entity handles across application instances. [Bevy Resource](https://docs.rs/bevy_ecs/latest/bevy_ecs/resource/trait.Resource.html), [Entity identity and stability](https://docs.rs/bevy_ecs/latest/bevy_ecs/entity/struct.Entity.html).

If a projection is useful, rebuild it after restart. Apply only committed revisions. Ignore duplicate/older revisions. Detect gaps that need reload. Do not allow a cache hit to bypass ROM's read-consistency, authorization, or expected-revision rules. Ordinary ROM users should never manipulate `World` to perform a mutation. Exposing unrestricted world access would be a trusted runtime-extension escape hatch, not an alternate supported resource API.

## Scheduling without game ticks

`App::set_runner` supplies the outer loop. `App::update` runs an update. `App` is not `Send` or `Sync`. Construct and retain it on its owning thread. Do not move it among async workers. A custom runner must respect plugin readiness, `finish`, `cleanup`, and exit handling; the standard schedule-runner source is a reference for those steps. [App lifecycle](https://docs.rs/bevy_app/latest/bevy_app/struct.App.html), [runner source](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_app/src/schedule_runner.rs).

Proposed service runner:

1. Wait for an incoming command, worker completion, shutdown signal, or the next retry/lease deadline.
2. Drain a bounded batch into internal queues. Run a bounded scheduling turn.
3. Process available completions. Admit eligible work. Dispatch jobs. Deliver transient hints for confirmed outcomes.
4. If ready work remains, continue. Otherwise, wait again. Recheck readiness atomically with waiting so a wake-up cannot be lost.

This is a design, not a built-in Bevy durable-service mode. A safety maintenance deadline can protect against missed external notifications, but must not substitute for a correct wake-up protocol. Main-thread/local tasks need explicit progress while waiting: either exclude them from long-lived service work or arrange wakeups/pumping. `TaskPoolPlugin` normally ticks global main-thread tasks in the `Last` schedule. [Pool ticking implementation](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_app/src/task_pool_plugin.rs).

Keep individual systems short and nonblocking. A system starts bounded asynchronous work and returns. A later turn handles the result. Do not hold ECS borrows or a world lock across database waits. A single endlessly running or blocking system can still stall a scheduling turn.

## Parallel execution does not establish transaction order

Bevy runs non-conflicting systems in parallel. Its scheduler sees declared ECS access, not a database row touched through an adapter or the semantics of a ROM action. Two systems with non-conflicting ECS data can still race on the same durable resource. Keep expected-revision comparison and idempotency enforcement inside the persistence transaction. [Scheduler access conflicts](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/index.html).

Use explicit system sets and dependencies for runtime phases. `before`/`after` can arrange deferred-command application, but ordering constraints across different schedules are ignored; referenced systems/sets also must actually be scheduled. Enable ambiguity diagnostics during development. ECS deferred application is a memory-update boundary, not SQL commit. [Schedule configuration](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/trait.IntoScheduleConfigs.html), [ScheduleBuildSettings](https://docs.rs/bevy_ecs/latest/bevy_ecs/schedule/struct.ScheduleBuildSettings.html).

Recommended internal flow:

- Admission accepts a bounded ROM request envelope, actor context, stable action identity, and expected revision. Queue acceptance is not durable success.
- ROM validates and computes a candidate transition from a versioned snapshot without external side effects. Bevy may execute independent pure jobs in parallel.
- The adapter atomically persists conditional state update, next revision, action receipt, and committed event records. Conflicting or indeterminate outcomes return to ROM explicitly.
- A completion turn publishes the confirmed outcome through ROM, updates any optional projection, and schedules recoverable post-commit work.

The pipeline can overlap different jobs; it does not require a global serialized commit stage. A broad `ResMut` over all runtime state would unnecessarily serialize systems, while splitting data merely to hide actual conflicts would be incorrect. Measure scheduler contention after the boundary is correct. Per-resource history ordering, delivery ordering, and task completion order remain separate contracts.

## Messages and observers are transient mechanisms

Bevy `Messages<M>` retains messages across its last two update calls. Slow readers can miss messages. If updates are disabled, buffers can instead grow indefinitely. Per-reader progress is in memory. Only when their lifetime is controlled, use these messages for internal hints/completions. Never make them the sole record of admitted durable work or committed events. [Message retention](https://docs.rs/bevy_ecs/0.19.1/bevy_ecs/message/struct.Messages.html).

Observer `Event`s trigger reactive systems. `World::trigger` evaluates immediately; `Commands::trigger` defers to a sync point. Ordering among observers of the same event is arbitrary, and nested triggers are recursively evaluated. Therefore ROM reaction order, retry policy, cascade limits, and durable acknowledgement cannot rely on observer order or recursion. [Observer execution semantics](https://docs.rs/bevy_ecs/latest/bevy_ecs/observer/struct.Observer.html).

After commit, an observer may wake a ROM reaction dispatcher. On restart or lost hints, that dispatcher rereads durable work. A reaction computes and submits a ROM action; it cannot mutate committed domain state through `Query<&mut ...>` or `Commands` directly. The core action contract applies regardless of which Bevy system initiated the request.

## Worker ownership and asynchronous I/O

Bevy separates same-turn compute, longer compute, and I/O-oriented task pools; its task documentation promises neither fairness nor execution order. Start with these pools and provide ROM-level admission/fairness budgets. The default pools are global and initialized through `get_or_init`, so the first initializer matters; an embedding host must own pool configuration. Do not claim each ROM instance gets independently reconfigurable global pools. [Bevy task model](https://docs.rs/bevy_tasks/latest/bevy_tasks/), [global pool initialization](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_app/src/task_pool_plugin.rs).

Dropping a Bevy task handle cancels its future; `detach` allows it to continue without that handle, and `cancel().await` waits for it to stop. Keep task handles in a supervisor with job identity and inspect completion/failure. Do not detach durable work casually or let removing an internal entity accidentally cancel a transaction. Cancellation still does not prove that a database commit failed, and a CPU future that does not yield cannot be forcibly interrupted mid-poll. [Task lifetime](https://docs.rs/bevy_tasks/latest/bevy_tasks/struct.Task.html).

`IoTaskPool` is an executor, not proof that a particular client's networking and timer driver exists. Bevy's task backend uses `async-executor`; SQLx documents its own runtime feature requirements. If the chosen database/HTTP client requires Tokio, let the adapter host own a compatible Tokio runtime and return results through bounded completion channels. Do not `block_on` a database request inside a scheduled Bevy system or assume that polling a Tokio future on Bevy creates a Tokio I/O driver. [Bevy task backend](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_tasks/Cargo.toml), [SQLx runtime support](https://docs.rs/sqlx/latest/sqlx/#runtime-support).

ROM defines shutdown: stop admission and claims, request cooperative cancellation outside protected commit work, allow a configured drain interval, record unresolved identities, and recover durable work after restart. Keep completion capacity and wakeups available throughout draining. Bound queued requests, payload bytes, active jobs, database connections, completions, retries, and reaction cascades; a pool's thread limit alone does not bound queued work.

## Concrete specification changes to make

These are recommendations for the specification owner, not silent new product guarantees:

| Specification | Proposed requirement and scenario |
| --- | --- |
| Runtime execution (new capability) | The runtime uses headless Bevy scheduling behind ROM's API. An application creates, reads, and changes a ROM resource without importing Bevy types or accessing `World`. |
| Resource model | Persistent identity and schema are independent of ECS handles/components. Rebuilding the runtime after restart preserves ROM IDs/revisions even if internal handles differ or no projection exists. |
| Action runtime | Only the ROM action pipeline authorizes and commits domain mutations. A Bevy-initiated stale action receives a conflict without overwriting current state. Internal queue acceptance is distinguishable from durable success. |
| Persistence | An optional ECS projection never establishes commit. Crash after database commit but before projection update recovers the committed state/events/receipt without repeating the transition. |
| Event reactivity | Bevy messages/observers are transient signals; durable work is independently discoverable. Missing two scheduling updates or restarting does not lose a pending reaction. |
| Runtime execution | Scheduling is driven by work/deadlines, with bounded admission, completion handling, and shutdown. An idle instance waits; a completed worker wakes it; overload returns an explicit result. |
| Boundary adapters | Required I/O drivers remain adapter/host implementation details. A caller and core domain extension need not select or depend on Tokio to use ROM semantics. |
| Extension contracts | Native runtime extensions are trusted; normal field/action/reaction contracts expose ROM capabilities rather than unrestricted ECS mutation. A reaction's action uses identical validation and authorization to an external action. |

Avoid specifying one entity per resource, a universal frame rate, component-derived database schemas, observer execution order, or event sourcing merely because Bevy is the runtime.

## Validation plan and current blocker

The official 0.19.1 `bevy_ecs` and root manifests declare Rust **1.95.0** and edition 2024. Bevy's general setup guidance tracks the latest stable Rust, so pinning a release and compatible toolchain is necessary. The inspected `rom-dev` environment reported Rust **1.82.0**; no `rustup` or alternate toolchain was found in its normal root/ROM-user toolchain locations. No installation or configuration change was made. This is an identified toolchain incompatibility, not a failed Bevy compilation. [ECS compiler requirement](https://github.com/bevyengine/bevy/blob/v0.19.1/crates/bevy_ecs/Cargo.toml), [root manifest](https://github.com/bevyengine/bevy/blob/v0.19.1/Cargo.toml), [Bevy setup policy](https://bevy.org/learn/quick-start/getting-started/setup/).

After a compatible toolchain is available, run isolated probes before implementing ROM:

1. Compile only the proposed crates/features. Inspect `cargo tree -e features` for unintended renderer/window/reflection dependencies. Run a headless app once. Exit cleanly.
2. Verify explicit schedule dependencies, deferred command visibility, and concurrent execution of non-conflicting jobs. Compare single-thread and multithread results without expecting the same completion order.
3. Demonstrate message expiry after maintenance updates and immediate/deferred observer timing. Assert that a separate durable test queue recovers work despite lost transient hints.
4. Test custom runner idle wait, wakeup races, deadline wakeup, and continued main-thread task progress. Measure idle CPU and command/completion latency.
5. Exercise retained task handles, cancellation, worker failure, bounded queue saturation, and drain shutdown. Verify no commit is cancelled solely by a caller disconnect.
6. Integrate the selected database adapter's actual I/O driver. Run the revision-race, atomicity, uncertain-commit, stale-claim, and recovery tests from [the concurrency experiment matrix](rust-concurrency.md#experiment-matrix).

All six Bevy probe groups remain unrun. Earlier Tokio probes establish only those Tokio behaviors; they provide no evidence that the Bevy composition compiles, performs well, or satisfies ROM's transactional contracts.
