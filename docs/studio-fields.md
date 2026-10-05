# Studio field contracts

The 0.0.3 control catalog is under verification. This document does not identify a completed release artifact.

Declare field semantics in Rust. Studio selects controls from the disclosed shape and exact codec identity.
Do not infer a date, color, or number from a field name.

## Standard semantic fields

The `rom-fields` crate supplies these version-one codecs. They use the ordinary Resource mutation and discovery contracts.

| Rust type | Codec | Wire representation | Editor |
| --- | --- | --- | --- |
| `Date` | `rom.date` | Gregorian `YYYY-MM-DD`, years 0001–9999 | Date input |
| `Time` | `rom.time` | `HH:MM:SS` with optional nanoseconds | Explicit time text |
| `DateTime` | `rom.datetime` | UTC timestamp with nine fractional digits | Explicit timestamp and offset text |
| `Color` | `rom.color` | Lowercase `#RRGGBB` or `#RRGGBBAA` | Hex input and color picker |
| `Email` | `rom.email` | ASCII mailbox; domain is lowercase | Email input |
| `Url` | `rom.url` | Absolute HTTP or HTTPS URL without credentials | URL input |
| `Multiline` | `rom.multiline` | Exact UTF-8 text, at most one MiB | Compact preview and expanded text editor |
| `JsonDocument` | `rom.json-document` | Validated JSON source text, at most one MiB | Compact preview and expanded source editor |
| `Decimal` | `rom.decimal` | Canonical decimal text without an exponent | Exact decimal input |
| `UnitValue` | `rom.unit-value` | Object with decimal `value` and explicit `unit` | Magnitude and unit on one row |

The runtime validates every submitted value. Browser validation does not replace runtime validation.
Optional, nullable, list, and map wrappers retain their existing field-state semantics.

Timestamp normalization preserves nanoseconds. Studio does not use the browser's local zone to interpret an offset-free value.
A color picker changes RGB. An existing alpha suffix remains explicit in the hex representation.
Email validation does not establish mailbox ownership. URL validation does not perform network access.

Decimal values do not pass through JavaScript floating-point conversion.
The current query contract orders decimal strings lexically, rather than by numeric magnitude. It provides no decimal arithmetic.
Unit tokens are opaque. ROM does not infer units or convert measurements.

JSON documents retain their source text, including numeric tokens and duplicate member names.
Syntax validation does not convert that source into a JavaScript data object for storage.
The editor is a source editor, not a structural JSON tree editor.

## Enum labels

A Field can return advisory labels through `Field::enum_labels()`. Resource and Input derives expose those labels without a frontend schema.
Manual Resources can supply the corresponding `Resource::field_enum_labels()` bindings.
Registration validates label keys against the declared enum members, including optional, nullable, list, and map wrappers.
Labels do not change accepted values, persisted schema identity, or receipt normalization. Duplicate labels do not merge distinct enum members.

See [the demo enum](../demo/src/studio_choices.rs) and [the shared discovery fixture](../examples/consumer/tests/fixtures/enum-labels-discovery-v1.json).

## Lists and references

Generic collection editors accept at most one hundred entries and six nested levels.
Values beyond those editor bounds remain preserved; Studio must not submit a truncated replacement.
List rows retain private editor identities and invalid drafts during reordering.
Private identities are not part of the submitted wire value.
Move controls remain available when dragging is unavailable. Keyboard cancellation ends dragging; it does not reverse previously submitted reorder events.

A reference keeps the exact Resource ID. Its optional chooser queries only a currently disclosed Resource kind.
The chooser returns at most twenty candidates. Search filters those candidates; it is not an unrestricted full-text search.
A denied or cancelled lookup does not erase the stored ID. It must not retain a previously resolved title after disclosure changes.

An attachment reference does not replace the Blob upload, reservation, completion, or detach lifecycle.
Use the Attachments workspace for those operations.

## Custom renderers

Register a trusted Svelte component for the exact codec name and version in the application's entry point.
The standard Studio build does not load the demo application's custom renderer.
Unknown codecs retain a bounded display and preserve their stored value. They are not editable through an arbitrary text fallback.

Renderer props include an optional local `draft` and `onDraftChange` callback.
A renderer can retain invalid input through framework-owned list remounts with that callback.
The callback state is never submitted as a field value. Existing renderers do not need a bindable prop.

See [renderer types](../studio/src/lib/renderers/types.ts), [semantic controls](../studio/src/lib/renderers/SemanticField.svelte),
[backend codecs](../crates/rom-fields/src/lib.rs), and [shared codec vectors](../crates/rom-fields/tests/fixtures/semantic-codecs-v1.json).
