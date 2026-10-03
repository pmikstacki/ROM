# Actor alternatives: preliminary, not selected

Research date: 2026-10-02. The user selected **Tokio + Rayon** during this investigation. Actor research stopped at that point. This note preserves verified distinctions. It is not a completed selection study or a recommendation to add an actor dependency. No actor library was installed, compiled or benchmarked. No proprietary source was used for this comparison.

## Verified findings

| Alternative | Verified behavior | What remains open |
| --- | --- | --- |
| Ractor | Separates actor configuration from mutable state. Handlers on one actor do not execute concurrently. Its documented local user-message ordering is FIFO per sender; cross-sender ordering is unspecified. Supervision reports lifecycle events and leaves restart policy to the supervisor. | Admission bounds need verification against the exact spawning/queue API; general documentation refers to factory/configuration policies. No ROM integration or recovery behavior tested. |
| Kameo | Actors execute asynchronously in their own tasks. Bounded and unbounded mailboxes are available. Spawn documentation describes a default bounded mailbox of 64 and configurable supervision/restart construction. | Cancellation, restart delivery semantics, dependency/MSRV compatibility and operational behavior were not fully investigated. |
| Actix actor crate | Initial discovery identified the separate actor crate, rather than Actix Web. | Detailed API, maintenance and embedding comparison was not completed before the selection changed. |

Sources: [Ractor runtime semantics](https://github.com/slawlor/ractor/blob/main/docs/runtime-semantics.md), [Ractor supervision](https://docs.rs/ractor/0.16.5/ractor/actor/index.html), [Kameo actor module](https://docs.rs/kameo/0.22.2/kameo/actor/index.html), [Kameo mailbox module](https://docs.rs/kameo/0.22.2/kameo/mailbox/index.html), [Kameo spawning and supervision](https://docs.rs/kameo/0.22.2/kameo/actor/trait.Spawn.html), [Actix actor crate](https://docs.rs/actix/0.13.5/actix/).

Ractor distinguishes graceful stop from kill. Graceful stop allows an executing handler to finish. Kill can interrupt the handler. This matters for transactions: actor lifecycle cancellation is not a confirmed rollback. Ractor also documents that remote delivery/reconnection may lose messages without application-level acknowledgement and retry. These are actor-runtime properties, not durable delivery guarantees. [Ractor runtime semantics](https://github.com/slawlor/ractor/blob/main/docs/runtime-semantics.md).

## Implications for ROM

The following are architectural judgments derived from ROM's requirements, not measured library comparisons:

- An actor can own a worker, adapter connection, partition queue or reconciliation process. Resource remains the sole domain entity. It is unnecessary to map every resource to a permanently resident actor.
- Serial processing inside one actor can reduce local contention. It cannot establish exclusive ownership across independent processes or make a database update and event publication atomic.
- The database transaction remains authoritative for validated state changes, revisions, action outcomes and durable events. Reactions recover from persisted progress. A volatile mailbox can carry a wake-up or durable work identifier.
- A timeout or lost reply can leave an indeterminate action outcome. Scoped idempotency and outcome lookup belong in ROM's action contract regardless of scheduler choice.
- Plain Rust domain and extension contracts can stay independent of actor references, HTTP and RabbitMQ. Tokio tasks and bounded admission can host those contracts directly. Rayon can handle explicitly admitted CPU work where measurement justifies it.

There is insufficient completed comparative evidence here to rank the actor candidates. The decision to proceed with Tokio + Rayon is the user's selection, not a claim that this partial research proved actors unsuitable.
