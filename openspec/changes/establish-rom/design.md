# ROM architecture

## Context and agreed direction

ROM is Resource Oriented Meta Framework. The first product is a backend library, not a hosted service or UI framework. Resource is the sole domain entity; action and event are messages, not competing domain entity abstractions. A resource kind defines its fields and allowed behavior.

One main library owns the public contract. Database and transport integrations are separate implementations behind focused extension interfaces. Rust extensions are compiled with the host initially. WASM is deferred.

## Goals / Non-Goals

Goals: reusable resource behavior, reliable mutation, recoverable reactions, database persistence, built-in field types and custom Rust field types.

Non-goals: frontend generation, vendor device protocols, a broker requirement, runtime-loaded native plugins, WASM execution, distributed multi-resource transactions, or full legacy compatibility in the first release.

## Decisions

### One authoritative mutation pipeline

Authenticate at the boundary; carry a trusted actor context into the core. The core evaluates authorization, validates a typed action, checks resource revision, computes the transition, and commits state plus events atomically. Observers and reactions use this same path. A rejected action publishes no success event.

Validation and transition computation must not perform external side effects. Effects occur after commit through recoverable work. Actions and events carry schema versions and causal references. Logs and diagnostics must avoid raw protected values.

### Fields and extension contracts

Use separate contracts for field types, persistence and transport integrations. A field type supplies stable type identity and version, validation, canonical encoding/decoding, and declared capabilities such as comparison. Database adapters explicitly advertise support; unsupported querying fails rather than silently behaving differently.

Proposed initial families: boolean; signed/unsigned integers with explicit ranges; finite floating point; exact decimal; text; bytes; identifier; timestamp; date; duration; enum; optional; list; map; structured object; resource reference. Widths, precision, temporal rules, recursion limits and wire encodings require focused follow-up specifications before implementation. Missing, null and explicit default values are distinct at the action boundary.

Rust extensions are trusted application code; a trait does not sandbox them. WASM can later implement selected contracts through a bridge without forcing a WASM ABI into the initial public API.

### Persistence and reactions

The core owns the transaction semantics; a database adapter owns storage mechanics. Resource revisions provide optimistic concurrency. Reaction execution is at least once, with durable progress and stable deduplication identities. There is no claim of exactly-once remote effects.

Proposed starting point: current state plus durable event journal and pending-work records in one transaction. This is simpler than making historical events the exclusive state authority. Full event sourcing remains an alternative requiring explicit decisions about schema evolution, replay and protected historical data. Select a reference database after research; PostgreSQL is a candidate, not a committed dependency.

Per-resource event ordering is required; global ordering is not. Consumers need bounded buffering, retry limits and an inspectable failed-work state. Cross-resource reactions must carry causal context and be bounded to prevent infinite feedback loops. Exact scheduling and replay contracts need a follow-up spec.

### Public interface shape

Expose operations to register resource kinds/field types, execute actions, read resources, and subscribe to committed changes. Concrete Rust signatures will follow the research and a minimal vertical slice. HTTP and RabbitMQ adapt this interface; neither becomes part of the core dependency graph.

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
