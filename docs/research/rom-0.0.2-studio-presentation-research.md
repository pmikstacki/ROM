# ROM Studio presentation: Sidebar and controls

Date: 2026-10-04. Status: primary-source research and local dependency inspection; implementation and browser acceptance remain separate.

## Recommendation

Use the actual shadcn-svelte Sidebar, Select, Checkbox and Sheet component families. Keep the existing Vite/Svelte application and its controllers. Compose one responsive navigation shell around the current Resource, Work and Attachment pages. Do not introduce another framework, router or domain-specific dashboard model.

The official project provides a [Vite installation guide](https://www.shadcn-svelte.com/docs/installation/vite). Its examples do not require adopting SvelteKit. This recommendation is compatible with the inspected peer ranges; it is not a compilation result.

## Existing dependency baseline

The inspected [package manifest](../../studio/package.json), lock and installed package metadata agree on these relevant versions:

| Component or utility | Existing version | Compatibility observation |
| --- | --- | --- |
| Svelte | 5.57.1 | Satisfies Bits UI's `^5.33.0` peer range. |
| Bits UI | 2.19.5 | Already supplies Select, Checkbox and Dialog primitives. |
| `@internationalized/date` | 3.12.4 | Satisfies Bits UI's `^3.8.1` peer range. |
| Tailwind CSS and Vite integration | 4.3.3 | Already configured for the component styles. |
| shadcn-svelte CLI | 1.7.0 | Its installed Svelte peer range is `^5.0.0`. |
| `@lucide/svelte` | 1.51.0 | Already available; its peer range is `^5`. |
| `clsx`, `tailwind-merge`, `tailwind-variants` | 2.1.1, 3.7.0, 3.3.1 | Already declared application utilities. |
| `svelte-toolbelt` | 0.10.6 | Already locked transitively; its Svelte peer range is `^5.30.2`. |

The existing [component configuration](../../studio/components.json) selects `vega`, slate tokens and the current `$lib` aliases. Button, Input, Table and Dialog wrappers already exist. New wrappers should use those shared paths and the canonical UI utility module.

One dependency needs explicit accounting: [UI utils.ts](../../studio/src/lib/components/ui/utils.ts) reexports `cn`, but Studio does not declare that package directly. The installed CLI supplies `cn` 0.3.3 transitively. Either declare and pin it as an application dependency, or adapt the shared helper to the existing `clsx`/`tailwind-merge` dependencies with behavior checks. Do not rely on an unrelated development package to document runtime imports.

## Component choices

| Component | Recommended Studio use | Official contract |
| --- | --- | --- |
| Sidebar | One application navigation shell; descriptor-derived Resource entries; Work and Attachments groups; session controls in the footer. | `Provider` owns context; `Root` supports collapse variants; `Inset` wraps inset main content; `MenuButton.isActive` marks selection. [Sidebar](https://www.shadcn-svelte.com/docs/components/sidebar) |
| Select | Enum values, query fields/operators and field presence choices through one controlled adapter. | Use `Root type="single"`, string values and option items. Change callbacks are `onValueChange`, not native select events. [Select](https://www.shadcn-svelte.com/docs/components/select), [Bits UI Select](https://bits-ui.com/docs/components/select) |
| Checkbox | Boolean field values and genuine binary options. | Controlled `checked` and separate `indeterminate` properties; explicit labels and invalid-state attributes. [Checkbox](https://www.shadcn-svelte.com/docs/components/checkbox), [Bits UI Checkbox](https://bits-ui.com/docs/components/checkbox) |
| Sheet | Narrow-screen navigation or a secondary inspector when the layout requires an overlay. | Sheet extends Dialog, with side placement, title, description and close controls. [Sheet](https://www.shadcn-svelte.com/docs/components/sheet) |

Start with `Sidebar.Provider`, `Sidebar.Root variant="inset"` and `Sidebar.Inset`. An icon collapse mode is suitable for desktop navigation. Retain a visible Trigger and readable accessible names. Sidebar exposes separate mobile state through `useSidebar()`; do not destructure its reactive instance. The documented default shortcut is Cmd/Ctrl+B. Widths and theme values use shared CSS variables. [Sidebar API](https://www.shadcn-svelte.com/docs/components/sidebar)

Prefer CSS for layout breakpoints. Where rendering must change, Svelte's `MediaQuery` provides reactive matching and is available since 5.7.0. Its documentation explains the SSR fallback limitation. Studio's inspected Svelte version includes this API. [Svelte reactivity](https://svelte.dev/docs/svelte/svelte-reactivity)

Keep layout, navigation and control adapters in named modules. Do not move authentication, query or mutation orchestration into the copied UI primitives. Keep index modules as imports and exports. Use one navigation model for desktop and mobile to prevent descriptor and active-page divergence.

## Preserve ROM semantics

These are implementation constraints from the current Studio source, not new upstream component behavior:

- Derive Resource entries from authorized descriptors. Do not hardcode Task or Inventory navigation or fields.
- Keep `FieldIntent` choices distinct: `omit`, `value`, `null` and `remove`. Checkbox mixed state is not a substitute for those choices.
- Preserve a real enum value, including an empty string where the descriptor permits it. Do not use a legitimate value as a placeholder sentinel.
- Keep BigInt and lexical float encoding in the existing codec layer. Select labels must not normalize wire values.
- Keep unknown custom codecs read-only. Preserve codec wrapper handling, recursive collection bounds and action-input descriptor subsets.
- Preserve disabled state, retained drafts, descriptor-version invalidation, validation errors and explicit draft reopening.
- Keep unknown mutation outcomes and their exact retry identity visible when navigation collapses. Closing a Sheet must not imply commit or cancellation.
- Keep current session-generation checks, logout cancellation, CSRF handling and capability admission in their existing owners. Sidebar state is presentation state.

The inspected owners are [FieldHost](../../studio/src/lib/renderers/FieldHost.svelte), [ValueEditor](../../studio/src/lib/renderers/ValueEditor.svelte) and the application/controller modules. Control wrappers must delegate to their existing callbacks.

## Keyboard and responsive acceptance

Bits UI Select documents typeahead, arrow navigation and grouped options. Its current installed source also handles Home/End, Enter/Space, Tab and Escape. Checkbox handles Space and checked/mixed ARIA state. Preserve the primitive handlers when composing snippets. [Select behavior](https://bits-ui.com/docs/components/select), [Checkbox behavior](https://bits-ui.com/docs/components/checkbox)

Sheet inherits Dialog behavior. The inspected installed Dialog defaults include focus trapping, Tab looping, Escape dismissal and focus restoration, subject to callback overrides. Keep an accessible title and a close control. These mechanisms do not establish that Studio's eventual composition is accessible. [Bits UI Dialog](https://bits-ui.com/docs/components/dialog)

Run both existing browser engines against the composed controls. Check narrow-screen open/select/close, desktop collapse, long Resource labels, visible focus and focus return. Check enum selection, boolean changes, presence choices and validation focus. Repeat retained-draft, descriptor-change, unknown-outcome and logout regressions. Keep page/row/frame bounds during the presentation change. Record screenshots and executed assertions; do not infer human usability from component provenance.

## Provenance and notices

Copy component source into the existing UI tree rather than building parallel ad hoc implementations. Record the exact source revision or retrieved file hashes. A pinned CLI does not pin the remote registry contents. Do not use `@latest` as the release identity.

Preserve the existing [UI license](../../studio/src/lib/components/ui/LICENSE.md). The upstream [shadcn-svelte MIT notice](https://raw.githubusercontent.com/huntabyte/shadcn-svelte/main/LICENSE.md) requires retention of its copyright and permission text. Do not rewrite that text.

After implementation, regenerate the production bundle's existing runtime-module and notice inventory. Include every newly bundled dependency, including `cn` if retained. Preserve the actual `svelte-toolbelt` license file even though its installed package metadata has no license field. A list of package names or SPDX identifiers does not replace original notices. This is an engineering inventory requirement, not legal-completeness certification.

## Limits and source identity

Official documentation and installed Bits UI source were inspected. The exact upstream Sidebar registry dependency closure could not be retrieved. Its mobile branch was not source-confirmed to use Sheet. Therefore, review generated imports before claiming that Sidebar needs no additional direct dependencies.

No product code, dependency installation, build, browser run, service change, mount or cache write occurred for this research. Only this report was written. Browser diagnosis and implementation belong to separate tasks.

Baseline identities at inspection:

```text
568cae8125b8272637666c01f3fc2c61559c190fe1421892b9748bb5b067cd1c  studio/package.json
7830342279addd8dd5dab7ed82d66aa6574b0e70839b9c326b46225394c1f68e  studio/package-lock.json
0b919b66f29022c77392ced72a2b501a8c0e111eacb03169451b40f7458ec725  studio/components.json
```
