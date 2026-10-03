# Integrated MVP design

## Accepted premise

Resource is the only managed domain entity. Actions request changes and events
describe committed facts. User, provider configuration and application settings
use the same Resource contract. Derives generate codecs and typed bindings;
registration validates definitions; runtime owns dynamic guarantees. Tokio owns
async execution, Rayon CPU execution. Drivers and wire protocols are adapters.

The owner explicitly selected post-commit reaction chains: failure of a later
step preserves earlier commits. Retry is bounded; compensation is an explicit
action. A chain is execution metadata, not a competing domain entity.

## MVP profile and implementation defaults

These are reversible implementation choices for the requested MVP, not claims
that the owner approved every deployment policy. One runtime owns writes and
invalidation. One SQLite adapter and one redb adapter exercise the same atomic
commit and recovery contract. Direct typed calls are the primary API. An HTTP
extension performs generic descriptor routing; optional middleware must not own
accepted work. Native extension code is trusted application code.

Start with bounded scalar, nullable, collection and custom-codec fields, typed
reference values, equality/conjunction filters and stable identity ordering.
Unsupported query operators, topology and transport profiles fail explicitly.
Field projection returns a projected view, never a partially invalid typed
Resource. Raw complete typed reads require permission for the complete value.
External action inputs use the accepted descriptor and codecs, not independent
Serde field-name rules.

## Transaction and ownership boundaries

The persistence contract commits state/revision, action identity/fingerprint,
safe outcome, events and effect intentions together. Adapter rollback leaves
none of them. Response loss can leave the caller uncertain. Same-key resolution
uses the durable receipt under current authorization. Cache eviction never
means permission to execute again.

An accepted operation owns its permit until execution finishes, independently
of the observing caller. Admission and shutdown closure have one serialization
point. Reads, durable I/O, notifications, and streams have explicit count/byte
limits. Synchronous drivers run outside Tokio workers. A live handle retains
an invalidation marker and recomputes an authorized bounded result on demand.
Journal delivery is separate and reports gaps instead of coalescing facts.

Reaction registrations add durable intentions to the committing operation.
The worker invokes the same action path using a stable derived idempotency key
and an explicitly granted service identity. Restart recovers pending work.
Acknowledgment loss repeats observation, not the committed mutation. No-op
suppression and declared dependencies avoid unnecessary work. Causal depth,
total work and retry budgets terminate oscillation/storms visibly; a global
visited-resource set is not assumed correct for converging revisits.

## Other adapters

Configuration loaders submit values for registered Resources under host-granted
source authority; source files cannot define new types or grant themselves
permission. Preserve ownership/provenance and failed-reload state. Bootstrap
requires host provisioning. Configuration persistence and external activation
are distinct: activation failure remains visible and cannot pretend to roll
back an already committed Resource.

Blob storage and notification channels use core-owned contracts. Folder and
S3-compatible implementations remain separate. Notification intention commits
atomically with its originating action; delivery is at least once unless the
receiver implements idempotency. Authentication adapters produce verified
identity claims, which are explicitly linked to a local User Resource. Policy
is evaluated by core before execution and disclosure.

## Security and operational boundaries

Default deny; no email-based identity auto-link. Untrusted transports cannot
construct trusted actors from request fields. Expiry, disablement, row access
and field visibility apply to cached results and buffered deliveries too.
Provider credentials are host secret references, never configuration values.
Public descriptors, events, logs and receipt responses must omit protected data
according to current policy. Trusted persistence and private backups contain full
application values and protected metadata; core does not claim encryption at rest
or physical erasure. The host protects these stores. Configuration of a provider
is not its credential.

Retain receipts conservatively in this MVP with a finite configured budget.
At the limit, refuse new obligations. Do not silently purge identities then
accept old retries. Journals have an explicit cursor generation and retention
floor. Backup includes pending work and receipts; external blob completeness
must be separately checked. Restore changes history generation when needed.

## Deferred profiles

Multiple independent writers require tested lease/fencing and invalidation
protocols. Advanced joins/aggregates require explicit adapter capabilities.
WASM, runtime schema authoring, distributed exactly-once external effects and
production Svelte Studio are not MVP claims. Research remains indexed with a
recommendation and a reason for deferral. Numeric operational limits are host
policy with documented conservative defaults, not universal performance claims.
