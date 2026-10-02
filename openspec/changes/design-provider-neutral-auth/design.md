# Provider-neutral auth design

## Status and context

Proposed follow-up to `establish-rom`; no runtime exists. The confirmed requirement is integration with diverse identity providers in a backend-first Rust resource library. The design below is a recommendation pending focused interface and provider decisions. [Research and primary sources](../../../docs/research/auth-architecture.md) explain alternatives and dependency candidates. [Capability requirements](specs/provider-neutral-auth/spec.md) are the proposed normative source.

## Goals and boundaries

Credential verification belongs to trusted adapters; the core enforces access. Resources remain the sole domain entity. Principal, actor context, and decision are supporting values. The core does not require an identity-provider client, HTTP server, broker, policy engine, login UI, user database, or WASM runtime.

The owner explicitly confirmed that **User is a Resource**. ROM user records use the same definition, field, action, persistence and observation contracts as other resources; there is no parallel account entity engine. A curated Studio user screen is a specialized view over that resource and its permitted actions. The generic core need not require an account store for every embedded or service-only use, but representing users in ROM uses the Resource model.

External authentication and the User resource are complementary: a configured linking rule maps verified authority-qualified identities to the local user where required. Matching email or caller-supplied user IDs do not establish that link. ActorContext remains trusted invocation data, not a second persisted user model. Provisioning, first-administrator bootstrap, linking and disablement freshness need explicit policies before implementation. Profile access does not imply role or credential administration; credentials are not ordinary discoverable resource fields.

The embedding host owns adapter configuration and policy selection. Native extensions run as trusted application code; Rust type privacy is useful against accidental misuse but does not sandbox the host or its dependencies.

## Proposed interfaces and ownership

Use an immutable actor context carrying an authority-qualified subject, human/service kind, optional client identity, allowlisted trusted attributes, and validity deadline. Keep credentials and provider-specific claim formats in adapters. Trusted host construction is explicit; request deserialization cannot manufacture an authenticated context.

One authorizer decides an operation against that context, the relevant resource snapshot, and typed request metadata. Operations cover execute, read, discovery, subscribe, and event delivery, with requested/written fields where relevant. No policy or a policy error means denial. Begin with local Rust decisions over immutable policy data; Cedar or Casbin can be separate implementations later.

Exact Rust lifetimes, async boundaries, trait object use, and field-path vocabulary remain open. Do not define a universal credential enum: adapter-local verification methods can differ while yielding the same core context.

## Verification and trust sources

Configure each accepted authority and its issuer, audience, access-token profile, endpoints, allowed algorithms, and claim mapping. JWT verification includes signature, issuer, audience, expiry, applicable not-before and token-type checks. Bind fetched keys to trusted configuration; a token cannot select an arbitrary trust source. Introspection requires a positive active response and sufficient verified evidence of identity and intended resource. Define finite evidence lifetimes and bounded cache behavior.

OIDC login is optional application integration. ID tokens are not API access tokens. Unknown providers or principal kinds are denied rather than guessed. An explicit custom adapter can cover a different protocol without weakening the core contract.

For ROM Studio, the owner requested a branded sign-in screen and an optional direct handoff to a configured primary identity provider. The provider's managed configuration is a Resource, using the shared permissions and action path. This presentation/routing preference does not authenticate the visitor or bypass core policy. A visual mock explores primary selection separately from automatic handoff; exact uniqueness scope, trusted public login discovery, callback validation and failure/recovery behavior remain to be specified before implementing login. No provider credentials may be exposed through public discovery.

## Enforcement and transaction semantics

All entrypoints call the same authorization seam, including observers and reactions. Apply action/resource/field checks before committing; authorize the response independently where it discloses data. Evaluate state-dependent policy against the revision committed, with conflict detection protecting that relationship. External policy changes are not automatically transactional with ROM persistence: define policy snapshot version/freshness at the operation boundary before implementation.

Idempotency remains principal-scoped and input-sensitive. Authenticate and authorize every retry before disclosing a stored result. Losing permission does not undo an existing commit or permit a duplicate transition. Store sufficient non-secret authorization metadata for retries after deletion; the exact receipt/result shape remains open.

Reads and event delivery require their own permissions and field projections. Query predicates, counts, ordering, and cursor behavior also affect disclosure. Reject unsupported safe query shapes initially. Recheck stream authority at bounded intervals and delivery boundaries, including expiry. Retained history is internal persistence, not a public feed by default.

## Service identities and attribution

Workers and reactions use configured service principals with current permissions. Human/client identities remain distinct where delegation exists. Original actor IDs and causation in an event explain provenance; they grant no authority. Never replay an original token or reconstruct an actor context from event metadata. User-delegated background work needs its own later contract.

Persist minimal identifiers and policy references for attribution. Credentials and full claims/context are excluded from logs, events, and deduplication records. Protected resource values need separate storage, retention, and event projection policy.

## Risks and unresolved decisions

- Subject collisions, account linking, tenant membership, and role mappings require explicit authority-aware rules.
- Cached JWT keys, introspection results, and policy snapshots each create different freshness windows; select limits and outage behavior deliberately.
- Field authorization and historical event access require a precise vocabulary before exposing rich queries.
- Revoked access and resource deletion complicate cached outcome disclosure; specify safe receipts before implementation.
- Policy engines do not automatically satisfy ROM's failure semantics. In particular, Cedar skips individual policy errors unless the adapter inspects diagnostics.

## Migration and reversion

No existing actors, tokens, or stored outcomes need migration. Future changes to identity mapping or policy semantics will need versioning and explicit migration; silently rebinding an existing subject to a different authority is not a compatible change. Reverting this proposal affects only documentation.
