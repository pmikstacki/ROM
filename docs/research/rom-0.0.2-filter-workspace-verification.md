# Filter workspace verification

Date: 2026-10-04. Worktree: `release-query-index`.

The owner selected both quick filters and a Filters / Details sidebar. Both views use one mounted editor and one draft. Apply uses the current Resource kind. It does not use kind navigation to reset the selected Resource.

## Implemented behavior

The compact toolbar shows the applied query. The filter editor shows pending edits separately, including invalid input. The popover and sidebar move the same editor between stable hosts. They do not create independent copies of its input state.

The Details view retains its draft and captured revision across tab changes and layout changes. Apply re-reads the selected Resource with current authorization. A newer revision disables stale mutation submission. A denied read clears the selected projection.

The desktop inspector starts open. The mobile inspector starts closed. On mobile, Escape closes it and returns focus to the toolbar. A browser test checks Tab containment and draft retention.

Clear filters removes conditions from the draft. It does not execute a query. Discard edits restores the applied query. If metadata makes that query invalid, explicit discard resets the draft and explains the change. Apply remains a separate operation.

## Vendored composition

The fork retains an audited SVAR rule and tree composition subset. Its upstream commit is `1c581c3312c626c525ee64b8f94446a025fa141c`.

Original files, hashes, license and patch descriptions are in `studio/src/lib/filters/vendor/`. The emitted runtime inventory retains the original MIT license bytes. Negative notice tests reject missing ownership or a changed license profile.

ROM supplies the controls, typed values and native query translation. The fork does not use the upstream numeric parser, local evaluator, themes or AI hooks.

The translator preserves false, zero, empty strings, null, absence and exact integers. It rejects unsupported groups, operators and fields before execution. It also rejects accessor properties and invalid draft envelopes before it reads their members.

## Executed checks

| Check | Result | Scope |
| --- | --- | --- |
| Svelte check | 0 errors, 0 warnings | Current frontend source |
| Node tests | 117 passed | Client, controller, filters and runtime notices |
| Component build | Passed | Current test application assets |
| Browser components | 62 passed | Chromium and WebKit, 31 cases per engine |
| Production frontend build | Passed | Current frontend assets and emitted notices |
| OpenSpec | 10 items passed | Strict validation before final documentation update; repeated before commit |

The final command records are `ROM-filter-accepted-source-{type,node,component-build,components,production}.json` in `/root/ipi/research/disk-coordination-2026-10-04/`.

## Regression findings

The initial browser cases failed because the new views did not exist. Later tests and reviews found these defects:

- Separate editors lost invalid input when switching views. One mounted editor now preserves that input.
- Index keys reset a later rule after an earlier rule was removed. Stable private rule IDs now retain its input.
- Clear filters left errors for removed rules. Validation now counts only current rule IDs.
- A closed, mounted Dialog applied its default scroll lock. The inspector now locks scrolling only when its mobile Sheet is open.
- The mobile inspector opened automatically and blocked navigation. It now starts closed on mobile.
- Escape did not restore toolbar focus after a resize. An explicit focus callback now restores it.
- Invalid raw numeric input still showed Applied. The pending indicator now includes editor validation errors.
- Non-enumerable getters bypassed translator validation. Member validation now inspects all own keys before property reads.

Each behavioral regression has retained RED and GREEN evidence. Type checks also caught a missing callback prop declaration before acceptance.

## Design capture and limits

Actual layout captures are in `/root/ROM-design-screenshots/filter-workspace/`:

- `01-full-filters.png`
- `02-quick-filters.png`
- `03-details.png`
- `04-mobile-details.png`

These images use a real SQLite ROM host and a local human identity fixture. The native binary is from historical source `4ad726d4b1d34a834cdf90042b82bad67ccf9ce4`. The capture is design evidence, not acceptance of the new release binary.

The complete release producer, extracted consumer checks, current native browser workflows and durable preview remain pending. All eight release gates remain required. This report does not claim a published release or production readiness.

## Full-producer pagination follow-up

The producer for `9380941fbe5899aea133d6c4cd2c1fc9d81fe642` passed gates 1 through 6. Gate 7 stopped with 61 browser cases passed and one WebKit pagination case failed. Gate 8 did not run.

The requested page and its rows were correct. WebKit painted the page label, but its nested inline span had zero height. Five private-namespace attempts reproduced the failure. Font lookup succeeded; measured glyph height and font bounding height differed.

An explicit `inline-block` class gives the label a 16-pixel box. The original visibility assertion then passes. Expanded navigation captures are byte-identical before and after this CSS change. No query, anchor, test assertion or browser project changed.

A fresh component build and all 62 original browser cases passed with this change before formatter-only wrapping. The verification used a private 512 MiB temporary filesystem. The formatted source then passed Svelte check with zero errors and warnings. The next complete producer must verify the formatted committed source through all eight gates.

Evidence is under `/root/ipi/research/disk-coordination-2026-10-04/`: `pagination-webkit-diagnostic/`, `pagination-fresh-source-verification/`, `ROM-pagination-box-type-check.json` and `ROM-filter-producer-terminal.json`. The retained failed images remain available. Verified-zero compaction reclaimed 25,290,866,688 allocated bytes; their complete logical hashes remained equal.

## Native browser and collection follow-up

The `95bca07` producer passed gates 1 through 7. Gate 8 stopped with 16 native browser cases passed and eight failed. The external author did not run. The release remains unaccepted.

WebKit returned a zero-height box for a generic scalar span. An explicit inline box made the original visibility assertion pass in the diagnostic. Generic scalar and fallback displays now have explicit boxes. Empty codec-wrapped lists and maps have the same treatment. Nonempty collection layout remains unchanged.

A destructive alert description failed the original contrast assertion at 4.49:1. Its variant now uses the full destructive token, without reduced opacity.

A separate capture showed a closed Select with an active exit animation and a retained body pointer lock. Select now uses Bits UI's nonlocking default. Its departing floating wrapper becomes inert and hidden from accessibility queries. Both primitive prop spreads and their ref attachments remain intact. The exit animation remains present.

The popup helper selects the exact domain value from exactly one open listbox on the foreground page. This excludes an older closing popup. It does not force clicks, increase deadlines, or remove assertions. The original direct keyboard picker test remains unchanged.

Ten new component cases per engine cover scalar values, missing/null distinctions, empty collections, and nested mobile Select interactions. Both empty collection cases first failed the WebKit visibility assertion. Mobile tests use normal scrolling, clicks, Escape, Tab, and focus assertions.

Fresh assets passed all 24 native browser cases before the final inert-wrapper change. The complete Task and Inventory workflows took 12 to 14 seconds. This result uses the retained native binary and is diagnostic evidence, not acceptance of a new native release.

The final frontend passed Svelte check with no errors or warnings. All 82 component cases passed in a single-worker diagnostic. Default two-worker runs still failed at different WebKit Select interactions because their pages closed during actionability waits. Browser diagnostics showed graceful process exits, not an established browser crash. Five isolated picker repeats passed. These results do not establish the cause of the parallel failures.

The default suite, all eight release gates, external author, artifact verification, durable preview, and publication remain pending. No successful diagnostic substitutes for those requirements.

Evidence is in `/root/ipi/research/disk-coordination-2026-10-04/`: `native-pointer-diagnostic/`, `native-browser-fixes-final/`, `renderer-regressions-red/`, `renderer-regressions-final/`, `renderer-regressions-confirmation/`, `webkit-parallel-browser-diagnostic/`, and `webkit-worker-isolation-comparison/`. Registry provenance records preserve the original source hashes and identify the local Alert and Select adaptations.

Zero-only compaction of the failed producer images reclaimed 15,140,401,152 allocated bytes. Complete logical hashes, file sizes, inode identities, ownership, and modes remained unchanged. Fresh process, open-file, mapping, hardlink, and loop audits found no references or errors before compaction. The audit is `ROM-native-gate-owned-unwritten-image-compaction.json`. No source, evidence, or other project was deleted.

### Parallel lifecycle follow-up

The final component source passed all 82 cases with the unchanged two-worker profile on 2026-10-04.
This uninstrumented control completed in 40.9 seconds. Both browser projects and all original assertions remained enabled.
The source snapshot did not change during execution.
Evidence: `/root/ipi/research/disk-coordination-2026-10-04/webkit-parallel-control-followup/`.

A separate instrumented run also passed all 82 cases. It recorded monotonic timestamps for close calls and lifecycle events.
Independent review attributed observed closures to Playwright fixture teardown or Axe auxiliary-page cleanup.
No observed unsolicited closure, crash, capture cap or observer error occurred in that run.
Instrumentation adds scheduling overhead. This passing run does not establish the cause of earlier intermittent failures.
Evidence: `/root/ipi/research/disk-coordination-2026-10-04/webkit-lifecycle-capture/` and `webkit-lifecycle-instrumentation-review/capture-52215-analysis.md` in the same coordination directory.

These results permit continued candidate verification. They do not prove full clean-source release acceptance or preview deployment.
The approved quick-filter popover and Filters / Details sidebar remain unchanged.

### Current preview host diagnostic

The final frontend source at `0c40007995ada63f8e3304593ccbacc9deaca4c8` was built into the temporary Studio preview assets.
An actual host browser run passed 24 of 24 scenarios across Chromium, WebKit, SQLite, and redb.
It covered human login, generic Resource workflows, attachments, shutdown, bounded frames, and two-tab authority changes.
The run used the maintained existing native binary with SHA-256 `c93a20cfc1d92cc6e3e6f613105bbbcc61468179008fc118fafabb994d1c88f3`.
The external evidence is `ROM-current-preview-host-runtime-audit.json` and `ROM-current-preview-host-runtime.log` under `/root/ipi/research/disk-coordination-2026-10-04/`.
This diagnostic does not satisfy gate 8, which requires a fresh source build and extracted release assets.
