# ROM Studio field controls: draft for critique

Date: 2026-10-04. Status: visual proposal and first implementation slice. The
mock is not a production editor.

The current mobile form puts every field in a bordered card. It asks for an
“Operation” before it shows an input. Unknown custom codecs show a long warning.
The supplied iPhone screenshot shows that this makes ordinary editing slow and
lets infrastructure terms dominate the page.

The proposed form shows the control first. An unchanged field has no patch
entry. Changing the control produces a `value` intent. A small menu exposes
`null`, `remove`, and reset only when the field contract permits those states.
The form shows a visible edit count and a reviewable save action. This changes
presentation, not ROM's mutation semantics.

## Visual probes

- [Mobile edit form](../../studio/prototypes/field-editor-mobile.png)
- [Field-state action sheet](../../studio/prototypes/field-editor-states.png)
- [Field control atlas](../../studio/prototypes/field-editor-types.png)
- [Throwaway HTML](../../studio/prototypes/field-editor-mock.html)
- [Initial direct-control component harness](../../studio/prototypes/field-editor-current-build.png)
- [VPN preview after the direct-control change](../../studio/prototypes/field-editor-live-preview.png)
- [VPN preview with unsupported fields expanded](../../studio/prototypes/field-editor-live-advanced.png)
- [Explicit demo renderer on a mobile browser](../../studio/prototypes/field-editor-demo-renderer.png)

The sample uses a `maintenance-tickets` Resource because it contains ordinary,
nullable, collection, and custom-codec fields. The field atlas includes future
types to test the composition model. It does not claim that ROM already provides
date, time, email, or URL codecs.
The visual probe explores field content. The existing quick-filter popover and
Filters / Details sidebar remain part of Studio.

## Contract that the UI must keep

| Field state | UI behavior | Mutation result |
| --- | --- | --- |
| Unchanged | Show the current authorized value. Do not mark the field edited. | No patch entry. |
| Set value | Show an editor selected from the descriptor and registered codec. | `set` with a codec-valid value. False, zero, and empty are values. |
| Set null | Show an explicit empty state only for nullable fields. | `set` with `null`. |
| Remove | Offer only for a top-level `Presence<T>` field. | `remove`; do not substitute null. |
| Unknown codec | Show a compact read-only explanation. | Preserve the current field. |
| Invalid input | Keep focus and the draft. Show a field error. | Block submit. |

The wire descriptor and registered Rust codec remain authoritative. The UI
must not infer semantics from a field name. For example, `email_address` is
only an email control when the accepted descriptor declares email semantics.
The renderer registry uses the codec name and version. An unknown version must
not fall back to an editable plain string.

## Control families

| Accepted contract | Control proposal | Notes |
| --- | --- | --- |
| `String` | Single-line input; optional long-text presentation hint selects textarea. | A hint changes display only. |
| `bool` | Switch for a setting; checkbox for a selection. | Explicit false remains a value. |
| `u64`, `i64`, `FiniteF64` | Numeric input and optional step controls. | Keep integer text lossless until the shared codec parses it. |
| `Shape::Enum` | Select or radio group for a short set. | Persist the enum ID, not its label. |
| `ResourceRef<R>` | Authorized Resource picker with ID fallback. | Search through the shared query contract. |
| `Option<T>`, `Presence<T>` | Base control plus a field-state menu. | Null and remove are distinct. |
| `Vec<T>` | Repeatable row control. | Keep a bounded item count and stable keys. |
| `BTreeMap<String,T>` | Key/value rows. | Check duplicate keys and preserve order-independent semantics. |
| Registered custom codec | Versioned renderer. | Core validates the final value. |
| Proposed date/time, email, URL | Explicit versioned codec or accepted presentation metadata. | Add conformance tests before making the control editable. |

The installed shadcn-svelte `Field`, `Input`, `Switch`, `Select`, `Textarea`,
`Calendar`, `Popover`, and `Sheet` components can provide consistent control
structure. The current [Field guidance](https://shadcn-svelte.com/docs/components/field)
groups a label, control, description, and error. The current
[calendar guidance](https://shadcn-svelte.com/docs/components/calendar)
provides date and time examples. ROM must still supply descriptor selection,
field-state semantics, codecs, and authorization.

## Implementation boundary

First, change the existing patch form without changing the wire contract.
Give the renderer the current field value and a separate `FieldIntent`. Entering
text, selecting a value, or toggling a switch sets the intent. Reset restores
`omit`. Keep create, replace, and action inputs explicit; they do not have a
current value to preserve. Then add presentation metadata and new codec-backed
types through the same Field contract. Avoid a second Studio-only schema.

Browser checks must cover mobile width, keyboard focus, screen-reader names,
unknown custom codecs, current values, and every presence state. The same
tests must verify the exact patch JSON, including false, zero, empty, null,
remove, and omission. Only the accepted descriptor can enable a control.

## Verified first slice

The current Studio build uses direct controls for existing Resource patch
forms. It has a Switch for booleans, lossless integer text with a numeric
mobile keyboard, a reference ID search input with its target kind, and the
existing controls for enums, lists, maps, optional fields, and registered
custom codecs. In a direct editor, list and map entries now use bounded rows
with small add and remove controls. Unsupported versioned codecs are grouped under a collapsed
read-only section. Their authorized current values remain visible when the
section is opened. These changes do not add a new Rust Field type or infer a
type from its name.

The live mobile browser check used the VPN preview with the generic Studio
build and a `maintenance-tickets` Resource. The editor occupied the full
390-pixel viewport. The unsupported `demo-ticket-code` and
`demo-opaque-handle` codecs stayed read-only. A targeted regression test
first showed that the field menu's “Set a value” action inserted a default
instead of the current value; after the correction it passed. Current checks:
83 unit tests, 49 browser component tests, Svelte typecheck with no errors or
warnings, and the Studio production build. Twelve real-host Chromium scenarios
passed across SQLite and redb after the collection controls changed. They used
an existing native binary. One complete SQLite workflow was rerun after the
final null-versus-absent display change. Twelve WebKit real-host scenarios
passed with the final frontend bundle across SQLite and redb. Two real-host
scenarios also passed with the explicit `demo-ticket-code` renderer registered.
Those scenarios
edited the scalar,
optional, and list-wrapped custom values while the opaque codec stayed
read-only. The full native release gate is still pending.
The [browser evidence](evidence/rom-0.0.2/field-ux/README.md) records the
source, frontend, and existing native-binary limits.

A separate browser regression distinguishes a current null value from a
missing optional field and an empty string. Both remain unmodified until the
author enters a value or selects an explicit state action.

The mock's date/time, email, URL, long-text hint, and Resource picker remain
design candidates. They need explicit accepted codec or presentation metadata
and browser/runtime conformance before being made editable. The mock's sample
ticket-code editor assumes an installed `demo-ticket-code` renderer; the
generic VPN preview intentionally does not install that demo extension.
