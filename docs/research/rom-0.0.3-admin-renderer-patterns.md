# ROM 0.0.3: admin renderer patterns

Date: 2026-10-05. Status: primary-source research and design recommendations.

This note compares Supabase Studio, PocketBase, Directus, and JSON Forms.
It proposes a generic Studio editor with one Resource model and the existing Svelte/shadcn-svelte stack.
No product was installed or tested for this note. External behavior below comes from documentation or source review.
Recommendations are not implemented behavior or verified release evidence.

## Findings from primary sources

| Product | Observed contract | Lesson for ROM | Limit or tradeoff |
| --- | --- | --- | --- |
| Supabase Studio | A generic table editor uses database field metadata. Its row input source selects controls for enums, foreign keys, text, JSON, datetime, and booleans. | Select controls from accepted descriptors. Reuse field metadata across table and form views. | Database editing is a different authority model from ROM actions. Do not introduce direct storage writes. |
| PocketBase | Collections define field types. Select fields expose predefined values and maximum selection counts. Date fields define a datetime string representation. | A compact field catalog can support broad editing without application-specific forms. Explicit value contracts make generic controls possible. | PocketBase normally uses zero defaults for non-JSON fields. ROM must preserve its own absence and null semantics. |
| Directus | Field schema, interface, display, validation, and conditions are distinct configuration sections. | Keep semantic validation separate from editor selection and display formatting. | A large interface-options surface introduces compatibility and authoring costs. |
| JSON Forms | UI schema describes controls, layout, and renderer options separately from the data schema. | Reference the existing descriptor from presentation metadata. Avoid a second source of accepted values. | Its listed renderer sets target React, Angular, and Vue. Adoption would need separate Svelte work. |

Supabase's table documentation describes generic row management through the Dashboard.
Its enum documentation treats the allowed set as a database type.
The row input source uses discovered enum values for a select and provides a foreign-key record-selection action.
It also offers expanded editors for text and JSON, including values too large for inline editing.
These are source observations, not browser checks.
[Tables and data](https://supabase.com/docs/guides/database/tables),
[Postgres enums](https://supabase.com/docs/guides/database/postgres/enums),
[Studio InputField source](https://raw.githubusercontent.com/supabase/supabase/master/apps/studio/components/interfaces/TableGridEditor/SidePanelEditor/RowEditor/InputField.tsx).

PocketBase supports collection and record management through its Dashboard and APIs.
Its select field stores a string for one selection and an array for multiple selections.
Its fields also include text, email, URL, editor, date, file, relation, and JSON types.
The documentation specifies zero defaults for most fields and describes date comparisons as string comparisons.
ROM should borrow the field catalog, while keeping its own wire formats and field-state rules.
[PocketBase collections and fields](https://pocketbase.io/docs/collections/).

Directus separates database schema settings from interface settings and value displays.
Its field documentation distinguishes labels and help text from stored keys.
It also describes client validation followed by server validation and permission checks.
ROM can use this separation without adopting Directus's database model.
[Directus fields](https://directus.com/docs/guides/data-model/fields).

Directus's dropdown choices have separate displayed text and stored values.
Its color interface supports strings, optional opacity, and presets.
Its datetime interface distinguishes dates, times, local datetime values, and timestamps.
Its repeater stores an array of objects and supports item summaries and sorting configuration.
These examples show why control appearance cannot establish value semantics.
[Directus interfaces](https://directus.com/docs/guides/data-model/interfaces).

A Directus custom interface declares its identifier, compatible types, component, and options.
The host supplies the current value. The component emits a value change.
The interface can recommend displays and receive disabled or noneditable state.
Its extension components use Vue 3. ROM should borrow the boundary pattern, not the components.
[Directus interface extension contract](https://directus.com/docs/guides/extensions/app-extensions/interfaces).

JSON Forms separates form layout from its data schema through a UI schema.
Its renderer catalog maps schema features to controls, including primitive lists and object tables.
This is evidence for a descriptor/presentation split, not a recommendation to replace ROM's current renderer system.
[JSON Forms UI schema](https://jsonforms.io/docs/uischema/),
[JSON Forms renderer sets](https://jsonforms.io/docs/renderer-sets/).

## Current ROM boundary

The inspected [client types](../../studio/src/lib/client/types.ts) expose field names, shapes, codec identities, and codec wrappers.
They also expose Resource kinds, versions, fields, actions, and action input descriptors.
They do not contain field labels, editor options, Resource title selectors, or display configuration.

The [renderer registry](../../studio/src/lib/renderers/registry.ts) selects a renderer by codec name and version.
It rejects duplicate registration and returns a disposal function.
The [renderer props](../../studio/src/lib/renderers/types.ts) already include editor, detail, and cell modes.
The [value editor](../../studio/src/lib/renderers/ValueEditor.svelte) blocks editing when a custom codec has no renderer.
The [field host](../../studio/src/lib/renderers/FieldHost.svelte) also blocks field-state changes for that unsupported codec.
These are local source observations. No tests were executed for this note.

The existing [field UX proposal](rom-studio-field-ux-proposal.md) records separate generic-preview and explicit-demo-renderer checks.
It states that the generic preview does not install the demo extension.
This note does not independently verify those historical browser results.

## Recommended descriptor and UI split

Keep Resource descriptors as the authoritative semantic contract.
Add optional presentation metadata that refers to accepted Resource and field paths.
Generate both from the same Resource definition when practical.
The presentation contract must not redefine field types, accepted enum values, codecs, or authorization.

| Contract layer | Proposed content | Owner |
| --- | --- | --- |
| Resource semantics | Shape, codec identity/version, wrapper placement, action input, reference target, authoritative constraints | ROM Resource authoring and runtime |
| Resource presentation | Singular/plural label, safe title-field selector, field order, field groups, table columns | Resource author, published through descriptor discovery |
| Field presentation | Label, help, placeholder, editor preference, enum labels, cell/detail display preference | Resource author, checked against semantic descriptor |
| Studio execution | Svelte renderer implementations, capability checks, form state, draft errors, loading and focus behavior | Studio |
| Request authority | Authorized discovery and projected values, action availability, server validation | Runtime and host |

This separation remains one Resource model. A label map is metadata about the accepted model.
It must not become a Studio schema that independently defines valid Resource values.

Use a narrow typed presentation contract before adding arbitrary renderer options.
Prefer explicit variants such as single-line, multiline, select, radio, list, and table.
Validate compatibility with the underlying shape and codec.
Unknown optional presentation preferences should preserve the semantic editor when that editor is supported.
An unknown codec must still prevent semantic editing.

Publish a version for the presentation contract independently from codec identity.
Define how older clients handle new optional presentation fields.
Treat unsupported semantic features as compatibility failures or read-only fields.
Do not silently reinterpret them as strings.

## Authoring ergonomics and interoperability

Provide Rust helpers or derive attributes for common semantic types and presentation settings.
The exact attribute syntax needs a separate authoring trial.
Authors should declare an enum once and obtain its wire choices automatically.
Authors should select a standard date or color codec without rewriting a serializer, descriptor, and renderer registration.
Application-specific codecs should require explicit registration through a documented extension entrypoint.

Generate or validate frontend descriptor types from the authoritative wire contract.
Avoid hand-maintained Rust/TypeScript copies with no compatibility checks.
Give action inputs the same field presentation facilities as Resource fields.
Keep presence wrappers and codec wrapper placement explicit across both paths.

Maintain shared conformance fixtures for backend encoding and frontend interpretation.
Fixtures should cover nested wrappers, enum values, exact integers, date strings, color strings, and unsupported versions.
Run the Rust and TypeScript sides against the same expected wire values.
Test unknown optional metadata separately from unknown semantic contracts.
Round trips must preserve null, absence, false, zero, empty values, and list order.

These are proposed release checks. This research did not implement or execute them.

## Generic controls for 0.0.3

| Family | Recommended behavior | Required semantic decision |
| --- | --- | --- |
| Enum | Select by default; optional radio preference; readable labels in tables and forms | Persist exact descriptor values. Labels never replace IDs. |
| Ordered list | Repeatable rows with add, remove, move up, move down, and optional drag handles | Reorder the list value in the draft. Preserve duplicates and nested codec semantics. |
| Map | Key/value rows with duplicate-key errors | A map is not an ordered list. Do not promise persisted user order. |
| Date | Calendar plus typed entry | Define a calendar-date representation without implicit timezone conversion. |
| Time | Typed time input with optional seconds | Define precision and whether an offset is permitted. |
| Instant | Date/time input with an explicit displayed timezone | Define offset handling and canonical representation in the codec. |
| Local datetime | Date/time input without an implicit instant conversion | Define wall-clock semantics explicitly. |
| Color | Text input, swatch, picker, and optional presets | Define accepted color format and alpha support through an explicit codec. |
| Reference | Authorized search, readable title, exact ID fallback | Reference target and value format come from the descriptor. |
| Custom codec | Registered renderer for the exact codec version | Renderer output must pass the existing backend codec. |

List reordering and Resource query sorting are different operations.
Reordering changes a field value and should mark that field edited.
Query sorting changes a view and should not create a mutation.
Keyboard movement is necessary even if drag sorting is added.
Stable row identity must survive moves; array position is insufficient for editor identity.
Concurrent list edits need an explicit conflict experience under ROM's revision contract.

Date and color controls need semantic codecs if they constrain accepted values.
A presentation hint can select a convenient input for an existing string contract.
That hint must not imply backend date or color validation that does not exist.
For 0.0.3, standard versioned codecs provide a clearer interoperability boundary.

## Missing demo renderer

Treat `demo-ticket-code` as an application extension, rather than a built-in generic string type.
Register its renderer in the demo composition entrypoint when the demo advertises editable ticket codes.
Keep the generic Studio build capable of rendering arbitrary discovered Resources without importing demo behavior.
Test both configurations: renderer installed and renderer missing.

The unsupported state should show the authorized value and a short read-only explanation.
It must preserve that value on a patch that edits other fields.
Forms that require an unsupported value need a clear reason that submission is unavailable.
Avoid editable raw JSON as a default escape hatch for unknown codecs.
Such editing needs an explicit supported contract and backend validation.

## Human Resource titles

Replace a heading such as `Resource ["local","human","alice"]` with a human Resource title and type.
For example, use `Alice` with type `Human` when an authorized title field supplies that text.
Show the exact ID as secondary text with a copy action.
Do not parse an arbitrary ID string as JSON to manufacture title or type semantics.

Add a Resource presentation label and a title-field selector to the published metadata.
Resolve the title only from the authorized projected value.
Use the same title resolver for headings, tables, detail links, and reference pickers.
If the title is absent or unavailable, use the declared type label and exact ID fallback.
Do not fetch a hidden title through a second unrestricted endpoint.

Keep title formatting separate from key equality and mutation routing.
Labels do not provide authority and can be duplicated.
Use an explicit human-facing type label; retain the machine kind where diagnostic detail is useful.
An unavailable title must not produce a misleading blank control or erase the reference ID.

## Scope and unresolved questions

Broad generic editing requires a capability matrix, not only attractive controls.
Cover create, detail, patch, replace, actions, tables, filters, references, and unsupported codecs.
Document which combinations are editable, read-only, or unavailable.
Preserve runtime actions, authorization, revisions, and mutation receipts throughout this matrix.

Before implementation, resolve descriptor ownership for nested object fields and renderer options.
Decide which standard codecs belong in core versus an optional authoring package.
Confirm whether a schema version or a separate discovery protocol version governs presentation additions.
Measure authoring effort using one Resource with enums, nested ordered lists, date values, colors, and a custom codec.
Measure title behavior using authorized, absent, and withheld title fields.

The first-party documentation is mutable. The Supabase source link follows `master`.
Pin external source commits before using their implementation details as acceptance evidence.
The researched products were not tested for mobile behavior, accessibility, performance, or exact wire preservation.
No claims of component reuse, release readiness, or frontend/backend compatibility follow from this source review.
