# Implementer assessment

This slice resolves the central feasibility question for a bounded first core: generic resource handling, atomic state/events and authorization-aware filtered live snapshots can live in a reusable transport-free Rust library. The embedding example needs a declaration, ordinary business function, host policy and a few calls. No application repository, controller or broadcaster was required. Eleven executed tests support this narrow conclusion; they are not evidence of production completeness.

## Useful patterns to carry forward

**One mutation boundary is valuable.** Built-in CRUD and custom actions share authoritative state loading, permission, revision, validation, transaction and invalidation. The SQL-failure test shows why this boundary should belong to ROM. Applications should not manually keep events synchronized with repositories.

**The journal and live result solve different problems.** Immutable durable mutation history retains every committed change. A watch channel carries replaceable current query results and handles a slow consumer without an ever-growing per-consumer event queue. Recomputing a current result is a useful conservative first implementation. This is an architectural assessment supported by the tested behavior, not a benchmark against incremental query engines.

**Snapshot registration and authorization are part of query semantics.** Merely offering a channel is insufficient. The framework must own initial handoff, mutation invalidation, policy invalidation, stale-buffer refresh and shutdown. Review exposed a real mistake in the first implementation: delivery checked current policy against a historical buffered row. The fixed implementation recomputes under the authoritative lock when generations differ, and an explicitly forced stale-buffer regression now protects that behavior. This complexity belongs inside ROM, not in every application.

**Keep the resource contract independent of the macro.** The small macro emits ordinary descriptors, and the runtime accepts those descriptors. Separate derive experiments show richer compile-time metadata can target such a contract. This slice does not combine that derive implementation or Bon with the runtime, so a polished hybrid public API remains an integration work item.

## Human developer assessment

| Activity | What feels useful | Work still needed before calling the interface pleasant |
| --- | --- | --- |
| Add a resource | One readable declaration plus registration, with zero storage or subscription plumbing | Generate ordinary typed data and discoverable field handles; richer derive diagnostics |
| Add a custom action | Plain Rust function and explicit declared action name | Typed arguments/state, fewer positional strings; clear side-effect restrictions |
| Subscribe to a live query | One call returns initial/current and changed snapshots; policy invalidation is automatic | Typed field selection, documented consistency modes, result-size limits and query diagnostics |
| Diagnose a failed mutation | Revision errors name resource/action/expected and actual revision; field errors name resource/field; business error preserves context | Consistent action context on every validation error, source chains, structured tracing and stable error taxonomy |
| Embed and test | Fluent setup, default deny and public integration tests; no web framework dependency | Clear ownership/cancellation semantics, failure supervision and reusable test helpers |

The implementation is better than application-specific plumbing **for the stated goal of automatic resource management** because the difficult correctness rules have one owner and one set of probes. A per-kind repository/controller alternative was not built or measured. Fewer lines alone do not establish better ergonomics, and this experiment has not included independent human usability sessions.

## What can now be estimated

Estimate a first core against explicit, narrow contracts rather than an unspecified complete framework. The demonstrated baseline is one process/runtime, one backend adapter, registered resource kinds, synchronous local policy, validated mutations, current-state/event transaction, and conservatively recomputed boolean live queries. There is now executable reference behavior for those boundaries.

The main complexity drivers for a first implementation are:

1. **Typed authoring contract:** shared descriptors, derive/fluent integration, typed action/query handles, nullable/container/custom-field support, compatibility and diagnostics. Keep maintainers' machinery hidden without hiding generated behavior from debugging.
2. **Mutation and persistence semantics:** backend-neutral transaction capability, identity/revision rules, no-op/delete policy, idempotency, schema evolution, errors and cancellation after an operation has been admitted.
3. **Live result contract:** snapshot handoff, invalidation dependencies, policy changes, buffer semantics, memory bounds, pagination and honest single-process versus multi-process guarantees. Broader predicates increase this work substantially.
4. **Host authorization contract:** trusted actor context, action versus read/history permissions, policy replacement/versioning, data-dependent rules and later field-level rules. Authentication providers remain adapters, not this core's demonstrated feature.
5. **Lifecycle and operability:** explicit shutdown, worker-failure propagation, panic containment decisions, tracing, cancellation, connection management and packaging.
6. **Integration evidence:** preserve deterministic failure/race probes while combining the separate authoring and resource-flow experiments. HTTP/Rabbit, durable reactions and provider integrations need separate scope and acceptance cases.

No person-day numbers follow from a successful tiny prototype. The evidence is enough to decompose and estimate a scoped first library, while naming unresolved behavior. A production estimate still requires agreed query breadth, durability/idempotency guarantees, supported backend(s), typed extension contract and operational boundaries. These are scope choices, not reasons to continue indefinitely prototyping unrelated features.

SQLite/JSON made transactional behavior quick to probe. JSON still exposes string field/action names and run-time validation; SQLite plus one mutex serializes all work. Neither should silently become the final storage or type-system decision. The tests demonstrate a useful reusable boundary, not a final implementation to promote unchanged.
