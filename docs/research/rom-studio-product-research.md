# ROM Studio: product and discovery contract research

Date: 2026-10-02. Primary documentation and source review; neither Supabase nor PocketBase was installed or run. Source observations cover the paths linked below, not every screen or deployment. GitHub heads observed: Supabase `b13d6c2878e009a33cf505e16cf288e87839745e`, postgres-meta `f380cc5be21edef4e77d9838c732d1f20af0b3c0`, PocketBase `5cec579da984436a258602a46a96302fbd31f77c`. Official PocketBase docs currently identify v0.40.4. Recommendations below are proposals for ROM, which has no integrated production core yet.

## Product direction and clarified scope

ROM Studio should combine curated Svelte administration screens, such as user management, with generated views for arbitrary resources declared in Rust. Both should reuse field editors, record lists and action forms derived from ROM discovery. It should not require another handwritten resource schema in TypeScript. Core owns the transport-independent semantic contract; an HTTP extension exposes it; Studio consumes that extension. This follows the existing [single-resource requirements](../../openspec/changes/establish-rom/specs/resource-model/spec.md) and [boundary requirements](../../openspec/changes/establish-rom/specs/boundary-adapters/spec.md).

The owner clarified that base resource/entity definitions are created in code through procedural macros. The latest clarification chiefly concerns curated versus generic UI rendering, not a requirement to implement runtime schema editing now. Extra attributes/additional fields remain an optional, unresolved future capability; Studio does not freely create base entity definitions. An unfinished reference to another possible language is not a language decision. Separate these operations visibly:

| Operation | Authority | Studio behavior |
| --- | --- | --- |
| Edit resource data; invoke existing actions | Compiled resource contract plus current core policy | Generated forms and action panels submit ordinary core operations. |
| Change labels, field order, help text or saved views | Presentation preferences/overrides | Editable without changing field meaning, validation or permissions. |
| Manage optional extension fields, if adopted later | An explicit runtime schema capability enabled by the compiled resource | Versioned, validated extension management, constrained by allowed types and limits; not a first-release requirement. |
| Add a base kind, alter a compiled field type or author a Rust action | Rust source/build/deployment | Show deployed metadata and provenance; change through code and deployment. |

This preserves the reason to use Rust types while leaving deliberate runtime flexibility possible. A screen that changes labels and one that introduces stored data fields cannot share an ambiguous “attributes” contract.

The owner explicitly confirmed that `User` is a ROM Resource, while Studio gives it a curated screen. Profile properties can use common field components; credentials, session revocation and invitation/reset workflows should use dedicated permitted actions and restricted projections. Declaring a field does not make it readable or CRUD-editable. The screen can compose generic components without duplicating the resource schema or bypassing core policies. These screen details are ROM design recommendations, not a claim that either reference product exposes identical semantics.

## What the reference products actually do

### Supabase Studio

Studio is a Next.js/Tailwind dashboard for existing Supabase deployments. Its self-hosted README emphasizes table/SQL editors, database administration and API documentation; project deployment administration is outside that mode's stated scope. Hosted and self-hosted features are not interchangeable: self-hosting is a single-project setup with settings largely supplied externally. [Studio README](https://github.com/supabase/supabase/blob/b13d6c2878e009a33cf505e16cf288e87839745e/apps/studio/README.md), [self-hosting scope](https://supabase.com/docs/guides/self-hosting).

The relevant architecture is genuinely database-shaped. postgres-meta normalizes the Postgres catalog and offers table/column/role/function management and SQL execution. In current Studio source, table discovery calls `pgMeta.tables.list()` and executes the resulting SQL; row updates construct SQL through the pg-meta query builder, optionally wrap role impersonation, then invalidate matching React Query entries. ROM should borrow generic discovery and consistent refresh behavior, not this SQL control plane. [postgres-meta contract](https://github.com/supabase/postgres-meta/blob/f380cc5be21edef4e77d9838c732d1f20af0b3c0/README.md), [table discovery](https://github.com/supabase/supabase/blob/b13d6c2878e009a33cf505e16cf288e87839745e/apps/studio/data/tables/tables-query.ts), [row update](https://github.com/supabase/supabase/blob/b13d6c2878e009a33cf505e16cf288e87839745e/apps/studio/data/table-rows/table-row-update-mutation.ts).

Supabase supports both data and schema editing: its dashboard can create tables and add columns. Database row-level policies govern ordinary access; table owners and roles with bypass privileges can behave differently, so an administrator's successful query does not demonstrate an ordinary user's access. The product also provides Postgres Changes subscriptions configured through publications. None of that alone proves every Studio table view stays current through a race-free live-query protocol. [Table/schema management and policy distinction](https://supabase.com/docs/guides/database/tables), [Postgres Changes](https://supabase.com/docs/guides/realtime/postgres-changes).

### PocketBase admin

PocketBase distinguishes collection definitions from their records, but exposes management of both through its dashboard and APIs. Collections map to generated SQLite tables; collection administration APIs require superuser authority. Base, auth and SQL-backed view collections have different behaviors. Most field types use non-nullable zero defaults; those defaults must not be imported into ROM's presence-aware update semantics. [Collection model](https://pocketbase.io/docs/collections/), [collection API](https://pocketbase.io/docs/api-collections/).

The current frontend is **not Svelte**. The maintainer's April 2026 announcement identifies a complete Shablon rewrite and marks stage one released in v0.37.0. The current package uses Vite and has no Svelte dependency; its entrypoint uses the global reactive application object. The same announcement warns that UI extension APIs remain provisional. PocketBase remains useful product inspiration, but it is not current evidence for adopting a Svelte codebase or a stable upstream extension API. [Maintainer announcement](https://github.com/pocketbase/pocketbase/discussions/7612), [package](https://github.com/pocketbase/pocketbase/blob/5cec579da984436a258602a46a96302fbd31f77c/ui/package.json), [entrypoint](https://github.com/pocketbase/pocketbase/blob/5cec579da984436a258602a46a96302fbd31f77c/ui/src/main.js).

Its generic rendering mechanism is concrete: the records list traverses `collection.fields`, resolves each field's view through `app.fieldTypes`, and queries records through the JavaScript SDK. The text field registry separates settings, input and display implementations. This is a useful separation for ROM's field presentation plugins. It does not demonstrate automatic discovery/forms for arbitrary custom domain actions. [Record list](https://github.com/pocketbase/pocketbase/blob/5cec579da984436a258602a46a96302fbd31f77c/ui/src/records/recordsList.js), [field registration](https://github.com/pocketbase/pocketbase/blob/5cec579da984436a258602a46a96302fbd31f77c/ui/src/fields/text/init.js).

PocketBase's API offers SSE for record create/update/delete and checks list/view rules for subscribers. Separately, the admin's SDK wrapper emits local CRUD document events explicitly without relying on realtime. Product realtime capability therefore cannot establish that its admin list follows every external change. Superusers bypass collection rules; ROM Studio should instead express any elevated access as an explicit core policy, rather than silently bypassing the shared mutation path. [Realtime contract](https://pocketbase.io/docs/api-realtime/), [admin SDK wrapper](https://github.com/pocketbase/pocketbase/blob/5cec579da984436a258602a46a96302fbd31f77c/ui/src/pb.js), [API rules](https://pocketbase.io/docs/api-rules-and-filters/).

PocketBase embeds built UI assets into its Go binary and serves its admin at `/_/`. It also automatically records dashboard collection changes as migration files by default. Borrow the convenient packaging and reviewable configuration history; unrestricted runtime collection creation is outside ROM's clarified scope. [Embedded assets](https://github.com/pocketbase/pocketbase/blob/5cec579da984436a258602a46a96302fbd31f77c/ui/embed.go), [deployment routes](https://pocketbase.io/docs/), [migrations](https://pocketbase.io/docs/js-migrations/).

## What to borrow and what to reject

| Pattern | ROM decision and reason |
| --- | --- |
| One generic resource browser with typed editors | Borrow. Discover registered kinds and their permitted operations without application-specific pages, alongside curated screens that reuse those components. |
| Per-field input/display renderer registry | Borrow. Standard fields should work immediately; custom Rust types need an explicit browser representation and optional renderer. |
| Separate record browsing from definition inspection | Borrow. Users should understand whether they are editing values, presentation, extension definitions or inspecting compiled contracts. |
| Table/SQL editor as the principal control surface | Reject for core Studio. Storage tables do not necessarily describe domain resources, actions or adapter-neutral guarantees. |
| Administrator writes directly to storage | Reject. Studio writes must retain validation, authorization, revision checks, receipts and committed events. |
| Automatic arbitrary domain-action UI inferred from function names | Reject. Action target, input/output contract and retry semantics must be declared and discoverable. |
| Runtime creation of arbitrary base collections | Reject for this scope. It would introduce another schema authority alongside compiled Rust resources. |
| Client-maintained filtered-list membership from raw CRUD events | Avoid as the initial contract. Consume ROM live snapshots/resynchronization so pagination, policy and membership remain server responsibilities. |
| Mandatory separate frontend application server | Avoid for the first distribution if unnecessary. Static Svelte assets can be served by the optional ROM HTTP host; host authentication remains server-owned. |

These are design judgments drawn from the inspected mechanisms, not comparative usability or performance measurements. Svelte remains a reasonable independent preference: it compiles web components, and SvelteKit supports static output if its routing is useful. This does not select a grid/form library or prove performance on large datasets. [Svelte overview](https://svelte.dev/docs/svelte/overview), [static output](https://svelte.dev/docs/kit/adapter-static).

## Core discovery contract Studio needs

Discovery should be a versioned, policy-filtered projection of the authoritative ROM registry. Svelte components and layout preferences belong in Studio; resource semantics belong in core. The minimum useful projection is:

1. **Protocol and resource identity:** discovery version, deployed resource/field versions, stable identifiers, registry generation, documentation and supported capabilities. Avoid using display labels as identity.
2. **Field contract:** canonical wire type/codec version, missing/null/value distinctions, constraints, readonly/computed status, supported filter/sort operators, relation targets and cardinality. Structured presentation hints may select a renderer, but cannot redefine validation. Unknown custom types need an explicit safe fallback, including readonly where editing cannot be represented.
3. **Actions:** stable identity, kind/instance scope, input and output schemas, required target revision, durable request identity/retry behavior and structured errors. Availability may depend on actor and current record; discovery is an affordance, not authorization proof. Recheck on invocation.
4. **Read/query contract:** supported predicates, stable pagination, bounded results, projection and live-read capability. The UI must not offer arbitrary joins or sorts an adapter cannot satisfy.
5. **Live delivery:** initial snapshot handoff, generation/schema changes, reconnect/reset behavior, current-policy revocation and subscription termination. Active forms must recognize stale resource or schema versions rather than silently overwrite.
6. **Authorization and disclosure:** current actor/session boundary; permission to discover kinds/actions, read fields, invoke operations and administer extensions. Hidden UI controls are not enforcement, and disclosure of metadata itself may need scoping.
7. **Outcomes and operations:** committed/rejected/conflict/unknown states, durable outcome lookup where allowed, actionable field-path errors and correlation identity. History, reaction attempts and effect delivery are distinct views exposed only when core supports them.
8. **Extension configuration:** whether this compiled kind permits extras, allowed types/count/size/query capabilities, extension schema version, change permissions and compatibility rules. Surface presentation overrides separately.

The present [readiness report](core-implementation-readiness.md) already leaves codecs, complete query contracts and typed facade integration unresolved. Studio makes those omissions visible; a metadata mock is useful for UI exploration but would not prove the core implementation.

## Optional future extension fields and typed Rust resources

This section evaluates the earlier runtime-fields possibility. It is not required to deliver the clarified curated-plus-generic UI and should not block the initial Studio contract.

Recommended model: the compiled resource explicitly opts into an extension-field capability. Runtime definitions live in a bounded, versioned extension registry and values use a distinct namespace/container; they cannot shadow base field IDs, weaken base validation, replace identity/revision fields or introduce executable Rust behavior. The unified operational descriptor is derived from the compiled descriptor plus the validated extension version, with provenance retained for each field.

A runtime-added `priority` must not pretend to become a new member on an already compiled Rust struct. Typed code sees extras through a deliberate checked accessor/container; code-generated named accessors require a new build. Ordinary base-field mutations must preserve extension values, while explicit extension changes pass through the same action/receipt/event pipeline. A full replacement API must say what omitted extensions mean.

Extension schema changes need their own concurrency and compatibility policy: stable IDs, allowed conversions, default/backfill treatment, deletion/retention behavior, policy visibility and adapter capability admission. Capture an immutable combined schema version for each operation; reject or explicitly reconcile stale submissions and invalidate affected live reads. This introduces versioned runtime schema management alongside the currently frozen registration rule and therefore needs a deliberate future specification change. It is not accomplished by making a mutable map inside the derive.

Begin with optional extra fields from a small existing field family and explicit limits. Defer required new fields, custom runtime validators and type conversion until migration semantics exist. These recommendations preserve a single compiled base definition while making the cost of dynamic extensions explicit; they are not implemented behavior.

## First useful acceptance slice

Use a real integrated core when available, with two differently shaped Rust resources, one custom action, a nullable field and an ownership rule. The same Studio build should discover both, render records/forms, invoke the action, handle a revision conflict, recover a lost response with the same request identity and remove revoked data from a live view. Adding a third ordinary resource must require only Rust declaration/registration, with no Studio schema or page edit.

Include a curated user-management screen built from the same primitives, with a restricted public projection and a dedicated sensitive action. Confirm that fields absent from the projection are not exposed by the generic view and that forbidden actions fail server-side even when manually invoked.

If runtime extension fields are subsequently selected, add a separate acceptance gate: enable one optional extra field on one kind and verify restart persistence, base-field collision rejection, stale-schema handling, extension preservation during a typed base update, policy enforcement and change history. Inspecting compiled definitions is in scope; creating another base kind through Studio is not. Until the relevant integrated tests succeed, present Studio as a prototype of the intended contract rather than an existing generic admin product.
