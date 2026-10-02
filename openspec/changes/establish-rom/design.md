# ROM architecture

## Context and agreed direction

ROM is Resource Oriented Meta Framework. The first product is a backend library, not a hosted service or UI framework. Resource is the sole domain entity; action and event are messages, not competing domain entity abstractions. A resource kind defines its fields and allowed behavior.

One main library owns the public contract. Database and transport integrations are separate implementations behind focused extension interfaces. Rust extensions are compiled with the host initially. WASM is deferred.

## Defining premise: declare once

A resource declaration is the single application-facing definition for standard operations, persistence, query metadata, live reads and committed-change subscriptions. With a generic HTTP adapter enabled, it also supplies endpoints and live streaming without per-kind handlers. Adapters are selected once by the host, not reimplemented for each resource kind. Custom actions add domain behavior; they do not force reimplementation of ordinary resource CRUD.

Avoid mandatory per-kind storage schemas and repositories. A generic persisted envelope with typed payload validation is a candidate, with physical layout and indexing still to be decided. Changes to declared fields may need value compatibility rules, but do not imply handwritten endpoints or a new database table per kind.

## Human developer experience

Human usability is a first-class design criterion. Resource declarations should be readable, standard operations discoverable, and custom business behavior expressible in ordinary Rust. Reducing line count is useful only when the result remains understandable. Generated behavior must be inspectable; users should be able to identify the responsible resource, action and policy without tracing opaque macro expansion or incidental hook ordering.

ROM deliberately absorbs difficult implementation work. Complex code generation, typed helpers and execution machinery are justified when they give application authors a simple, reliable interface. Evaluate complexity at the application interface separately from implementation effort inside ROM. Before rejecting a useful typed design for awkward helper signatures, investigate whether framework-owned helpers can hide those signatures. Application developers should not have to reconstruct the framework's internal machinery to define and manage ordinary resources.

Provide a small documented path from resource declaration to persisted operations and live observation. Routine application code must not assemble executor pools, capacity permits, event cursors or publication machinery for each resource. Keep advanced controls available through explicit host configuration. Errors should identify the failed operation and a safe corrective next step, retaining structured details for code and avoiding disclosure of protected values.

Before stabilizing the public API, review a human-facing walkthrough: add a resource, add a custom action, observe a filtered live query, diagnose a rejected mutation, and test the behavior without an HTTP server. Record confusing steps and required infrastructure knowledge; a short declaration alone is not evidence of a pleasant API.

The owner's preferred authoring direction is a hybrid: a resource derive for structural bindings and a fluent Rust interface for composition. Both must feed the same resource contract. Generated helpers may provide typed fields, codecs and action bindings; they must not create separate implementations of authorization or transaction semantics for each kind. Statically knowable checks should become compiler checks where useful, while registration checks the composed application and runtime execution checks actual values, permissions and state. Exact syntax and helper crates remain subject to the [authoring research and crate trials](../../../docs/research/rust-resource-authoring.md).

## Registration and generated-contract integrity

Derived and manual definitions enter one registration gate. It checks unique identities, complete field/action bindings, codec consistency and required adapter capabilities, then freezes accepted definitions. A validated registry establishes these configuration invariants; it does not certify future input values, permissions or database revisions. Preserve resource/field/action provenance through generated bindings, registration and runtime diagnostics, with safe public errors distinct from internal invariant failures.

ROM's descriptor governs its public field names, presence, nullability and defaults. Protocol adapters use codecs consistent with that contract. Independent Serde derives may serve application purposes, but do not redefine ROM's protocol. A future direct-Serde codec mode needs explicit supported settings and rejects incompatible ones. Conformance tests must exercise actual encoding, decoding and validation, including manual extensions; metadata equality alone is insufficient.

These decisions apply [Beskid's compiler and Rust lessons](../../../docs/research/beskid-compiler-lessons.md) to ROM without importing compiler-specific infrastructure. Keep a capability matrix with explicit negative tests and source-local diagnostics. Public authoring remains ordinary Rust; internal phase machinery belongs inside ROM.

## Runtime direction: Tokio and Rayon

Selected by the project owner: Tokio handles asynchronous execution, networking and database I/O; Rayon handles substantial CPU-bound work. ROM owns domain types, action semantics, authorization, validation, persistence and durable events. Applications use ROM interfaces rather than executor task identities as domain identities.

The embedding host owns a long-lived Tokio runtime and a bounded Rayon pool, or explicitly delegates their construction to ROM. Do not construct a new runtime or thread pool per action or resource. Expose lifecycle and capacity configuration through a small execution interface. Exact versions, feature flags and minimum Rust version must be pinned after a compatible-stack probe.

Use bounded admission before spawning async tasks or submitting CPU jobs. Limits cover pending jobs and retained payloads as well as running work. Await CPU completion asynchronously through a result channel; do not block a Tokio worker waiting for Rayon. CPU work receives owned inputs and returns a proposed result, leaving persistence and domain authorization in the action pipeline. Small field checks remain synchronous when offloading would add overhead.

Cancellation is stage-aware. A started CPU task may continue after its caller stops waiting, so its capacity permit remains held until actual completion. A cancelled database commit may have an uncertain outcome: use idempotency and durable outcome lookup rather than assuming rollback. Shutdown stops intake, signals cooperative cancellation and supervises remaining work under a documented deadline. Neither executor supplies durable transaction or event-delivery semantics automatically.

Preserve explicit stages for intake, validation, durable commit, publication and reactions. Independent resources may progress concurrently; state-dependent decisions are checked against the revision committed. See [concurrency research](../../../docs/research/rust-concurrency.md) for evidence and proposed acceptance probes.

## Goals / Non-Goals

Goals: reusable resource behavior, reliable mutation, recoverable reactions, database persistence, built-in field types and custom Rust field types.

Non-goals: frontend generation, vendor device protocols, a broker requirement, runtime-loaded native plugins, WASM execution, distributed multi-resource transactions, or full legacy compatibility in the first release.

## Decisions

### One authoritative mutation pipeline

Authenticate at the boundary; carry a trusted actor context into the core. The core evaluates authorization, validates a typed action, checks resource revision, computes the transition, and commits state plus events atomically. Observers and reactions use this same path. A rejected action publishes no success event.

Validation and transition computation must not perform external side effects. Effects occur after commit through recoverable work. Actions and events carry schema versions and causal references. Logs and diagnostics must avoid raw protected values.

### Fields and extension contracts

Use explicit Rust registration for extensions. ROM defines focused contracts for field types, persistence adapters and transports; registered implementations use the shared action and execution interfaces. No general-purpose dynamic plugin loader is required initially.

Use separate contracts for field types, persistence and transport integrations. A field type supplies stable type identity and version, validation, canonical encoding/decoding, and declared capabilities such as comparison. Database adapters explicitly advertise support; unsupported querying fails rather than silently behaving differently.

Proposed initial families: boolean; signed/unsigned integers with explicit ranges; finite floating point; exact decimal; text; bytes; identifier; timestamp; date; duration; enum; optional; list; map; structured object; resource reference. Widths, precision, temporal rules, recursion limits and wire encodings require focused follow-up specifications before implementation. Missing, null and explicit default values are distinct at the action boundary.

Rust extensions are trusted application code; a trait does not sandbox them. WASM can later implement selected contracts through a bridge without forcing a WASM ABI into the initial public API.

### Provider-independent infrastructure

The owner requires generic contracts across infrastructure: persistence implementations, folder/S3 blob stores and named notification channels. Core owns semantic operations and guarantees; adapters own provider protocols, physical layout and driver types. Resource definitions do not select engines. The [capability-adapter design](../design-capability-adapters/design.md) specifies this separation and resilience boundaries; its experiments include SQL/key-value substitution, actual S3-compatible interoperability and durable function-channel delivery. These are separate proofs, not an integrated production runtime.

### Persistence and reactions

The core owns the transaction semantics; a database adapter owns storage mechanics. Resource revisions provide optimistic concurrency. Reaction execution is at least once, with durable progress and stable deduplication identities. There is no claim of exactly-once remote effects.

Proposed starting point: current state plus durable event journal and pending-work records in one transaction. This is simpler than making historical events the exclusive state authority. Full event sourcing remains an alternative requiring explicit decisions about schema evolution, replay and protected historical data. Select a reference database after research; PostgreSQL is a candidate, not a committed dependency.

Per-resource event ordering is required; global ordering is not. Consumers need bounded buffering, retry limits and an inspectable failed-work state. Cross-resource reactions must carry causal context and be bounded to prevent infinite feedback loops. Exact scheduling and replay contracts need a follow-up spec.

### Public interface shape

Expose operations to register resource kinds/field types, execute actions, read resources, observe live reads, and subscribe to committed changes. Concrete Rust signatures will follow the research and a minimal vertical slice. HTTP and RabbitMQ adapt this interface; neither becomes part of the core dependency graph.

The owner approved a core-owned transport layer expressed through capabilities, with separate invocation, live-observation and journal-subscription contracts. Resource remains the application domain entity; invocation and subscription values are protocol machinery. Bindings declare supported operation families, codecs, limits and recovery semantics. Validate required combinations at registration and reject unsupported calls before effects. Capability support describes technical ability, not caller authority.

Feathers supplies the reference pattern of equivalent local and transported operations. Tower's generic Service/Layer composition is a candidate implementation mechanism, not an adopted core dependency or a substitute for ROM's lifecycle and durability rules. A focused trial must check readiness, cancellation, admission ownership and stream lifetime before adoption. Do not force all operations into one universal request/response object. See the [transport review](../../../docs/research/transport-layer-review.md), [Feathers services](https://feathersjs.com/api/services) and [Tower Service](https://docs.rs/tower/latest/tower/trait.Service.html).

Keep trusted identity context separate from serialized input. HTTP status codes and broker acknowledgments map to core outcomes; they do not define commit status. Live results can refresh/coalesce, whereas journal delivery requires explicit history cursors and gap handling. A binding can support only a subset, but each supported operation preserves the shared semantics. Durable queued acceptance requires a persisted recovery obligation and is not implied by transport receipt.

### Reactive reads and durable facts

Live reads answer what the current authorized result is; committed-event streams answer what happened; reactions request subsequent actions. They share the resource model but require distinct delivery semantics. A live query may coalesce intermediate states, whereas a durable consumer resumes through an explicit cursor and retention contract. A transient notification is a wake-up hint, not the authoritative history.

Generate live forms of standard resource queries without per-kind subscription code. The initial implementation may invalidate all dependent reads of a kind and rerun them; precise dependency tracking is an optimization. Changes must account for filter membership, deletion, ordering and supported pagination. Subscription startup needs a consistent snapshot/change boundary or explicit refresh to close the setup race. Custom reads require declared or tracked dependencies; arbitrary external state cannot be automatically observed without an integration contract.

Authorization and field projection apply to initial results and subsequent delivery. Policy changes must be reflected under an explicit freshness rule. Bounded buffering and a defined resync path are required for slow consumers. These are intended contracts: event replay in a prototype does not establish live-query correctness. See the [research comparison](../../../docs/research/state-of-art-resource-frameworks.md) and [prototype findings](../../../docs/research/prototype-results.md).

## Risks / Trade-offs

A universal resource can become an untyped property bag: retain typed kind validation and versioned field contracts. Too many abstractions can obscure simple applications: require concrete conformance tests and an actual implementation for each supported contract. Custom field capabilities may differ by database: reject unsupported capabilities at setup. Reactions can form cycles: preserve causality and impose explicit budgets.

## Migration and rollback

No existing ROM data exists. Future schema changes require versioned decoding and explicit migrations; rolling back code must not silently reinterpret newer data. Native extension compatibility is source-level compatibility under Rust compilation, not a stable binary ABI.

## Open Questions

- Concrete Rust signatures and ownership/async model.
- Reference database and transaction implementation.
- Exact built-in field encodings and capability vocabulary.
- Reaction cursor, retry, retention and cycle-budget contracts.
- Whether desired/observed sections are universal or an optional resource convention.
- Protected-field policy and encryption adapter design.

These questions are intentionally unresolved; initial setup does not authorize silently choosing them as implemented behavior.
