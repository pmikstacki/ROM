# Configuration through the Resource contract

Date: 2026-10-02. Architectural research proposal; ROM's production core and these lifecycle contracts are not implemented. All managed entities remain Resources, including identity-provider configuration and application settings. No new entity hierarchy or final Rust interface is proposed.

## One model for managed configuration

Provide built-in Resource kinds for useful configuration, declared through the same Rust macro or manual/plugin contract as application Resources. An `IdentityProvider` Resource can describe an accepted authority, verification profile and secret references. Application settings can be another ordinary Resource kind. Their actions use the same validation, expected revisions, authorization, durable persistence, idempotency, journal and live reads. Studio discovers and edits their authorized descriptors without a separate configuration database/API model. [Resource requirements](../../openspec/changes/establish-rom/specs/resource-model/spec.md), [Studio client contract](rom-studio-client-contract.md).

A configuration Resource is the managed description of an integration. Executable adapter code implements the integration. The remote identity service, database or notification service exists outside ROM. Selecting a registered implementation does not upload executable code or create a remote system. Distinguishing these roles does not create competing ROM domain entities. [Capability-adapter design](../../openspec/changes/design-capability-adapters/design.md).

External files and providers supply configuration documents as desired input. A resolver produces a validated effective snapshot for execution. Source documents, candidate snapshots, provenance and activation receipts are supporting values, not another managed-entity model. If a source itself needs managed configuration, represent that configuration as a Resource too.

Keep three facts distinguishable: requested configuration, committed Resource revision and active runtime generation. They need not coincide. Their representation can use ordinary resource fields, action results, protected diagnostics and runtime views; this proposal does not mandate universal `spec/status` or desired/observed sections on every Resource. [Architecture](../../openspec/changes/establish-rom/design.md).

## Source ownership and hierarchical precedence

Priority resolves competing authorized contributions. It never grants authority. The host selects the hierarchy, namespaces and field ownership. One possible profile is built-in defaults, deployment defaults, scoped configuration Resources and explicit operator overrides, from lower to higher priority. The exact ordering remains a design decision informed by the separate Beskid/source-provider research; do not silently treat environment variables or any file as universally highest priority.

Before you choose a loader, define deterministic merge semantics. Absence means no contribution. Explicit null follows the field's contract. Deletion/tombstones are distinct. Unless a declared identity-based merge exists, lists replace. Incompatible types and unknown fields fail.

Reordering irrelevant files must not change the result. Resource IDs must come from stable declared identities, not array positions or content that changes on edit.

| Ownership profile | Studio behavior and reconciliation |
| --- | --- |
| Resource-owned value with external defaults | Studio commits an ordinary action. External refresh can update defaults but cannot erase the higher-priority stored value. Removing an override is an explicit action that reveals the next source. |
| Externally owned Resource/field | Show the effective value and permitted source provenance; make direct mutation unavailable or return an ownership conflict. Edit the source through an explicit supported writeback operation, if any. |
| Explicit overlay | A Studio action writes to a named authorized overlay; show that it shadows another contribution and what removing it would reveal. Never claim to have changed an underlying file. |

Ownership can be field-level when explicitly supported. If an unchanged external winner will immediately restore a different value, do not acknowledge a Studio edit as effective. External reconciliation submits actions as a configured service principal with the same authorization and conflict checks. File parsing never writes storage directly. Expected source revision/digest and Resource revision detect concurrent file/Studio edits. Missing/unavailable source is an error under its declared policy, not automatic permission to delete its Resources or fall back to weaker settings.

For each effective field, retain safe provenance: source identity/revision, priority, ownership, contributing Resource revision, winning versus shadowed contribution and resolution generation. Expose values and source locations only when authorized; provenance can itself reveal sensitive paths or infrastructure. Content fingerprints must exclude secrets or use an appropriate protected scheme, since publishing hashes of low-entropy secrets can leak them.

## Bootstrap and protected trust

Bootstrapping requires a small trusted host seed before persisted Resources and normal authentication are usable: which compiled implementations are available, how to reach initial storage/secret resolution, and the initial authority for loading and administering configuration. This seed is an execution prerequisite, not an alternative persistent managed-entity hierarchy. Once core storage is available, establish built-in configuration Resources through the same validated action path using a narrowly scoped bootstrap principal; retry safely and define when bootstrap authority ends.

An empty installation must not let the first unauthenticated network caller claim administration. Recovery needs an explicit host-controlled path when authentication/storage settings are broken, with auditable ordinary Resource actions once the core is usable. Keep the seed minimal and explain any immutable host constraints in Studio rather than advertising unchangeable settings as editable.

Changing identity-provider issuers, key sources, audience/profile, role mapping or configuration-source trust requires dedicated permission under the currently trusted policy. The candidate configuration cannot authorize its own installation. An untrusted project file cannot introduce a trusted issuer merely by winning precedence. New sources need host-approved provenance and permitted scopes before their content participates in resolution. The existing [authentication design](../../openspec/changes/design-provider-neutral-auth/design.md) already binds keys to configured trust and separates profile access from credential/role administration.

JWT guidance requires issuer/key binding, audience validation and caution around attacker-provided key URLs. Configuration implication: activation must retain those checks and reject a candidate that weakens the configured trust boundary without authorized policy change. This is a ROM design inference, not a claim that JWT standards define a configuration system. [RFC 8725 §§3.8–3.10](https://www.rfc-editor.org/rfc/rfc8725.html#section-3.8).

## Secret references and disclosure

Store typed references to approved secret resolvers, scoped identifiers and optional pinned secret versions. Resolve values only inside the trusted adapter lifecycle, not during generic discovery or Studio rendering. Whether a reference follows rotation or pins a version must be explicit. A change to a secret reference can change authority and is itself privileged.

Credentials must stay out of resource events, idempotency receipts, source diffs, diagnostics and generic projections. A reference identifier may also need redaction. Masking an input widget does not protect persistence or journal history. Resource read/write permissions, disclosure projection and resolver access are separate checks; authorization to inspect configuration need not grant permission to extract credentials. [Authentication requirements](../../openspec/changes/design-provider-neutral-auth/specs/provider-neutral-auth/spec.md).

## Startup, reload and adapter activation

1. Read a bounded, versioned source set. Establish provenance and trust. Resolve precedence. Decode against accepted descriptors. Validate cross-resource references/capabilities. Collect safe diagnostics without secret content.
2. Prepare one immutable candidate with its exact source/Resource revisions. Validate identity-provider profiles, adapter compatibility and available secrets. A connection probe can detect problems but cannot guarantee future availability.
3. Before publication, verify that the candidate still applies. Serialize or conditionally arbitrate activation against the active generation. This prevents a slow earlier reload from overwriting a later one. Publish one complete local snapshot. In-flight work pins the appropriate generation under explicit freshness and shutdown rules.
4. Record active generation and activation outcome through recoverable bookkeeping. If external activation is required, distinguish pending, active, failed and unresolved observations from the earlier committed configuration revision.

Invalid startup configuration does not start dependent protected services without a valid state. Invalid reload leaves the last known valid generation active only while its explicit security/expiry policy permits; it is not an excuse to accept expired credentials or ignore a mandatory revocation. Reject the candidate visibly and retain bounded, safe diagnostics. Policy/trust updates need cache invalidation and bounded propagation to subscriptions and in-flight calls. [Immutable descriptor snapshots](../../openspec/changes/establish-rom/specs/field-extensions/spec.md), [authorization freshness](../../openspec/changes/design-provider-neutral-auth/specs/provider-neutral-auth/spec.md).

Local atomic publication is not a distributed transaction. Replacing pools, moving a listener, changing remote credentials or reconfiguring external services can have irreversible or uncertain effects. Prepare replacement handles where possible, switch local references, then drain old handles; record restart-required changes explicitly. Durable activation intent and idempotent reconciliation recover crashes between Resource commit and activation. Rollback means a new validated activation attempt, not a guarantee that remote effects disappeared. If compensation fails, retain degraded/unknown states. [Existing durable-effect boundaries](../../openspec/changes/design-capability-adapters/design.md).

Do not assume atomic multi-Resource configuration writes. The current persistence proposal guarantees a resource with its associated records. Larger transactions are separate capabilities. A candidate spanning several Resources needs an explicitly coordinated revision set/publication protocol that prevents mixed edits from activating halfway. Multi-host convergence likewise needs version observation, freshness and failure policy; a local snapshot swap does not prove all hosts changed together.

## Acceptance questions before implementation

- Can settings and identity-provider configuration use the same Resource registration, action, authorization, journal and live-read paths as ordinary application data?
- Can Studio explain the winning source and ownership, reject a shadowed edit, remove an overlay explicitly, and survive concurrent source refresh without silent loss?
- Do untrusted higher-priority inputs fail to change trust roots, secret access or their own permissions? Can the installation bootstrap and recover without public unauthenticated takeover?
- Do invalid reloads, deleted/unavailable sources, expired last-valid state and secret rotation produce distinct observable outcomes with no credential leakage?
- Can crash injection at commit/prepare/publish/drain boundaries recover to an honest active generation, detect stale candidates and report external partial failure without claiming atomic rollback?
