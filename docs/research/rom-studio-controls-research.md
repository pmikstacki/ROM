# ROM Studio controls: maintenance and selection

Date: 2026-10-02. Research only. Official documentation, repository source, GitHub commit/release metadata and published npm manifests were inspected. No dependencies were installed and no upstream or Studio tests were executed. This complements the [frontend architecture research](rom-studio-frontend-research.md).

**shadcn-svelte is a credible, actively maintained candidate for Studio's visual controls.** Recommend a small executable comparison using selected shadcn-svelte components over Bits UI, with direct Bits UI for a custom field where necessary. Keep Skeleton as an alternative if its packaged design system better fits the desired visual language. None supplies ROM's resource descriptor renderer, command semantics or authorization behavior.

## Maintenance evidence

GitHub's API was queried directly for each official repository's default-branch commits and releases. All three repositories were unarchived at inspection. Dates below are UTC. Default-branch commits are distinguished from repository-wide push activity.

| Project | Latest default-branch evidence observed | Release evidence observed | Interpretation |
| --- | --- | --- | --- |
| **shadcn-svelte** | 2026-09-30: native-select option-group fix, commit `3ef557c`. A 2026-09-29 commit updates svelte-sonner to v1.2. [Component fix](https://github.com/huntabyte/shadcn-svelte/commit/3ef557c89e744820bd038d4d979aaf2ea22ff4f4), [dependency update](https://github.com/huntabyte/shadcn-svelte/commit/5da38a72f62e149905748bdecd242f5960c9b89e). | CLI `shadcn-svelte@1.7.0`, published 2026-09-16; preceding 1.6.1 published 2026-09-04. [1.7.0](https://github.com/huntabyte/shadcn-svelte/releases/tag/shadcn-svelte@1.7.0), [1.6.1](https://github.com/huntabyte/shadcn-svelte/releases/tag/shadcn-svelte@1.6.1). | Recent component and dependency work supports active maintenance. CLI release numbers do not version every copied component in an application's source. |
| **Bits UI** | 2026-10-01: release commit `cdf337d`; preceding fix handles touch taps whose target stops click propagation. [Release commit](https://github.com/huntabyte/bits-ui/commit/cdf337dfa35b4ca1c8435b207fe0c12a9c2d643b), [interaction fix](https://github.com/huntabyte/bits-ui/commit/9576fb2645a50ee12bb130af2ddfd666d499eaca). | `bits-ui@2.19.4`, published 2026-10-01; preceding 2.19.3 on 2026-09-22. [2.19.4](https://github.com/huntabyte/bits-ui/releases/tag/bits-ui@2.19.4), [2.19.3](https://github.com/huntabyte/bits-ui/releases/tag/bits-ui@2.19.3). | Active work on interaction internals matters directly to dialogs/menus in an admin interface. |
| **Skeleton** | 2026-08-28: dependency update, commit `e65535e`; preceding docs changes on August 20/19. Repository push metadata was newer but is not evidence of a newer default-branch implementation. [Commit](https://github.com/skeletonlabs/skeleton/commit/e65535ebfb9b37bf37ccde1ed854b96397d21550). | `@skeletonlabs/skeleton-svelte@5.0.1`, published 2026-08-19, alongside the core package. [Svelte release](https://github.com/skeletonlabs/skeleton/releases/tag/@skeletonlabs/skeleton-svelte@5.0.1). | Recent v5 release and migration documentation support continuing maintenance; a slower short-term cadence does not establish abandonment. |

This is an activity snapshot, not a guarantee about future support, response times or maintainer capacity. No issue-response study or governance/bus-factor audit was performed. The shadcn-svelte website changelog's latest headline is May 2026. Repository activity is newer. The headline alone gives an incomplete picture. [Changelog](https://www.shadcn-svelte.com/docs/changelog).

## Ownership and upgrade work

| Choice | What ROM owns | What upstream updates | Tradeoff |
| --- | --- | --- | --- |
| **shadcn-svelte + Bits UI** | Selected component source, styling, composition and ROM wrappers. | Installed Bits UI/other runtime dependencies; replacement source is available from the registry. | Fast path to coherent controls, with source review/merge responsibility for copied files. |
| **Direct Bits UI** | All visual design and primitive composition, plus ROM field hosts. | Installed headless primitives and their internals. | Maximum design control; more styling and repeated composition work unless ROM builds its own small control library. |
| **Skeleton 5** | Screen composition, themes/customization and ROM adapters. | Installed design-system and Svelte component packages, which use Zag behavior dependencies. | More packaged visual conventions; upgrades can require theme/token/markup migrations. |

shadcn-svelte explicitly distributes editable component source. Its dialog root imports `Dialog` from `bits-ui`; its content wrapper supplies portal/overlay composition, classes and a labeled close control. Thus copying shadcn components does not copy the entire underlying interaction engine. [Ownership model](https://www.shadcn-svelte.com/docs), [dialog root](https://raw.githubusercontent.com/huntabyte/shadcn-svelte/main/docs/src/lib/registry/ui/dialog/dialog.svelte), [content source](https://raw.githubusercontent.com/huntabyte/shadcn-svelte/main/docs/src/lib/registry/ui/dialog/dialog-content.svelte).

The CLI's `add --overwrite` replaces existing files. The Svelte 5 migration guide explicitly calls for review of those diffs. Updating the CLI package alone does not update copied components. Recommended ROM policy:

- Record imported component provenance.
- Keep resource logic above the visual wrappers.
- Update runtime dependencies separately.
- Before you merge upstream source changes into customized controls, compare the changes.

Use only needed components. This is a proposed maintenance policy, not work performed here. [CLI](https://www.shadcn-svelte.com/docs/cli), [migration workflow](https://www.shadcn-svelte.com/docs/migration/svelte-5).

Skeleton's v5 migration changes theme tokens, utilities and some field-group markup; its CLI automates only part of that work. A packaged library therefore reduces copied-source maintenance but does not eliminate migration cost. [Migration guide](https://www.skeleton.dev/docs/svelte/get-started/migrate-from-v4).

## Compatibility and composition

Published npm metadata observed shadcn-svelte CLI **1.7.0** with Svelte ^5.0.0; Bits UI **2.19.4** with Svelte ^5.33.0; Skeleton core/Svelte **5.0.1**, with Tailwind ^4.0.0 and Svelte ^5.40.0 respectively. Skeleton Svelte's manifest depends on Zag packages and its shared package; it is a different interaction stack from Bits. [CLI metadata](https://registry.npmjs.org/shadcn-svelte/latest), [Bits metadata](https://registry.npmjs.org/bits-ui/latest), [Skeleton core](https://registry.npmjs.org/@skeletonlabs/skeleton/latest), [Skeleton Svelte](https://registry.npmjs.org/@skeletonlabs/skeleton-svelte/latest).

Both projects document SvelteKit setup. Skeleton lists Kit 2, Svelte 5 and Tailwind 4 as minimums. shadcn-svelte's inspected docs application still declares Kit ^2.70.1, Svelte ^5.54.0, Bits ^2.18.0 and TanStack Svelte Table ^9.0.0. Their component APIs are primarily Svelte APIs, but this inspection does not certify their scaffold instructions against newly published Kit 3. [shadcn installation](https://www.shadcn-svelte.com/docs/installation/sveltekit), [docs manifest](https://raw.githubusercontent.com/huntabyte/shadcn-svelte/main/docs/package.json), [Skeleton installation](https://www.skeleton.dev/docs/svelte/get-started/installation/sveltekit).

The current shadcn table recipe uses TanStack Table v9's native Svelte adapter. It supplies composition examples, not server-side ROM filtering, cursor semantics or automatically generated columns. The generic browser derives columns from accepted descriptors. Curated user-management screens can reuse the same table/cell controls with deliberate layouts. [Table recipe](https://www.shadcn-svelte.com/docs/components/data-table).

Form support is similarly layered. shadcn's Field examples provide field layout and error presentation; its Formsnap integration adds Superforms machinery. Choosing shadcn visual controls does not require adopting that integration. Superforms 2.31.0's Kit peer still excludes v3, as documented in the companion research. A first proof can use plain Svelte form state, ROM operation encoding and selected Field/Input/Select controls. [Field examples](https://www.shadcn-svelte.com/docs/components/field), [Formsnap integration](https://www.shadcn-svelte.com/docs/forms).

## Accessibility evidence and limits

Bits UI documents focus trapping, initial/return focus and Escape behavior for Dialog. Its repository contains browser test cases for focus restoration, nested focus scopes, ARIA description wiring and interaction behavior. The inspected file also contains a TODO-marked Escape test alongside other active Escape cases. The existence of a suite does not establish that every case runs. These are stronger signals than an accessibility slogan, but this review neither ran those tests nor audited a complete Studio screen. [Dialog documentation](https://www.bits-ui.com/docs/components/dialog), [browser test source](https://raw.githubusercontent.com/huntabyte/bits-ui/main/tests/src/tests/dialog/dialog.browser.test.ts).

Skeleton's Dialog exposes Zag behavior and its inspected Svelte test file checks rendered component parts. That file alone does not establish keyboard or screen-reader coverage for the complete framework. It also does not establish that Skeleton is less accessible: upstream Zag behavior and other tests were not comprehensively audited. [Dialog documentation](https://www.skeleton.dev/docs/svelte/framework-components/dialog), [test source](https://raw.githubusercontent.com/skeletonlabs/skeleton/main/packages/skeleton-svelte/test/components/dialog.test.ts).

For either choice, ROM must verify its own labels, dynamic errors, focus after validation, dialogs containing custom controls, live row removal, contrast and curated workflows. Headless behavior plus custom styling can still produce an inaccessible result. Generic metadata must provide enough semantics for a meaningful label and help/error relationship; a visual component cannot infer those from an opaque field ID.

## Conditional selection

**First trial: selected shadcn-svelte controls over Bits UI.** This combines an existing visual system with accessible-interaction machinery while keeping ROM's generic field registry and curated screens in ordinary Svelte code. It is a recommendation for the next proof, not a locked dependency decision. Use direct Bits where an unusual renderer needs lower-level composition. Avoid implementing the same control twice merely to keep both libraries visible.

The executable trial should compare one generated resource form/table and one curated user-management screen using shared controls. Include nullable/absent/value intent, a custom field, select-in-dialog keyboard behavior, validation focus and policy-driven removal. Review one upstream component update against a small local customization to make maintenance cost concrete. Lock the toolchain/dependencies. Verify Kit compatibility. Keep Skeleton as the focused alternative if its themes and packaged controls materially reduce that same work; do not run a broad library tournament without a failed acceptance criterion.
