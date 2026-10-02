# Resource frameworks and Rust building blocks

Research date: 2026-10-02. Status: design input, not an approved dependency list or implementation specification. Sources are public first-party documentation. Recommendations below are architectural judgments, distinguished from documented upstream behavior. No performance benchmarks, compatibility build, or dependency-license audit were performed.

## Recommendation

Build ROM as a small resource engine with its own contracts. Borrow Ash's resource/action/type organization, Kubernetes' reconciliation discipline, and CQRS libraries' separation of intent from committed facts. Start with persisted current state plus a transactional event outbox; do not require full event sourcing. Use compiled Rust extensions first. HTTP, RabbitMQ, and a future WASM host belong outside the core.

The single domain entity is the **resource**. Actions, events, subscriptions, and persistence records remain necessary supporting concepts; they do not need to become separate domain entities or resources themselves.

## Closest design references

| Reference | Verified behavior | Suggested use in ROM | Boundary |
| --- | --- | --- | --- |
| Ash Framework | Actions provide resource operations; transaction handling collects notifications. Custom types define input casting, stored-value loading, storage representation, and constraints. | Closest conceptual reference: let a resource definition describe fields and actions, with separate extension contracts. | Borrow the concepts, not Elixir macros or the full framework surface. A post-commit notification alone is not a durable delivery guarantee. |
| Kubernetes / kube-rs | A controller watches a resource and related objects, schedules reconciliation, and reschedules according to its result. | Reactions can reconcile current state rather than assume each wake-up describes a unique business occurrence. | A Kubernetes client is not a generic ROM runtime. Reconciliation and processing every business event are distinct contracts. |
| cqrs-es | Aggregates associate commands, events, errors, and services. Its event-store model uses ordered events and optimistic concurrency. | Study action validation, event envelopes, resource version checks, and adapter boundaries. | Adopting an event-sourcing framework commits ROM to additional history/replay semantics. Consider as an optional persistence strategy only after that requirement exists. |

Sources: [Ash API and transaction notifications](https://ash.hexdocs.pm/Ash.html), [Ash custom types](https://ash.hexdocs.pm/Ash.Type.html), [kube-rs reconciler](https://kube.rs/controllers/reconciler/), [cqrs-es API](https://docs.rs/cqrs-es/latest/cqrs_es/), [cqrs-es aggregate design](https://doc.rust-cqrs.org/intro_add_aggregate.html), [cqrs-es event-store guarantees](https://doc.rust-cqrs.org/demo_event_store.html).

The CQRS tutorial includes older code examples; use the current crate API for any implementation. None of these sources establishes that ROM needs every upstream feature.

## Contracts worth specifying before choosing dependencies

These are proposed ROM design decisions, not claims about upstream libraries.

1. **One mutation boundary.** Every persisted change passes through an action execution path, including changes initiated by reactions or adapters. Authorization, field validation, resource invariants, and expected-version checks precede commit.
2. **Explicit patch semantics.** Missing, null, false, zero, empty, and remove must not collapse into one value. Action arguments may be strongly typed; transport decoding must preserve this distinction.
3. **Atomic state and event persistence.** A successful mutation persists its new version and event records in one transaction. Publishing happens afterward. An adapter unable to provide that contract must report unsupported capability rather than weaken the guarantee silently.
4. **Optimistic concurrency per resource.** Compare the expected version and increment the version atomically. Reject stale actions or rerun only a documented retry-safe action. Avoid a global process lock as a distributed correctness mechanism.
5. **Durable reactions are separate from live observation.** Durable consumers need acknowledgement/checkpoint state, retries, and recovery after restart. Live subscribers may instead receive change hints and reload the latest resource. Make the distinction visible in the API.
6. **No implicit exactly-once promise.** Retrying an action or a delivery requires stable identity and deduplication. A publisher may crash after broker acceptance but before marking an outbox record delivered. External effects need idempotency support or explicit compensation semantics.
7. **No assumed global event order.** Resource versions provide local ordering. Parallel outbox publishers can reorder messages; preserve per-resource order where required. A database sequence allocated before commit is not automatically a safe committed-event cursor: a later transaction may commit first.
8. **Bound reaction cascades.** Specify causation metadata, retry limits, concurrency limits, cancellation, and what happens to permanently failing work. A reaction producing an identical state should not accidentally create an infinite change loop.

SQLx supplies explicit transactions; it does not supply these domain guarantees automatically. RabbitMQ distinguishes publisher confirms from consumer acknowledgements; neither covers ROM's database transaction. Tokio broadcast explicitly drops old values when receivers lag. These facts motivate the proposed boundaries. [SQLx transactions](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html), [RabbitMQ acknowledgements and confirms](https://www.rabbitmq.com/docs/confirms), [Tokio broadcast](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html).

## Smallest useful crate stack

The placement below is a recommendation, not a prescribed workspace layout. One public ROM library can expose a cohesive API while optional adapters remain separate crates.

| Layer | Candidate | Documented capability and proposed use | Tradeoff |
| --- | --- | --- | --- |
| Core values | [serde](https://docs.rs/serde/latest/serde/) | Serialization/deserialization traits for persisted and transported values. | Serialization alone does not define schema evolution or domain validation. |
| Boundary representation | [serde_json](https://docs.rs/serde_json/latest/serde_json/) | Typed JSON conversion and a dynamic value representation. | Useful at boundaries; an unvalidated JSON map should not become the resource model. Specify decimal, binary, and timestamp encoding explicitly. |
| Public errors | [thiserror](https://docs.rs/thiserror/latest/thiserror/) | Derive standard Rust error implementations. | Keep machine-readable domain error variants; do not expose only strings. |
| Identifiers | [uuid](https://docs.rs/uuid/latest/uuid/) | UUID construction and parsing. | Wrap identifiers in ROM types; an identifier is not an ordering or concurrency token. |
| Async execution | [tokio](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html) | Async synchronization, including bounded broadcast. | Prefer an explicit host-owned runtime. Broadcast can wake a durable worker; it cannot replace durable storage. |
| Persistence adapter | [sqlx](https://docs.rs/sqlx/latest/sqlx/) | Async SQL access and database-specific drivers. | Start with one database, recommended PostgreSQL for the first production adapter. SQLite is a separate capability/test target, not evidence that SQL semantics are portable. |
| Diagnostics | [tracing](https://docs.rs/tracing/latest/tracing/) | Structured spans and events. | Instrument actions, commits, and reactions while excluding field values by default. Diagnostics are not domain event persistence. |
| Contract testing | [proptest](https://docs.rs/proptest/latest/proptest/) | Property-based testing with shrinking. | Useful for codec round trips and patch semantics; explicit integration tests still need to exercise transaction failures and concurrent mutations. |

Start with these capabilities only as their first concrete use appears. Do not add a broker, actor framework, ORM, schema generator, or plugin loader to make the initial engine look complete.

## Rust field extensions

Prefer explicit registration through a builder and small Rust traits. A field-type implementation should provide a stable logical type name and version, accepted input, normalized representation, validation, storage encoding/decoding, and declared comparison/query capabilities. For example, a custom geographic value may support equality without supporting portable ordering. Unsupported database operations should fail explicitly.

Ash offers a useful precedent for separating input casting, loading stored data, and dumping storage values, including derived constrained types. ROM should define its own smaller contract rather than copy every callback. [Ash.Type](https://ash.hexdocs.pm/Ash.Type.html).

Proposed implementation choices:

- Keep Rust concrete types where possible; introduce type erasure only at the registry boundary. Persist logical type identifiers, not Rust implementation names or memory representations.
- Use explicit registration initially. It makes duplicate types, version conflicts, and enabled capabilities inspectable at startup.
- Consider [async-trait](https://docs.rs/async-trait/latest/async_trait/) only if dynamically dispatched async extension methods require it; its documented approach uses boxed futures. Synchronous field validation need not become async.
- [typetag](https://docs.rs/typetag/latest/typetag/) supports Serde serialization of trait objects, but that does not establish ROM's migration or stable storage contract. It is optional, not the starting plugin architecture.
- [schemars](https://docs.rs/schemars/latest/schemars/) generates JSON Schema from Rust types. Add it for schema export when needed; it does not replace action invariants or a runtime custom-field registry.
- [rust_decimal](https://docs.rs/rust_decimal/latest/rust_decimal/) and [time](https://docs.rs/time/latest/time/) are candidates for decimal and temporal built-ins. Their value representations still require a deliberate persistence and wire-format policy.

Separate a type's mathematical/domain meaning from confidentiality policy. A sensitive string remains a string with restricted handling; adding a plugin must not grant it unrestricted access to secrets, storage, or publication. Native Rust plugins run in-process and are trusted code, not sandboxed components.

## Adapters and future WASM

**HTTP:** [axum](https://docs.rs/axum/latest/axum/) is a suitable later adapter because it provides routing and extraction and integrates with Tower middleware. It should translate requests to the same action API used in-process. It must not own validation or persistence semantics.

**RabbitMQ:** [lapin](https://docs.rs/lapin/latest/lapin/) is an AMQP client candidate. The adapter should handle publisher confirms, manual consumer acknowledgement, routing, reconnects, and mapping delivery identifiers to durable ROM identity. Confirm support alone does not make an outbox atomic with RabbitMQ. [RabbitMQ confirmation semantics](https://www.rabbitmq.com/docs/confirms).

**WASM:** defer runtime selection. The Component Model's [WIT interface language](https://component-model.bytecodealliance.org/design/wit.html) describes typed interfaces and records, variants, and resources. Its technical term “resource” is not ROM's domain entity. A future WIT contract can wrap selected Rust extension capabilities, but Rust traits are not automatically a cross-language ABI. Before adoption, define host capabilities, size/fuel/time limits, deterministic behavior requirements, and error translation. Do not promise that database or transport adapters will run unchanged in WASM.

## Decisions still open

- Which database is first, and what transactions/query capabilities must every conforming adapter support?
- Are resource schemas compiled Rust definitions only, or can applications assemble definitions at startup? Runtime-loaded executable plugins remain deferred either way.
- Which reactions are durable event consumers and which merely reconcile the latest state?
- What ordering, retention, resume, and consumer checkpoint guarantees are public?
- Which field types are required in the first milestone, and which comparisons are portable?
- Are multi-resource atomic actions required, or is one resource the transaction boundary initially?
- Does history need to reconstruct every past state, or is current state plus retained events sufficient? The latter is the simpler initial recommendation.

Before dependency adoption, select compatible releases, verify the Rust minimum version and feature flags, inspect licenses and maintenance, and run an actual build. This note verifies documented capabilities, not a tested combination of crate versions.
