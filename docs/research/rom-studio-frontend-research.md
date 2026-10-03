# ROM Studio frontend: Svelte fit and reuse

Date: 2026-10-02. Research only: current primary documentation, published npm metadata and existing ROM research were inspected. No packages were installed, prototype scaffolded, or frontend tests run. The versions below describe this inspection, not an adoption decision.

**Recommend Svelte 5 for a disposable Studio proof, with SvelteKit as the application shell and Bits UI as the first component candidate.** Its component composition and reactive state fit a generic metadata-driven interface. TanStack Table is a credible list-view dependency when richer table behavior is needed. Superforms/Formsnap are conditional candidates. Their useful form machinery does not remove ROM's descriptor and presence semantics. The currently published Superforms package does not declare compatibility with SvelteKit 3.

Studio consumes the runtime's accepted Resource descriptors. Base definitions originate in Rust procedural macros or plugin/manual code, and the runtime handles their normalized descriptors identically. This research does not propose a no-code resource-definition editor or a second schema authority. Optional runtime attributes/extra fields remain unsettled and would need an explicit extension contract. Existing [authoring findings](authoring-crate-trials.md) and [implementation readiness](core-implementation-readiness.md) already identify descriptor/codec consistency and policy freshness as unfinished integration work.

## Candidate matrix

| Candidate | Reusable capability | Fit and limit for ROM |
| --- | --- | --- |
| **Svelte 5 + SvelteKit** | Components, reactive state, routing, layouts and configurable rendering. SvelteKit can also produce a static SPA. [Kit introduction](https://svelte.dev/docs/kit/introduction), [SPA deployment](https://svelte.dev/docs/kit/single-page-apps). | Strong preferred direction. Studio can call ROM's Rust HTTP extension through a browser client; adding Kit does not require moving resource logic into a JavaScript server. Choose deployment mode separately. |
| **Bits UI** | Headless Svelte 5 primitives emphasizing accessibility and composition. [Getting started](https://www.bits-ui.com/docs/getting-started). | Good candidate for selects, dialogs, menus and other interaction primitives. ROM still supplies field meaning, error wiring, design and keyboard/accessibility verification of assembled screens. |
| **shadcn-svelte** | Editable styled components built with Bits UI and Tailwind CSS. [Introduction](https://www.shadcn-svelte.com/docs). | Optional accelerator if its visual foundation suits Studio. Copied components become maintained application code; it is not a generic resource admin engine. |
| **Superforms 2** | Form state, client validation adapters, errors, constraints and submission hooks; JSON Schema adapter and an SPA mode for external APIs. [JSON Schema guide](https://superforms.rocks/get-started/json-schema), [SPA mode](https://superforms.rocks/concepts/spa). | Worth a bounded comparison. It does not choose widgets from ROM descriptors. Its schema adapter needs a supported projection, and its defaults/submission model must preserve command intent. Current Kit 3 peer mismatch is a concrete gate. |
| **Formsnap 2** | Accessible field/control/label/description/error composition around Superforms. [Documentation](https://formsnap.dev/docs). | Useful with Superforms; not an independent schema renderer. Generic field paths and custom controls remain ROM work. |
| **TanStack Table 9 Svelte adapter** | Headless table state and opt-in sorting/filtering/pagination/selection; native Svelte 5 reactive integration. [Svelte state guide](https://tanstack.com/table/latest/docs/framework/svelte/guide/table-state). | Good candidate for metadata-derived columns. Translate supported sort/filter/page state into ROM queries; do not silently filter only a downloaded page while presenting a global result. |
| **Fetch + EventSource behind a ROM client** | Standard request transport and server event streams with reconnection/event IDs. [EventSource standard](https://html.spec.whatwg.org/multipage/server-sent-events.html). | Small initial transport surface. ROM owns schema decoding, command identities, conflicts, subscription epochs and resynchronization. |
| **Kit remote functions/live queries** | Generated client/server invocation inside Kit; presently experimental. [Remote functions](https://svelte.dev/docs/kit/remote-functions). | Defer as the ROM integration foundation. They invoke a Kit server and do not directly provide the Rust runtime's resource/authorization protocol. |

## Verified compatibility, not an “install latest” recipe

Published metadata was fetched directly from npm's registry with read-only Node `fetch`. No dependency resolver was run. Primary manifests and docs confirm the relevant boundaries:

| Published version observed | Relevant declared compatibility |
| --- | --- |
| [Svelte 5.57.1](https://registry.npmjs.org/svelte/latest) | Node >=18. |
| [SvelteKit 3.0.0](https://registry.npmjs.org/@sveltejs/kit/latest) | Svelte ^5.57.1; Node >=22.17; Vite ^8.0.12 and TypeScript ^6.0.0 peers. |
| [Superforms 2.31.0](https://registry.npmjs.org/sveltekit-superforms/latest) | Svelte 5 is included; Kit peer is **1.x or 2.x**, excluding 3.0.0. |
| [Formsnap 2.0.1](https://registry.npmjs.org/formsnap/latest) | Svelte ^5.0.0 and Superforms ^2.19.0. |
| [Bits UI 2.19.4](https://registry.npmjs.org/bits-ui/latest) | Svelte ^5.33.0; Node >=20; @internationalized/date ^3.8.1 peer. |
| [TanStack Svelte Table 9.2.4](https://registry.npmjs.org/@tanstack/svelte-table/latest) | Svelte ^5.0.0; Node >=20. |

These declarations are not executed compatibility results. For a Kit 3 proof, start with ordinary Svelte form state and a small ROM adapter. Evaluate Superforms after declared Kit 3 support, or in a separately locked Kit 2 comparison if its value warrants that branch. Do not force-install across the peer mismatch and report it supported. [Superforms manifest](https://raw.githubusercontent.com/ciscoheat/sveltekit-superforms/main/package.json), [Kit v3 migration](https://svelte.dev/docs/kit/migrating-to-sveltekit-3).

TanStack's current v9 uses `createTable`, feature registration and rune-aware state. The older v8 guide uses `createSvelteTable`; these are different API generations. Current shadcn-svelte's data-table recipe already uses v9. There is no basis for repeating the old blanket claim that TanStack lacks Svelte 5 support. [Current adapter manifest](https://raw.githubusercontent.com/TanStack/table/main/packages/svelte-table/package.json), [v8 guide](https://tanstack.com/table/v8/docs/framework/svelte/svelte-table), [current shadcn recipe](https://www.shadcn-svelte.com/docs/components/data-table).

## How Svelte generic rendering works

Generic rendering is a runtime dispatch layer over ordinary compiled Svelte components. Decode server metadata into a validated discriminated union such as boolean/string/enum/custom-field descriptors. A local registry maps semantic field identifiers to editor, detail and compact cell components. A generic ResourceView loops over allowed descriptor fields, and a FieldHost selects the registered component for that field and mode. The registry can use Svelte's `Component<Props>` type to require a common host contract. Svelte 5 renders component-valued variables dynamically in runes mode. [Component typing](https://svelte.dev/docs/svelte/typescript#The-Component-type), [dynamic components](https://svelte.dev/docs/svelte/v5-migration-guide#Svelte-components-are-no-longer-classes-svelte:component-is-no-longer-necessary).

Illustrative FieldHost shape only; these ROM names are proposed and the snippet was not compiled:

```svelte
<script lang="ts">
  // descriptor and draft have already passed the ROM client boundary.
  let { descriptor, draft, mode, renderers, onoperation } = $props();
  let Renderer = $derived(renderers.resolve(descriptor, mode));
</script>

<Renderer {descriptor} {draft} {onoperation} />
```

`resolve` must always return a component: an approved custom renderer, a compatible built-in, or an explicit unsupported read-only fallback. A custom type plugin contributes its frontend renderer through the trusted frontend build/registration process. Receipt of a new server type ID does not download arbitrary executable JavaScript. List cells and detail displays need not instantiate full editors. The host owns labels, permissions, operation presence and error paths so renderers do not each recreate those rules.

Svelte's three reuse mechanisms have different jobs:

| Mechanism | Role in Studio |
| --- | --- |
| Component registry | Runtime selection for a descriptor that arrived over HTTP; automatic data-browser views use it. |
| Snippets (`{#snippet}` / `{@render}`) | Caller-supplied markup for a table cell, field layout or screen section; useful overrides in curated screens. [Snippet documentation](https://svelte.dev/docs/svelte/snippet). |
| TypeScript component generics | Compile-time relationships between known props, such as table rows and selection callbacks. They do not inspect a Rust type or generate controls from unknown runtime JSON. [Generic props](https://svelte.dev/docs/svelte/typescript#Generic-$props). |

A PocketBase-like resource browser can therefore be automatic while user management remains a curated Svelte route with deliberate sections and workflows. That route reuses the same FieldHost, value renderers, form operations, table and action client. It can choose field order and provide snippets without redefining the resource schema or bypassing server authorization. A new ordinary resource should appear in the generic browser without a new page. A specialized workflow may intentionally warrant one.

The [controls maintenance comparison](rom-studio-controls-research.md) examines shadcn-svelte, direct Bits UI and Skeleton separately from this runtime mechanism.

## What the descriptor-to-UI layer must own

The following is a proposed division of responsibility, not an implemented frontend contract:

| Accepted metadata/semantics | Generic Studio behavior |
| --- | --- |
| Stable resource/field identity, descriptor version and value codec | Route and column identity, decoding and compatibility checks. Keep stable identities separate from display labels. |
| Boolean, string, supported numeric family, enum, nullable/container shape | Default controls and read renderers, selected by semantic type/capabilities. Metadata should describe behavior; labels/order/widgets are optional presentation hints. |
| Action inputs, requiredness, defaults and supported write capabilities | Action forms and controls appropriate to the requested operation. Create requirements and patch requirements need separate interpretation. |
| Declared query capabilities | Offer only supported filters/sorts; pass them to the backend. Keep query limits and pagination explicit. |
| Structured error paths and action/revision outcomes | Field errors, summary/focus management, conflict recovery and uncertain-outcome UI. |
| Custom semantic field identifier | Registered, trusted Svelte renderer/editor plus ROM codec integration; a conservative unsupported/read-only fallback when unavailable. |

A runtime descriptor cannot manufacture TypeScript compile-time knowledge of an unknown plugin field. Known generated client bindings may be strongly typed. Dynamic Studio must validate descriptors and values at its boundary. A single exported JSON Schema projection can serve standard client validators, but must be generated from the same accepted descriptor. Do not manually maintain parallel Zod schemas for every resource.

Superforms' JSON Schema guide explicitly excludes unresolved definitions/`$ref`: references must be resolved before its adapter is used. Its `as const` type-inference example describes a statically known schema. A fetched schema does not confer the same compile-time type. Dynamic descriptor caches also need bounded versioned ownership. [JSON Schema guide](https://superforms.rocks/get-started/json-schema).

Custom Rust validation cannot in general be translated into browser validation. Use exportable constraints for immediate feedback, then display the authoritative Rust result. Numeric codecs also need an explicit JavaScript representation for integers/decimals outside safe native-number semantics; mapping every Rust number to a browser `number` is not a complete protocol. Field renderer lookup should use a registry of application-approved components, not executable scripts supplied by metadata. Svelte 5 supports component values and dynamic component rendering in runes mode. [Migration guide](https://svelte.dev/docs/svelte/v5-migration-guide).

## Presence, defaults and live form state

For patch/action drafts, keep **unchanged/absent**, **explicit null**, and **set value** distinct. An empty string, false or zero is a value. A dirty-field flag alone is insufficient: a user can intentionally set a value equal to the baseline, clear it to null, or revert the operation to unchanged. The command encoder should consume an explicit operation state. Read-only/disallowed fields must be excluded deliberately.

This needs particular care with Superforms. Its defaults can fill required empty fields with zero/false/empty strings, and a field that is optional and nullable defaults to null. Its JSON mode submits the form model, including values whose DOM controls are disabled, using data supported by `devalue`; it is not automatically ROM's canonical JSON protocol. [Defaults](https://superforms.rocks/default-values), [nested data](https://superforms.rocks/concepts/nested-data). An external-API SPA hook is a useful seam for a separate ROM encoder. Client validation remains advisory; the Rust action path validates values, permissions and revisions. [SPA mode](https://superforms.rocks/concepts/spa).

Keep the last authoritative snapshot, its revision and the user's draft separate. A live refresh must not silently overwrite edits or submit hidden defaults. On conflict, show the current server state and let the user resolve/reapply the intended operation. A canceled browser request or lost response does not prove that the mutation failed. The browser client must retain command identity for outcome lookup/retry according to ROM's contract. These requirements follow from the retained [persistence and cancellation findings](capability-prototype-results.md).

## Live reads, authentication freshness and resynchronization

Prefer one browser-client owner for requests/subscriptions, with Svelte bindings above it and Studio components above those bindings. This keeps the reusable protocol independent of Svelte and avoids each widget inventing retry/cursor logic. Mount/unmount cleanup can use Svelte lifecycle facilities. [Svelte lifecycle API](https://svelte.dev/docs/svelte/svelte#onMount).

EventSource supplies reconnect behavior and `Last-Event-ID`, but its constructor only accepts the URL and credentials option: arbitrary Authorization headers require another streaming approach, such as fetch plus an SSE parser. After the authentication transport is fixed, choose that approach. Do not put long-lived credentials into a query string. [HTML standard](https://html.spec.whatwg.org/multipage/server-sent-events.html#the-eventsource-interface).

ROM must define race-free initial snapshot/subscription handoff, reconnect generation/cursor validity, reset messages, expired-history recovery and bounded results. A durable event stream does not itself maintain filtered membership. Prefer authoritative result replacement for the first proof. Only with explicit ordering/resync rules, optimize into deltas. The [library slice](prototype-results.md) already found and repaired stale buffered authorization, so that regression belongs at the browser boundary too.

Policy changes must invalidate visible rows, field values, action availability and any affected drafts, even when no resource mutation occurred. Use actor/session and subscription generations to discard old responses. Logout, tenant changes and denial should close subscriptions and clear affected caches. Reconnection must reauthorize and obtain a fresh result when continuity is uncertain. No client can retract data already received, and a disconnected browser cannot learn a new revocation immediately; freshness behavior must be explicit rather than implied by “live.” Authorization remains enforced on the server for every disclosure and action.

## Smallest useful disposable proof, later

Use one generic list/detail/action surface with two accepted resource descriptors, including equivalent macro/manual origins where the backend fixture supports them. Include string, boolean, nullable field and one custom semantic field. Reuse Bits primitives; add the native v9 table only for required list behavior. Keep definition editing outside this proof.

Acceptance should demonstrate descriptor-driven rendering without per-resource pages; absent/null/value encoding including false/zero; structured Rust validation errors; revision conflict and lost-response retry; filtered membership changes; policy revocation while a draft is open; reconnect/reset without stale data; and keyboard/error-focus behavior. If the integrated HTTP contract is not ready, start with recorded protocol fixtures. Clearly label this as UI evidence rather than end-to-end ROM correctness. Record the locked dependency combination and compare form-helper effort only after this baseline exists.
