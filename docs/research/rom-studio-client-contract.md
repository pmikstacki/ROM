# ROM Studio client and discovery contract

Date: 2026-10-02. Research proposal for a generic frontend, with Svelte preferred for presentation. No frontend implementation or new domain entity is introduced. Base Resource definitions come from Rust macros or plugin/manual code and use the same runtime handling.

## Recommendation and existing foundation

Build Studio over a framework-independent client SDK consuming authorized Resource descriptors and the core transport contract. The [boundary specification](../../openspec/changes/establish-rom/specs/boundary-adapters/spec.md) already names discovery, actions, reads, live observation and journal subscriptions; the [resource specification](../../openspec/changes/establish-rom/specs/resource-model/spec.md) makes the registered descriptor and codec one semantic authority. Studio should reveal that contract through generated navigation, tables, forms and actions, without creating a separate collection/model system or handwritten per-kind API calls.

The missing client contract is chiefly discovery shape, schema compatibility, supported query vocabulary and visible operation lifecycle. A generic HTTP binding may carry it, but neither HTTP paths nor Svelte components should define Resource behavior. [Transport review](transport-layer-review.md).

## Minimal discovery document

Publish a bounded, authorized descriptor snapshot rather than the complete internal Rust registry. Discoverability does not grant permission to act. Keep protocol version, descriptor generation, kind/action schema versions and UI-hint version distinct; they change for different reasons. Use immutable generations and a defined refresh/invalidation mechanism, as required by the [field-extension specification](../../openspec/changes/establish-rom/specs/field-extensions/spec.md).

| Portion | Proposed client-visible meaning |
| --- | --- |
| Scope and compatibility | Server/application identity, applicable isolation scope, supported client protocol/codec versions, descriptor generation, freshness rules and binding capabilities. Cache authorized metadata by session/scope and clear it when either changes. An opaque cache scope is not identity evidence. |
| Resource and field identity | Stable qualified kind IDs and field paths, recursive logical types and versions, reference targets, constraints, public encoding, read projection and separate create/update presence rules. No Rust `TypeId`, database columns or driver types. |
| Action definition | Stable action ID/version, target shape, input/output schemas, revision requirement, idempotency policy, completion/status-lookup facilities and safe error paths. Custom action inputs need not mirror stored fields. |
| Queries | Allowed predicates per field/type, ordering and null rules, projection, pagination mode/limits, cursor lifetime and consistency, and available count/live forms. Describe valid combinations rather than promising every combination of individual flags. |
| Observation | Supported live/journal modes, initial snapshot contract, update/reset frames, resume scope, expiry/gap behavior and freshness bounds. |
| Presentation | Locale keys/fallback labels, descriptions, units, preferred widgets, grouping, display order and confirmation hints. Optional, bounded, safely rendered data; no server-supplied executable frontend code. |

Static capabilities describe supported mechanisms; current action/field availability describes a scoped, potentially stale policy view. Metadata must not expose forbidden kind/field details or rejection reasons. Core still authorizes every read, invocation, discovery and delivery at use time. A read-only field annotation is not an authorization decision. [Authentication requirements](../../openspec/changes/design-provider-neutral-auth/specs/provider-neutral-auth/spec.md).

## Values and generated forms

Forms need explicit **unchanged/omitted**, **null**, and **value** states. False, zero and empty collections remain values. A hidden or unreadable field is not automatically null or absent from stored state; generate mutations from edited permitted inputs, not by resubmitting a projected resource. Preserve expected revision and the descriptor generation used to prepare the draft; stale metadata yields refresh/revalidation rather than reinterpretation.

JSON Schema can export structural constraints, but it is not the entire ROM contract. Its documentation distinguishes missing properties from null, and describes `default`, titles and descriptions as annotations rather than automatic mutation rules. Therefore a form must not silently materialize a suggested default as explicit user input; ROM needs authoritative omission/default behavior beside any schema export. [Objects](https://json-schema.org/understanding-json-schema/reference/object#required), [annotations](https://json-schema.org/understanding-json-schema/reference/annotations).

Codec descriptors must cover exact decimals, integer ranges, timestamp/date rules, byte encodings and nested/custom types. JSON implementations can differ beyond binary64 precision; the interoperable exact-integer range documented by RFC 8259 stops at ±(2^53−1). Proposal: use declared lossless encodings and SDK codecs rather than route every number through JavaScript `Number`. Unsupported custom editors should visibly refuse unsafe editing while retaining authorized inspection where a safe representation exists. Local validation improves feedback; core validation remains authoritative. [RFC 8259 §6](https://www.rfc-editor.org/rfc/rfc8259.html#section-6).

Localized labels and widget hints never change stable IDs, enum values, numeric units on the wire or validation rules. Parse locale-sensitive input deliberately and serialize canonical values. A display label rename must not change action identity or query meaning.

## Query, observation and action lifecycle

The SDK builds only declared query operators and combinations; it never substitutes client filtering over a page for server filtering over a collection. Opaque cursors stay bound to their query/order/scope and declared consistency. On query changes discard old cursors and ignore obsolete responses. Do not invent total counts or stable snapshots from a paginated result. [Storage query research](storage-relational-backends.md).

Expose live states such as loading, current, reconnecting/stale, resync-required and denied. Startup requires the core's consistent snapshot/change boundary or refresh. Reconnect uses the appropriate resume mechanism; coalesced live updates are not journal history. Revocation/expiry stops protected delivery and invalidates relevant cached UI data. Deltas need explicit base/version applicability; otherwise refresh. Reference-count subscriptions and bound cached pages/results so component churn cannot create unlimited work. [Live-read requirements](../../openspec/changes/establish-rom/specs/event-reactivity/spec.md).

Separate per-attempt request/correlation ID from the stable principal-scoped idempotency identity of one intended mutation. Keep identity and semantic input stable across lost replies; edits after a conflict constitute a new intended mutation. Show unresolved outcome distinctly from definite rejection or success; cancellation stops waiting, not necessarily execution. Recover through authorized outcome resolution or same-key retry under the retention contract. Never automatically replay pending work under a different logged-in principal. Persistent cross-reload recovery, if offered, needs an explicit bounded storage and privacy policy. [Action requirements](../../openspec/changes/establish-rom/specs/action-runtime/spec.md), [transport lifecycle](transport-layer-review.md).

## Ownership and control surfaces

| Layer | Owns |
| --- | --- |
| Core and binding | Descriptor truth, codecs, policy decisions, query semantics, transactional action/outcome behavior, revisions, live consistency and journal retention; binding-specific wire translation. |
| Generic client SDK | Discovery/version negotiation, lossless codecs, query construction, presence-aware drafts, session-scoped caches, retry identity, outcome resolution, bounded subscriptions and normalized lifecycle/errors. No independent business authorization or automatic browser persistence of credentials. |
| Svelte integration/components | Reactive adapters, accessible forms/tables/navigation, localization, field-editor registry, loading/conflict/reconnect UI and explicit user action controls. Custom renderers reuse SDK operations. |

Svelte documents stores as useful for complex asynchronous streams and manual subscription control. A thin Svelte store adapter over SDK subscriptions is therefore plausible; this does not require exposing Svelte reactive primitives in the protocol or SDK. [Svelte stores](https://svelte.dev/docs/svelte/stores).

Studio can expose resource browsing, standard/custom actions, permitted history, and safe operation diagnostics. Administrative retry, failed-work inspection, registration changes or transport controls appear only if separate authorized capabilities exist; frontend access is never blanket administration. Such controls are operations and infrastructure views, not competing domain entities.

## Base definitions and tentative runtime extensions

**Accepted base definition:** Rust macros or plugin/manual implementations register descriptors and handlers through the same validation gate and runtime. Studio consumes the accepted shared descriptor, manages instances and invokes registered actions regardless of authoring origin. It does not need separate generated-versus-manual code paths. Base-definition changes follow the host's code/plugin deployment lifecycle; Studio is not a no-code base schema editor. This matches the owner's clarification and the [authoring direction](rust-resource-authoring.md).

**Tentative runtime additional fields/attributes:** a future explicit extension contract might allow configured additions and publish validated generations. It must define storage and codec representation, admissible field types, policy, query/live behavior, migration/compatibility and rollback. Such additions cannot silently alter a compiled Rust struct or fabricate executable business handlers. Current immutable-generation requirements permit disciplined reconfiguration; they do not authorize arbitrary runtime schema creation.

The SDK can consume a combined accepted descriptor if that extension is later approved, while preserving provenance and explicitly marking which configuration is editable. Runtime additions remain unresolved; no alternate implementation language or dynamic base-definition profile is selected.

## Acceptance for a first client contract probe

- One new registered Resource appears in Studio without kind-specific routes/forms; unsupported custom codecs fail visibly. Round-trip omission/null/false, exact decimals and large integers through actual binding codecs.
- With two principals/scopes, prove discovery, counts, fields and caches do not leak across sessions; expire authority during live delivery and during uncertain-action recovery.
- Race query replacement, live startup, reconnect and concurrent edits. Verify consistent refresh, bounded retention, explicit gaps and conflict preservation; losing a mutation response never creates a second transition.
- Change descriptor generation while a form is open. Preserve the draft for explicit reconciliation without submitting it against incompatible semantics. Show equivalent macro/plugin definitions identically; offer runtime extension editing only if its separate contract is approved and exposed.
