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
