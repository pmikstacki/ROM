# ROM 0.0.3: descriptors, controls, and Studio usability

Date: 2026-10-05. Status: research and proposed release scope, with an approved inspector increment in progress.

This report combines local source review, primary-source research, and the owner's feedback on 0.0.2.
It does not certify 0.0.3 or report unexecuted experiments as successful tests.

## Release purpose

Make Studio a useful generic interface for backend operations on ROM Resources.
A Resource declaration must produce understandable tables, details, forms, action inputs, and relation choices.
Improve descriptor authoring and backend/frontend conformance together. More controls alone cannot repair an incomplete contract.

Keep the single Resource premise. Studio must not bypass actions, authorization, revision checks, receipts, or events through direct database editing.
Keep Svelte and shadcn-svelte. Keep compact labeled field rows, icons on narrow screens, and text beside icons where space permits.

## Lessons from 0.0.2

| Evidence or feedback | Cause or lesson | Release response |
| --- | --- | --- |
| `demo-ticket-code` renderer unavailable | The demo entry point registers its renderer; the standard entry point does not. | Verify extension composition in the actual packaged application, not only an isolated harness. |
| `Resource ["local","human","alice"]` heading | The details heading uses the exact ID as its human title. | Separate human presentation from exact identity. Use authorized title fields; keep ID copyable. |
| Enum options show raw values | A wire value and a display label have different purposes. | Preserve wire values; add labels and semantic displays. |
| Strings cover many unrelated meanings | Storage shape does not establish date, color, money, or email semantics. | Add explicit semantic formats and accepted value contracts. |
| Work shows hashes and JSON | The protocol is exposed without an operator presentation layer. | Use a compact list, status labels, structured inspector, and explicit recovery controls. |
| Details toggle sits beside quick Filters | The control does not indicate the panel location or direction. | Use a right-edge chevron and the shared right inspector. |
| Automated workflows passed, but users found friction | Correctness tests do not establish usability. | Add interaction, layout, focus, and readable-label acceptance scenarios. |

Local evidence: [client descriptors](../../studio/src/lib/client/types.ts), [renderer registry](../../studio/src/lib/renderers/registry.ts),
[value editor](../../studio/src/lib/renderers/ValueEditor.svelte), [Resource details](../../studio/src/lib/application/ResourceDetails.svelte),
[demo entry point](../../demo/studio/main.ts), [standard entry point](../../studio/src/main.ts),
[build composition](../../studio/vite.config.ts), and [Work view contract](../../crates/rom/src/operator/protocol.rs).
These are source observations, not new browser test results.

## Primary-source conclusions

Use metadata-driven controls as Supabase Studio does, but route operations through ROM's core rather than directly editing database rows.
PocketBase supplies a useful field catalog. Its defaults and null rules must not replace ROM's field-state contract.
[Supabase row editor source](https://raw.githubusercontent.com/supabase/supabase/master/apps/studio/components/interfaces/TableGridEditor/SidePanelEditor/RowEditor/InputField.tsx),
[PocketBase fields](https://pocketbase.io/docs/collections/).

Directus separates field interfaces from displays and validation. JSON Forms separates layout metadata from the data schema.
For ROM, this supports one semantic descriptor plus optional presentation metadata, not two competing definitions of accepted values.
[Directus fields](https://directus.com/docs/guides/data-model/fields),
[JSON Forms UI schema](https://jsonforms.io/docs/uischema/).

Reuse the installed shadcn-svelte primitives for most controls. Input Group supports adornments; Date Picker composes Calendar and Popover.
A control library supplies interaction primitives. ROM must still define representation, validation, and mutation semantics.
[Input Group](https://shadcn-svelte.com/docs/components/input-group),
[Date Picker](https://shadcn-svelte.com/docs/components/date-picker).

Detailed research: [admin renderer patterns](rom-0.0.3-admin-renderer-patterns.md),
[Svelte controls and adoption probes](rom-0.0.3-svelte-controls-research.md),
[screen ergonomics audit](rom-0.0.3-screen-ergonomics-audit.md).

## Descriptor contract

| Layer | Proposed responsibility | Must not do |
| --- | --- | --- |
| Semantic descriptor | Shape, codec/version, wrappers, constraints, reference kind, accepted enum values, structured field paths | Infer semantics from names or CSS widgets |
| Presentation metadata | Resource/field labels, help, title selector, preferred columns, editor/display hint, units, enum labels | Redefine allowed values or grant authority |
| Discovery projection | Expose only authorized metadata, with bounded versioned encoding | Reveal hidden fields through title templates or relation labels |
| Renderer registry | Deterministic selection for editor, cell, detail, filter, and action contexts | Download arbitrary executable code named by a backend descriptor |
| Runtime | Authorize, validate, normalize, commit, and return field-path errors | Trust browser validation as enforcement |

Prefer one intermediate descriptor model for derive generation and manual declarations.
Expose convenient Rust attributes and fluent helpers over that model. Exact attribute syntax needs a compiler diagnostic probe before adoption.
Require no handwritten TypeScript schema for an ordinary Resource.
Generate or mechanically verify Rust/TypeScript wire definitions. Test actual discovery JSON, codec roundtrips, and mutations, not just type declarations.

Separate discovery protocol version, Resource schema version, and codec version. A cosmetic label change must not require a persistence migration.
A representation or semantic change needs an explicit compatibility decision. An old client must not silently interpret a new codec as an old one.
Unknown optional presentation hints can use a known semantic renderer. Unknown semantics cannot become editable through a text fallback.

### Custom codecs

Support three cases explicitly:

1. A standard semantic codec with a built-in generic control.
2. A custom codec that declares a supported, lossless editable representation and backend validation.
3. A genuinely opaque codec that needs an explicitly bundled custom renderer.

A renderer can emit a candidate value; the backend remains authoritative.
A shape of `string` alone does not prove that arbitrary text is a valid custom value.
Keep a readable, bounded, read-only fallback for unknown semantics. Show renderer diagnostics to developers without dominating ordinary field labels.
Verify required opaque fields during application composition, so authors learn about missing editors before a user reaches an unusable create form.

### Resource presentation

Resolve the title from a declared field or field selector using the current authorized projection.
For a User, show `Alice` as the title and `User` as the kind label.
Show `["local","human","alice"]` as secondary technical identity, with an exact copy action.
Do not parse arbitrary string IDs as JSON to infer a human name.

If the title is absent or denied, use a neutral kind label and shortened ID.
Reuse the resolver in headings, table summaries, relation pickers, breadcrumbs, and links.
Presentation cannot request a hidden field solely to improve a title.

## Control coverage proposal

| Priority | Value family | Editor | Table/detail presentation | Acceptance risk |
| --- | --- | --- | --- | --- |
| P0 | String and multiline | Input or expanded Textarea | Bounded text, explicit expansion | Draft retention, long values |
| P0 | Boolean | Switch or checkbox | Labeled state, optional icon | False differs from absence |
| P0 | Enum | Select; searchable chooser for large sets | Label or badge | Label changes must not change stored value |
| P0 | Enum list | Multi-choice control | Bounded chips and count | Membership, duplicates, ordering |
| P0 | i64/u64/f64 | Exact text draft with numeric affordances | Exact integer; declared numeric formatting | Precision, incomplete input, overflow |
| P0 | Date/time | Segmented entry and Calendar/Popover where useful | Explicit locale/timezone display | Date-only versus instant; DST and precision |
| P0 | Color | Text plus swatch; native picker for supported syntax | Swatch plus accessible exact value | Alpha and unsupported browser syntax |
| P0 | Resource reference | Authorized searchable selector | Human title, kind, secondary ID | Revocation, missing targets, no N+1 lookup |
| P0 | Lists/maps | Compact summary and expanded editor | Count, short preview, inspect action | Reorder draft/error identity; key collisions |
| P0 | Optional/nullable | Existing intent menu, integrated with each editor | Distinct empty states | Omit/null/remove/empty must remain distinct |
| P0 | Custom codecs | Representation-based generic or bundled renderer | Context-specific display | Version mismatch and wrapper composition |
| P1 | Email/URL/phone | Semantic input and validation hints | Safe link plus copy | Unsafe schemes, preservation of exact values |
| P1 | Decimal/money/duration | Declared precision/unit-aware draft | Declared scale, currency, unit | No conversion through floating-point Number |
| P1 | Structured objects/JSON | Nested descriptor editor; bounded text/tree alternative | Summary plus expansion | Heterogeneous object support requires descriptor work |
| P1 | Blob/file/image | Existing Blob lifecycle with semantic field binding | Authorized filename/preview | No invented direct storage URL or upload commit |
| P1 | Secret/protected value | Masked/replace control when contract permits | Redacted state | No reveal through discovery, logs, or table title |
| Extension | Rich text/code/geo/specialized domain value | Explicit lazy-loaded specialized control | Safe specialized display | Sanitization, bundle cost, unsupported semantics |

P0/P1 are proposed sequencing, not an implemented support matrix or unconditional release guarantee.
Agree the finite acceptance catalog before implementation. Do not advertise support for every possible Rust type.
Broad coverage comes from composition of semantic controls and wrappers, with extensions for domain-specific values.

## Sortable lists

React DnD is not a Svelte-native dependency. Evaluate `svelte-dnd-action` with the exact installed Svelte version.
Keep keyboard move actions even if pointer dragging succeeds. Preserve stable row identity for duplicate values.
Move value, draft, validation error, and focus together. Never serialize temporary drag IDs or placeholder rows.
[Library documentation and keyboard behavior](https://github.com/isaacHagoel/svelte-dnd-action).

Compare native move controls against the drag library on touch, keyboard, nested inputs, and teardown.
Choose from executed interaction results and built bundle size, not the existence of a drag demo.

## Shared inspector and Work

The owner approved the right-edge chevron concept. Keep the quick Filters popover and a shared right inspector with Filters/Details tabs.
On desktop, the inspector occupies a right column. On narrow screens, it opens from the right as a drawer.
The chevron indicates open/close direction. Accessible names and expanded state remain available without visible text.
Preserve unsaved forms and filters when switching tabs or changing viewport size.

Use the same shell for Resource and Work inspection; the content and guarantees remain domain-specific.
The Work list should show definition, category, status, and attempts before the secondary technical handle.
Use the available state/category/definition filters and cursor-based continuation.
Load permitted capabilities and initial data automatically. Expose refresh and errors rather than manual protocol setup buttons.

The current WorkView contains state, attempts, due, delivery, source, and target, but no complete event history.
Show these as a current snapshot. The meaning and units of `due` need verification before labeling it as a wall-clock time.
Do not infer start times, execution duration, or earlier attempt outcomes from a single snapshot.

Temporal's UI demonstrates compact summaries, event-group timelines, and separate full history views.
For ROM, start with the compact list and inspector. A real timeline needs an authorized, paginated history contract first.
[Temporal workflow UI design](https://temporal.io/blog/the-dark-magic-of-workflow-exploration).

Recovery remains explicit. Preserve expected version, confirmation, and the same idempotency key after an unknown outcome.
Never rename an unknown outcome to failure or trigger Retry automatically after a timeout.

## Experiments and E2E acceptance

| Experiment | Positive case | Failure or regression case | Evidence to record |
| --- | --- | --- | --- |
| Descriptor authoring | Two unrelated Resources, derive and manual route | Invalid hint/type pair and hidden title field | Compile diagnostics and discovery fixtures |
| Cross-language contract | Native fixture reaches actual packaged Studio | Unknown protocol/codec, mixed version, duplicate labels | Rust/TS roundtrip and independent consumer |
| Renderer catalog | Create, patch, action input, cell, detail, filter | Nested wrappers and missing required custom editor | Shared conformance matrix |
| Numeric/date/color | Exact roundtrip through backend | Large integers, DST ambiguity, alpha, invalid draft | Chromium/WebKit plus backend state assertion |
| Sortable list | Keyboard and touch reorder duplicate values | Validation follows wrong row; cancel/teardown | Identity assertions and interaction video |
| Resource titles | Authorized human title with exact ID copy | Missing/revoked title, malicious text, oversized ID | No disclosure and layout assertions |
| Inspector | Chevron opens from right; Escape restores focus | Resize loses draft; controls clipped; duplicate IDs | Browser interaction and accessibility checks |
| Work | Readable list and structured snapshot | Unauthorized recovery, stale version, unknown acknowledgement | Backend work state and request identity |
| Performance | Large enum, relation lookup, nested values | Unbounded queries, eager editor dependencies, long UI stalls | Bundle bytes, request count, render timing |

Small ergonomics failures deserve E2E coverage when they cross components or browser behavior.
Use unit tests for deterministic label and value resolution. Use browser tests for focus, keyboard, geometry, drawer interaction, and drag behavior.
Assert behavior rather than screenshot equality alone. Screenshots supplement the tested transitions.

Run representative E2E cases on Chromium and WebKit, with narrow and wide layouts.
Run shared backend journeys on SQLite and redb. A polished UI must preserve the same durable semantics on both adapters.
Test forbidden metadata and mutations, server field errors, conflict recovery, sign-out, and stale-session responses.

## Delivery sequence

1. Land the approved inspector increment and its focused interaction regressions.
2. Establish the descriptor/presentation model and Rust/TS conformance fixtures.
3. Implement human Resource presentation and verify custom renderer composition in packaged builds.
4. Deliver enums, scalar semantic controls, lists/maps, and relation choices through shared contexts.
5. Complete Work and Attachments usability using the common inspector and existing authority boundaries.
6. Execute the adoption probes for dragging, JSON editing, and specialized controls before adding dependencies.
7. Run the whole-screen ergonomics matrix, independent author journey, full local verifier, and extracted release acceptance.

Do not bump the release version or claim release readiness from this research alone.
Retain the 0.0.2 evidence and screenshots. Update README images when the new implemented UI passes browser acceptance.

## Mock status

[Right inspector mock](mockups/rom-0.0.3/right-inspector-proposal.png) is an imagegen proposal based on the accepted 0.0.2 screenshot.
The prompt requested closed/open desktop states, a right-edge chevron, a right inspector, and a readable User heading.
Illustrative rows, pagination, field formats, and colors in the generated image are not implemented contracts or exact screenshot evidence.
The owner approved the side-panel direction. The generated mixed-kind table does not require a new cross-kind query feature.
