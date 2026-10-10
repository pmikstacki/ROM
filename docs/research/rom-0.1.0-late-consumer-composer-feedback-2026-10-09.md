# Late consumer panel and composer feedback

Date: 2026-10-09. Scope: selected source review and proposed acceptance. This report closes no research, usability or release task.

The late Astral Plane feedback asks for a broader AI panel and a visible composer. The reported response adds width and expansion controls. Selected current source contains these changes. ROM's public controls can support this composition, but the combined panel, history and composer geometry still needs installed-consumer acceptance.

## Evidence boundary

The coordinator read the latest Astral Plane thread, identified by prefix `01a111a5`, with reported completion timestamp `1791478749`. The thread reports deployed changes and 191 tests. This investigation did not run those tests, visit the deployment or observe a human using it. That count remains a consumer report.

The retained [source review](/root/ROM/.superpowers/rom-010-native-copy-preparation-20261009/late-consumer-ui-source-review.json) records consumer HEAD `71fb093608d05bcf4edd6ecdf1381540166797c3` and rom-ui HEAD `f642a03c948c8be66beeb1b70e43289e02a80189`. The selected files were read directly. Their identities are:

| Source | SHA256 |
| --- | --- |
| `/root/astral-plane/web/src/SidePanel.svelte` | `828d26c1dc148f20e172e600282e11e04df01b2b5705f39e4c5ed778830a128b` |
| `/root/astral-plane/web/src/ChartChatPanel.svelte` | `fdb7301b582071b78ff55626ee9a05ae859a897f8989ecc460f8ef553608e1a8` |
| `/root/rom-ui/src/lib/ui/components/ResponsiveDetails.svelte` | `c8487cf65a9aa93f285c15edc7a3199559b6f336dcd5db4798b4dbd06e25ed92` |
| `/root/rom-ui/src/lib/ui/components/HistoryList.svelte` | `88a61a1d872cff62c8283a9ccb33990c1be93783210c404942cdedf5059f01e6` |

No browser, build, provider or application workload ran for this report. R11's frozen sequence, native binaries and original evidence were not changed.

## Current composition and responsibility

[SidePanel](/root/astral-plane/web/src/SidePanel.svelte) sets contained desktop width to 560px and expanded width to 840px. Its desktop expansion button has a changing accessible label and `aria-pressed`. Mobile uses a modal dialog; desktop uses a nonmodal dialog. Expansion changes CSS around one rendered child snippet. This suggests preserved child state, but source inspection does not prove focus or mount behavior.

[ChartChatPanel](/root/astral-plane/web/src/ChartChatPanel.svelte) creates a flex column through application descendant selectors. Conversation content can grow and scroll. The form cannot shrink, and the textarea has a bounded height. History, privacy text and suggestions have separate limits. Short-height rules reduce headers and history height. Actual composer visibility depends on their combined geometry.

These selectors include `.chart-questions`, `.sessions`, `.conversation`, `form` and `.history-list`. They are application composition assumptions. They are not established public ROM layout contracts. In particular, the inspected shared HistoryList root does not declare a `.history-list` class.

[ResponsiveDetails](/root/rom-ui/src/lib/ui/components/ResponsiveDetails.svelte) supplies responsive region/Sheet behavior, labels, explicit opener/fallback focus, reduced-motion handling and scroll locking. Its [preserveEditor helper](/root/rom-ui/src/lib/ui/components/preserve-editor.ts) relocates one mounted editor node between stable hosts. It does not expose a desktop expansion control or a distinct fixed composer/footer surface.

[HistoryList](/root/rom-ui/src/lib/ui/components/HistoryList.svelte) supplies bounded history rows, current selection, localized dates/counts and pending-selection feedback. Its epoch and authority-token checks suppress stale selection errors. The caller still owns authorized history data and the `onSelect` operation. This component does not itself supply a draft barrier or durable mutation recovery.

The inspected ROM [public facade](/root/ROM/studio/src/ui-components.ts) exports ResponsiveDetails and HistoryList from `rom-ui/ui/components`. The earlier [UI gap review](rom-0.1.0-ui-feedback-gap-review.md) described HistoryList as absent at that review point. Current selected source changes that implementation observation. It does not retroactively establish installed acceptance or close AP-UX-022.

The remaining generic gap is a documented public composition for a scrollable conversation beside a stable composer and accessible expansion control. Start with an installed recipe using the existing exports. Add a shared layout seam only if the recipe needs private DOM selectors. Keep conversation generation, Human Design calculation, terminology and result correctness in the application domain. They require application evidence, not ROM core changes.

## Bounded next acceptance

Use a separate locked consumer installed from the selected public package. Record its package, lock, compiler and browser identities. Import controls only through public exports. Preserve the original AP-UX-003, AP-UX-009 and AP-UX-022 requirements from the earlier gap review.

Run the composition in Chromium and WebKit at 1280x900, 390x844 and 390x500. Include long history titles, an empty history, multiline drafts, long answers and delayed history selection.

| Check | Required observable result |
| --- | --- |
| Expand and collapse | The control announces its state. The same editor instance, draft, selection and pending request survive. |
| Composer geometry | The composer and send control remain reachable without scrolling the conversation to its end. Long content does not overlap them. |
| History geometry | Current, empty and pending states remain readable. History scrolls within its allocated region without hiding the composer. |
| Focus and close | Expansion preserves appropriate focus. Escape and close return focus to a valid opener or documented fallback. |
| Responsive transition | Crossing the breakpoint preserves the editor instance and draft. Inactive surfaces do not retain a scroll lock. |
| Short viewport | Keyboard navigation reaches history, composer and close controls. Privacy text and suggestions do not consume the entire available height. |
| Authority change | Pending history selection cannot publish stale status or content after the owner changes. Application mutation barriers remain explicit. |

Count mounts and mutation submissions in the fixture. Capture geometry and focus assertions. Keep those results separate from original-consumer human feedback and the reported 191 tests. An external package test can establish its selected composition; it cannot establish calculation correctness or deployed consumer usability.

## Additional ROM source review

A later review found an existing public `ConversationLayout` in ROM, beyond the four selected files above.
Its facade exports header, history, body and footer snippets with bounded regions and optional desktop expansion.
The [composition guide](../studio-compositions.md#conversation-layout) documents its public import and width properties.
The [source evidence](/root/ROM/.superpowers/rom-010-native-copy-preparation-20261009/late-consumer-existing-layout-review.json) binds the five inspected files.

The installed-consumer source already uses this component through `rom-studio/ui/components`.
Its existing test source covers 1280x900 and 390x500 geometry, snippet identity, keyboard scrolling, expansion focus and composer submission.
This review did not run those browser tests.
The source does not cover 390x844 or the combined details, history, pending-selection and authority-change scenario described above.

Use this existing layout for the next acceptance fixture.
Do not add another layout abstraction before testing its public composition with `ResponsiveDetails` and `HistoryList`.
The remaining gap is combined installed acceptance and consumer integration, not an absent ROM conversation layout.
No AP-UX task or release gate is closed by this source review.
