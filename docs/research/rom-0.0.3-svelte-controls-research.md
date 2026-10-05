# ROM 0.0.3: Svelte controls research

Research date: 2026-10-05. This note records source review and proposed experiments. No control was implemented or tested for this note.

## Recommendation

Reuse the installed shadcn-svelte source and Bits UI primitives. Use native inputs where they preserve the field contract. Add no dependency for the first control increment. Evaluate `svelte-dnd-action` separately if pointer dragging remains a requirement. Keep move buttons even when dragging is available.

The checked-in [package manifest](../../studio/package.json) pins Svelte 5.57.1, Bits UI 2.19.5, Tailwind CSS 4.3.3, and `@internationalized/date` 3.12.4. Calendar, Command, Popover, Select, Input, Textarea, and Native Select source already exists under [the UI directory](../../studio/src/lib/components/ui). Current upstream examples can change independently of these installed versions. Verify exported APIs against installed source before using an example.

## Preserve the value contract

ROM's current [field descriptors](../../studio/src/lib/client/types.ts) describe shapes, codec identity, and codec wrappers. They do not expose a generic UI-hint structure. The renderer [registry](../../studio/src/lib/renderers/registry.ts) keys custom renderers by codec name and version. The [value editor](../../studio/src/lib/renderers/ValueEditor.svelte) applies codec renderers inside declared wrappers.

Use semantic codecs when values have different meaning or validation rules. Use UI hints when two controls edit the same value contract. This distinction is a proposed design rule, not an implemented metadata extension.

| Field choice | Semantic contract | Optional UI hint |
| --- | --- | --- |
| Enum | Declared members and their wire values | Select, radio group, searchable chooser |
| Date | Calendar date, local datetime, instant, or zoned datetime | Calendar popup, segmented input, display locale |
| Color | Accepted syntax, color space, alpha, canonical representation | Swatches or picker |
| Number | Integer range, binary floating point, or decimal precision and scale | Unit label, input step, slider |
| Relation | Resource kind and stable reference ID | Searchable label or compact ID input |
| JSON | Allowed JSON structure and numeric interpretation | Text view or tree view |
| File | Blob reference and attachment lifecycle | File chooser, drop target, thumbnail |
| List | Element contract and meaningful sequence order | Move buttons or drag handle |

Do not infer date, money, color, or JSON semantics from a field name. Do not make display locale part of wire encoding. Treat UI hints as advisory. Retain a contract-preserving fallback when Studio does not know a hint or codec.

## Enum and relation choosers

The current shadcn-svelte combobox composes Command and Popover. Its example closes the popup and returns focus to the trigger. It separates stored country values from displayed labels. This composition fits a searchable enum without another dependency. [shadcn-svelte Combobox](https://www.shadcn-svelte.com/docs/components/combobox)

Bits UI also supplies dedicated Combobox primitives. Its examples separate search text from selected values and let the application filter items. Choose this API when the requirement needs an editable combobox input. Do not assume the Command composition and dedicated Combobox expose identical keyboard behavior. [Bits UI Combobox](https://www.bits-ui.com/docs/components/combobox)

For a short enum, retain the existing Select adapter or use Native Select. For a large enum, use bounded search results. Preserve wire values when labels change. Do not create arbitrary enum values from typed search text. Show an explicit error for an unknown stored member instead of selecting the first option. These are ROM recommendations.

For relations, retain the stable reference ID as the value. Fetch display labels through authorized Resource queries. Keep the selected ID visible when a label cannot be resolved. Bound results and cancel superseded searches. Distinguish loading, no matches, and query failure. Use the existing client's query and cancellation contracts; a combobox supplies no authorization boundary. [ROM client contracts](../../studio/src/lib/client/types.ts)

Check accessible names, selected-option announcements, Escape behavior, and focus after selection. The WAI combobox pattern describes these interaction requirements. A copied example is not proof that the complete form meets them. [WAI Combobox Pattern](https://www.w3.org/WAI/ARIA/apg/patterns/combobox/)

## Dates, time, and time zones

The shadcn-svelte date picker composes Popover with Calendar or RangeCalendar. Its date-and-time example adds a separate time input. Natural-language parsing is a separate example with `chrono-node`; it is not required for the calendar. Reuse the installed calendar before introducing another picker library. [shadcn-svelte Date Picker](https://shadcn-svelte.com/docs/components/date-picker)

Bits UI Date Field provides date and time segments. Its `granularity` supports day, hour, minute, and second. A segmented field is a candidate for explicit datetime entry. Granularity controls the visible segments; it does not define the wire contract or preserve precision beyond those segments automatically. [Bits UI Date Field](https://www.bits-ui.com/docs/components/date-field)

`@internationalized/date` supplies distinct `CalendarDate`, `CalendarDateTime`, and `ZonedDateTime` types. These cover date-only, local datetime, and named-zone datetime use cases. The package is framework-independent despite its documentation appearing under React Aria. [Internationalized Date](https://react-aria.adobe.com/internationalized/date/)

| Meaning | Suggested editor | Required decision |
| --- | --- | --- |
| Calendar date | Calendar plus explicit date text | Calendar system and accepted year range |
| Local time | Native time input plus text fallback | Precision and whether seconds are allowed |
| Local datetime | Date Field or date and time inputs | No implicit conversion to the browser's zone |
| Instant | Date and time plus explicit display zone | Canonical instant encoding and retained precision |
| Zoned datetime | Date and time plus IANA zone chooser | Whether the zone survives serialization |
| Time-zone identifier | Searchable string chooser | Supported identifiers and alias policy |

A native `datetime-local` value has no time zone. Native time and date inputs also have specific normalized formats and step rules. Keep these controls behind codec adapters instead of changing arbitrary strings into timestamps. [HTML input states](https://html.spec.whatwg.org/multipage/input.html)

For a named-zone datetime, define how to handle daylight-saving gaps and repeated times. `ZonedDateTime` offers earlier, later, compatible, and reject disambiguation. Its default can adjust a local time. Its string representation includes zone and offset; its absolute string represents UTC. Use the codec's declared representation instead of choosing a serializer from the UI type. [ZonedDateTime](https://react-aria.adobe.com/internationalized/date/ZonedDateTime)

Recommend rejecting ambiguous or nonexistent local times unless the codec defines another policy. Show the resulting offset when a user chooses a repeated time. Preserve subsecond precision during a change that only edits the date. Do not silently reduce an existing timestamp to the control's displayed precision.

`Intl.supportedValuesOf("timeZone")` can supply runtime-supported zone identifiers. Its result is not a complete product policy for historical aliases or application-supported zones. Use descriptor-provided choices when that policy matters. Show offsets for the selected date; avoid permanent labels such as “GMT-5 New York.” [ECMA-402 supported values](https://tc39.es/ecma402/#sec-intl.supportedvaluesof)

## Sortable lists

React DnD is explicitly a React library. Adopting its React component API would require a second framework integration. That is unnecessary for a Svelte field editor. Treat the user's React DnD reference as an interaction requirement unless a React integration is explicitly required. [React DnD repository](https://github.com/react-dnd/react-dnd)

| Approach | Evidence | ROM recommendation |
| --- | --- | --- |
| Move up/down buttons | W3C gives adjacent move controls as a non-drag alternative | Implement as the baseline; no added dependency |
| `svelte-dnd-action` | Svelte action; documented mouse, touch, keyboard, drag handles, and announcements | Evaluate as progressive enhancement |
| React DnD | React-focused component integration | Avoid for the current Svelte Studio |

WCAG 2.2 treats keyboard access and a single-pointer alternative to dragging as separate requirements. Keyboard dragging alone does not satisfy the latter. Move buttons work with keyboard activation and clicks or taps. [W3C Dragging Movements](https://www.w3.org/WAI/WCAG22/Understanding/dragging-movements.html)

`svelte-dnd-action` documents `consider` and `finalize` events, stable item IDs, touch support, and keyboard interaction. Its README marks accessibility support as beta. The package manifest inspected declares version 0.9.79, MIT licensing, no runtime dependencies, and a Svelte peer range that admits Svelte 5. This is upstream source evidence, not an installed-version test. [README](https://github.com/isaacHagoel/svelte-dnd-action), [package manifest](https://raw.githubusercontent.com/isaacHagoel/svelte-dnd-action/master/package.json)

The release history records Svelte 5 runes fixes and ongoing keyboard, touch, and teardown changes. Version 0.9.58 had a `$state` regression; 0.9.59 records its fix. Version 0.9.66 records another runes fix. Pin the chosen release and test it with ROM's exact stack. Do not treat an old regression as proof that the current version is broken. [Release notes](https://raw.githubusercontent.com/isaacHagoel/svelte-dnd-action/master/release-notes.md)

Keep UI row IDs separate from Resource IDs and list values. Duplicate primitive values still need distinct row identities. A reorder must move the value, draft, validation error, and row identity together. Do not serialize drag placeholders or temporary IDs. Keep input editing independent from drag handles. Preserve focus after a move. These are proposed ROM invariants.

## Color

Use a labeled text input with a preview first. Add a native color picker for an explicitly supported subset, such as opaque sRGB colors. Browser support for newer alpha and color-space options needs verification on ROM's supported browsers. The HTML standard defines color input state and its related attributes; it does not establish browser interoperability. [HTML Color state](https://html.spec.whatwg.org/multipage/input.html#color-state-(type=color))

Define accepted syntax through a semantic color codec. Decide whether equivalent representations normalize and whether alpha is required. Keep text entry available so the picker cannot erase a value it cannot express. Give the preview an accessible text value. Validate on the server as well as in the editor. No extra color library is justified until these requirements exceed native controls.

## JSON, nested values, and maps

Reuse ROM's recursive list and map editors for declared shapes. A map is not an arbitrary JSON document. The current shape union does not describe heterogeneous nested object fields. A JSON-string codec also differs from a structured JSON wire value. Keep these cases explicit. [Shape and codec contracts](../../studio/src/lib/client/types.ts)

Start an arbitrary JSON editor with Textarea and ROM's bounded `parseWire` and `stringifyWire`. Keep invalid text as a draft. Prevent submission until parsing and semantic validation pass. Retain duplicate-key rejection, depth and byte limits, large integers, and null-prototype maps. These properties already exist in [the wire serializer](../../studio/src/lib/client/serialization.ts).

`svelte-jsoneditor` is a possible later enhancement. Its 3.x line requires Svelte 5 and its repository declares the ISC license. It offers text and structured editing. Its default parser is native JSON; custom parsers and validation parsers are supported. Its change callback can return temporarily invalid text. [svelte-jsoneditor](https://github.com/josdejong/svelte-jsoneditor)

Do not adopt the JSON editor with its default parser for exact ROM integers. Evaluate a ROM parser adapter, structural edits, and validator behavior together. A parser hook alone does not prove every internal edit preserves `bigint` or floating-token categories. Measure the built dependency cost before adoption. Prefer a lazy-loaded advanced view if its benefit is demonstrated.

For maps, reject duplicate key insertion and rename collisions. Treat empty keys according to the declared contract. Preserve values when changing keys. Keep bounds visible when editing is unavailable. Test prototype-like keys such as `__proto__` through the existing null-prototype representation. These are proposed checks, not executed results.

## Exact numeric editing

Svelte converts bound numeric input values to numbers. Empty or invalid numeric inputs become `undefined`. This is unsuitable as the sole path for exact `u64` and `i64` values. [Svelte binding documentation](https://svelte.dev/docs/svelte/bind)

ROM already parses integer drafts with `BigInt` and checks the declared integer range. Preserve that path. Use a text draft with a suitable `inputmode`; convert only after lexical validation. Keep intermediate empty or minus-only drafts separate from committed values. Do not use `valueAsNumber`, a slider, or a floating-point formatter for exact integers. [ValueEditor](../../studio/src/lib/renderers/ValueEditor.svelte), [normalization](../../studio/src/lib/client/normalization.ts)

For `f64`, retain the finite binary-floating-point contract. If money or exact decimal quantities are required, define a separate decimal codec with precision and scale. A unit suffix or currency label does not make `f64` exact. Do not add a decimal package before the wire representation and arithmetic requirements are agreed.

## File controls

Use a native file input as the selection control. Its `accept` attribute is a chooser hint. Validate content and limits through the blob boundary. Add a drop target only as another selection path. [HTML File Upload state](https://html.spec.whatwg.org/multipage/input.html#file-upload-state-(type=file))

ROM exposes blob capabilities, reservation, upload, download, and detach operations. Those operations and limits define the lifecycle. A generic string renderer cannot safely infer that a value is a blob reference. Show selected file details, operation state, and errors through the blob contract. [Blob client contracts](../../studio/src/lib/client/types.ts), [blob client](../../studio/src/lib/client/blobs.ts)

Keep reservation and upload outcomes distinct. Make detach explicit. Do not treat an interrupted upload as a confirmed failure or clear the stored reference automatically. Provide an accessible chooser even when a preview or drop surface is present. No upload-widget dependency is required for the initial control.

## Compatibility and licensing limits

shadcn-svelte and Bits UI upstream licenses are MIT. Preserve existing attribution and ROM's runtime-notice workflow when copying or adding source. [shadcn-svelte license](https://raw.githubusercontent.com/huntabyte/shadcn-svelte/main/LICENSE.md), [Bits UI license](https://raw.githubusercontent.com/huntabyte/bits-ui/main/LICENSE)

This note does not establish release support periods or predict future maintenance. Upstream manifests and release notes show declared compatibility and recorded changes. They do not prove compatibility with ROM's pinned build. No package download size, compressed bundle size, or accessibility pass rate was measured.

## Proposed experiments

1. Build searchable enums and relations from installed components. Check unknown values, query cancellation, labels, focus, and keyboard selection.
2. Compare Calendar plus text with Bits UI Date Field. Check leap days, invalid dates, precision retention, and daylight-saving gaps and folds.
3. Add list move controls. Verify duplicate values, nested drafts, errors, read-only mode, and focus after reordering.
4. If dragging is required, test pinned `svelte-dnd-action` with touch scrolling, keyboard handles, nested lists, and deletion during dragging.
5. Exercise `u64` maximum, `i64` minimum, `9007199254740993`, invalid drafts, and exact request serialization through every numeric control.
6. Compare bounded JSON Textarea with an optional JSON editor. Check large integers, floating tokens, duplicate keys, invalid drafts, and dependency cost.
7. Verify file selection, capability limits, interrupted uploads, retry behavior, download, and explicit detach through the real blob boundary.

Run the affected checks and full local verifier when these proposals become implementation work. Record browser, assistive technology, package versions, and evidence paths for executed experiments.
