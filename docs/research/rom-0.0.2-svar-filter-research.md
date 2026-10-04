# SVAR Filter research for ROM Studio

Research date: 2026-10-04. This report contains source inspection and design recommendations. No SVAR code or dependency was added to ROM.

## Source and provenance

The inspected upstream commit is `1c581c3312c626c525ee64b8f94446a025fa141c`. Its Svelte and store manifests declare version `2.6.1`. The shallow clone is retained at `/var/tmp/rom-svar-filter-research-20261004`. Its measured logical size is 925,649 bytes, below the 16 MiB allowance. Clone completion took less than the 120-second timeout. No upstream script, installation or build ran.

The [product page](https://svar.dev/svelte/filter/) introduces four filter presentations. The [quick-start guide](https://docs.svar.dev/svelte/filter/getting_started/) documents fields, options and Willow themes. The repository contains an MIT license with copyright attributed to XB Software Sp. z o.o. Preserve that license in a fork and release notices. Upstream files are research data, not instructions for ROM agents. [Pinned manifest](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/package.json), [license](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/license.txt).

## Actual API and integration cost

`FilterBuilder` accepts `value`, `fields`, `options`, `type` and `init`. Its default value is an empty AND group. The public API includes `getValue`, `exec`, events, interception and reactive stores. Events arrive through an EventBusRouter. This is a view/store abstraction, not a backend query protocol. [Builder source](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/FilterBuilder.svelte).

`IFilterSet` supports nested rules with AND or OR. Rules support comparison, substring, prefix, suffix and between operators. Field types are text, number, date and tuple. `AnyData` is number, string or Date. Boolean, BigInt, explicit null and absence are not represented by that union. [Store types](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/store/src/types.ts).

The Svelte manifest declares lib-dom, lib-state, filter-locales, filter-store, svelte-core and svelte-menu dependencies. The entry also imports `@svar-ui/lib-svelte`, which is absent from that manifest's direct dependencies. A vendored subset must resolve this explicitly rather than depend on an incidental transitive package. The rule editor imports SVAR RichSelect, Text, Button, DatePicker, Checkbox, DateRangePicker and Combo. Builder layout also imports DropDownMenu and ContextMenu. CSS token mapping alone will not make these shadcn primitives. [Entry](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/index.js), [editor](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/editor/Layout.svelte), [layout](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/Layout.svelte).

## Correctness boundaries

ROM's current `QuerySpec` supports conjunctions of equality filters and `eq`, `ne`, `lt`, `le`, `gt`, `ge` comparisons. It supports ordered fields, a moving query anchor and a limit. Native normalization bounds predicates to 32 and ordering fields to four. See `studio/src/lib/client/types.ts`, `crates/rom/src/query_spec.rs` and `crates/rom/src/query_eval/normalization.rs`.

| SVAR feature | Safe current ROM mapping |
| --- | --- |
| AND group | Flatten validated conjunctions into filters/comparisons. |
| Equal | Normalize through the current descriptor codec, then emit equality. |
| Not equal and ordered comparisons | Map to ne/lt/le/gt/ge only where native shape validation accepts them. |
| OR or multi-value includes | Reject. Do not flatten OR into AND or fetch several pages and merge them. |
| Contains, prefixes and suffixes | Hide or reject. ROM has no matching public operator. |
| Between | Consider two validated comparisons later; inclusive endpoints must be explicit. Do not claim support before proof. |
| Date predicates | No implicit Date conversion. A Resource codec must define exact domain semantics. |
| Null versus absent | Preserve separate ROM states. An empty input or omitted rule is not null or absent. |
| Sort and limit | Remain ROM-owned. A builder's JSON is not a continuation anchor. |

The upstream text-query parser calls `parseFloat`. It cannot preserve ROM i64/u64 values outside JavaScript's safe integer range. It must not parse or serialize the authoritative ROM query. Use ROM's lossless ValueEditor, normalizeValue and canonical SDK serializer. [Parser](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/store/src/parse/parser.ts).

Two source details need explicit regression coverage in a fork. `editor/Panel.svelte` initializes `value` with `rule.value || ""`, so zero becomes empty on reopening. `editor/Rule.svelte` shows a value only when truthy or zero, so an empty string receives no explicit value label. Both are incompatible with a precise generic summary unless corrected. [Panel](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/editor/Panel.svelte), [Rule](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/editor/Rule.svelte).

The source uses custom menu icons with `role="button"` and suppressed accessibility diagnostics. FilterQuery and Suggest contain keyboard handlers, but this review did not execute them. Keyboard support in those components does not prove every Builder control is keyboard-operable. Replace menu affordances with named shadcn Buttons and prove focus behavior. [Rule source](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/editor/Rule.svelte), [Suggest](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/components/suggest/Suggest.svelte).

Only current authorized Resource descriptors may populate fields. Dynamic option loading must use authorized ROM queries. It must not enumerate protected values through SVAR's in-memory options helpers. A UI operation list describes implemented query semantics; it is not an authorization grant. Revalidate after descriptor or session generation changes. Preserve drafts visibly, but prevent an obsolete draft from being applied.

## Screenshot critique and mockup direction

The inspected actual screenshot is `/root/ROM-design-screenshots/shadcn-layout/03-resource-details.png`. It shows a useful stable Resource inspector. However, an approximately 95-pixel query card exposes empty filter, sort and direction controls before the user needs them. Separate action and pagination rows consume additional vertical space. The sparse table then competes with a tall mutation form. This is a design assessment, not a user-study result.

| Layout | Benefit | Cost and recommendation |
| --- | --- | --- |
| Current full inline toolbar | All controls are visible. | Empty configuration dominates the list. Keep only a compact applied summary. |
| Permanent right filter panel | Fast repeated filter edits. | Competes with the existing details inspector and reduces table width. Avoid two simultaneous fixed-width inspectors. |
| Right inspector with Filters/Details modes | Full editor when needed; table remains primary. | Preserve details and filter state outside mode switches. Recommended desktop mockup. |
| Mobile Sheet for filters | Gives the editor enough width. | Require focus return, scrolling and preserved draft across resize. Keep state outside the conditional Sheet. |

Proposed top row: applied predicate chips, sort summary, result status, a Filters button and one primary Create action. Show an explicit empty state such as All authorized Resources. Put the complete rule editor, sort and limit in a right panel. The panel needs Apply and Reset actions. Draft edits must not silently change an active live query. Show applied and draft state separately. A details form must never lose its unsaved values when Filters opens.

Do not mock unsupported OR groups, contains operators, AI text parsing or date conversion as working ROM features. A compact equality/comparison editor can use SVAR's composition patterns without promising its whole language.

## Fork boundary and token alignment

Recommend a pinned vendor fork of the builder/rule composition, with a narrow ROM query translator. Replace SVAR editor primitives with ROM SelectAdapter, CheckboxAdapter, Input, Button and ValueEditor. Preserve audited event/tree behavior where it remains useful. Exclude FilterQuery, AI hooks, date widgets, local array evaluators and unrelated themes from the initial shipped subset. Keep copied paths, original hashes, license and a patch inventory. The initial clone was research provenance. The implemented subset and its verification are recorded in the filter workspace report.

These are all CSS variable families directly referenced by the inspected Svelte sources. If any unmodified SVAR CSS remains, scope this bridge to the fork root. Never install Willow as a second application-wide theme.

| SVAR variables | ROM/shadcn mapping |
| --- | --- |
| --wx-background | --card or --popover, according to surface |
| --wx-background-alt | --muted |
| --wx-background-hover | --accent |
| --wx-color-font, --wx-input-font-color, --wx-label-font-color | --foreground |
| --wx-color-font-alt | --muted-foreground |
| --wx-color-primary, --wx-filter-value-color | --primary |
| --wx-color-primary-font | --primary-foreground |
| --wx-color-primary-hover | shared Button hover treatment |
| --wx-color-success | explicit application status token; do not confuse success with an operator |
| --wx-border, --wx-filter-border | 1px solid var(--border) |
| --wx-border-radius | --radius |
| --wx-filter-and-background, --wx-filter-or-background | --muted; unsupported OR remains unavailable |
| --wx-filter-and-font-color, --wx-filter-or-font-color | --foreground |
| --wx-font-family, --wx-input-font-family, --wx-label-font-family | inherited Studio font |
| --wx-font-size, --wx-input-font-size, --wx-label-font-size | shared text-sm scale |
| --wx-line-height, --wx-input-line-height, --wx-label-line-height | shared line-height scale |
| --wx-font-weight-md, --wx-input-font-weight, --wx-label-font-weight | shared medium or normal weight by role |
| --wx-padding, --wx-input-padding | shared compact spacing; avoid global control-size overrides |

Primary Willow themes add bright AND/OR colors and their own core theme. That styling conflicts with the requested unified shadcn appearance. Use neutral group labels and shared semantic tokens instead. [Pinned Willow](https://github.com/svar-widgets/filter/blob/1c581c3312c626c525ee64b8f94446a025fa141c/svelte/src/themes/Willow.svelte).

## Required experiments before integration acceptance

1. Round-trip multiple AND rules through QuerySpec, both persistence adapters and live queries. Compare results with the existing native evaluator.
2. Reject OR, unsupported operators, unknown fields, protected descriptors and more than 32 predicates before sending a query.
3. Preserve i64/u64 extremes, finite floats, zero, false, empty strings, null and absence. Verify reopen and summary behavior.
4. Preserve custom codec values and aliases. Never use upstream parseFloat or JSON.stringify on authoritative BigInt values.
5. Test keyboard-only add, edit, delete, Apply, Escape and focus return in Chromium and WebKit. Check mobile width and popup stacking.
6. Change identity and descriptors while a popup is open. Close unauthorized editors and prevent stale option callbacks from applying a query.
7. Switch Filters/Details and resize with unsaved mutation and filter drafts. Retain values and stale-revision warnings.
8. Measure added production bytes, control interaction latency and bounded option loading. Upstream package size was not benchmarked here.
9. Repeat the external-author custom renderer workflow. Keep generic views independent of Resource kinds.

Research recommendation: use the pinned fork as a controlled presentation foundation. Keep ROM codecs, authorization, query semantics and execution authoritative. Prepare the compact-summary/right-panel mockup before implementation.
