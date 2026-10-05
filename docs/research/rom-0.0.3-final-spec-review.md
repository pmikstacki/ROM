# ROM 0.0.3 independent specification review

Reviewed on 2026-10-05 against foundation `6d3b6b2` and the dirty release source.
This review uses source inspection and existing test definitions. The reviewer did not run builds or browser tests.

## Executive review

The implementation follows the Resource premise. Presentation and enum labels remain outside persisted schema identity. Authorized discovery supplies shared forms, Settings groups, human titles, and reference choices. Standard codecs provide backend validation; browser controls preserve exact decimal and JSON text. The inspector uses a right-edge chevron and shares its shell with Work and Attachments. Work uses current snapshots rather than an invented history.

Two ergonomic defects remain in the inspected source. First, closing an expanded collection editor discards its local invalid draft. Parent validation can then remain active without the draft that explains it. Second, read-only semantic details use the cell preview truncation and provide no full-value disclosure. Long stored values cannot be inspected completely.

These findings require focused browser regressions and corrections. They are implementation gaps, separate from the known release gates. Actual SQLite/redb browser journeys, packaged custom renderer acceptance, clean-source artifact verification, README screenshots, and persistent preview deployment remain pending. The existing foundation commit and component results do not complete release acceptance.

No application-specific controller or inappropriate remote-code loading was found in the reviewed paths. The finite semantic catalog and explicit custom composition are consistent with the specification.

## Findings

### P2: Expanded editors do not retain invalid drafts when closed

Source: `studio/src/lib/renderers/FieldHost.svelte:108` and `:119`.
The expanded `ValueEditor` is inside ordinary `Dialog.Content`. The content is not force-mounted, and FieldHost does not own or bind an `EditorDraft`.
`ValueEditor.svelte:23` creates its draft locally. `ResourceForm.svelte:112` retains validation errors in the parent; `:140` disables submission when an error remains.

Example: enter invalid integer text in a list row, close the dialog, and reopen it.
The invalid text and row-local error state are lost with the editor instance. The last canonical candidate remains, while the parent error can still block Save.
Valid edits surviving through the parent wire candidate do not establish invalid-draft retention.

The existing test `closing a keyboard drag releases the action and preserves the form candidate` covers a valid reordered list. It does not cover this case.

Required correction: retain the draft above the dialog content lifecycle. Reset it only for an explicit operation-mode reset. Add Chromium and WebKit regressions for invalid scalar and codec rows across close/reopen.

### P2: Read-only semantic details cannot disclose complete stored values

Source: `studio/src/lib/renderers/SemanticField.svelte:140`–`:155`.
The read-only/detail branch limits visible text to 160 characters and the hover title to 512 characters. It has no expansion or copy control.
`ValueDisplay.svelte` sends both cell and detail contexts through this renderer. Stale or blocked Resource forms also use its read-only branch.

Example: a stored JSON document or multiline value exceeds 512 characters. A user can neither inspect nor copy its complete value through this branch. Cell preview limits are appropriate, but detail inspection needs a bounded full-value path.

Required correction: keep compact cell previews. Provide an explicit read-only disclosure for the complete supported value, with bounded scrolling and exact-text copy. Add a regression for a long JSON numeric token and multiline text. Do not parse JSON into floating-point values to display it.

## Requirement assessment

| Requirement | Source assessment |
| --- | --- |
| Bounded generic descriptors | Registration validates presentation and enum labels. Discovery projects only disclosed metadata and charges the full response budget before cloning. Shared Rust/TypeScript fixtures exist. |
| Human titles and exact identity | The shared resolver reads only disclosed title fields; exact IDs remain separate. It does not infer names by parsing IDs. |
| Shared right inspector | Chevron toggle, desktop aside, and narrow right Sheet share one preserved child tree. Quick filters remain separate. Expanded field dialogs still have the draft gap above. |
| Work and recovery | Definition, category, status, and attempts precede the handle. Pending recovery retains its request and blocks ordinary navigation. No fabricated timeline was found. |
| Settings and plugin groups | Settings classification selects ordinary Resources and reuses ResourcePage, forms, revisions, and mutation paths. It is not a separate privileged configuration API. |
| Semantic controls | Ten versioned semantic leaves, enum labels, ordered multi-choice lists, map/list recursion, and protected unknown codecs exist. Read-only details need the correction above. |
| Relation choices | Choices use the session-owned lookup, current disclosed kinds, abort/epoch checks, bounded candidate pages, and exact IDs. Candidate filtering is correctly described as local to that page. |
| Browser ergonomics | Tests cover right-panel focus, retained inspector drafts, login, attachments, reference choices, enum labels, and sortable rows. The two cases above are missing. |

## Release gates, not new implementation defects

The task checklist still records final catalog acceptance, screenshots, and artifact/preview delivery as open.
Actual-host semantic tests are being added in `studio/tests/runtime/semantic-workflow.ts`; their source alone is not execution evidence.
Custom renderer registration is explicit in demo and extracted consumer entry points. Final packaged acceptance must confirm the intended assets actually contain that registration.
Update versions, migration notes, README, and checked task states only against final source-bound evidence.
Keep the release goal active until these gates pass.

## Follow-up disposition

A source re-review on 2026-10-05 closes both implementation findings above.
The earlier executive review describes the first inspected snapshot; this disposition supersedes its open-defect status.
The reviewer did not run new builds or browser tests.

### Expanded draft finding: corrected

FieldHost now owns `editorDraft` outside the dialog content. Both expanded and inline framework editors bind that same draft.
An explicit operation-mode change resets the draft; closing the dialog does not reset it.
The ownership also retains list-row identities, map child drafts, and codec draft callbacks across content unmounts.

Three new browser regressions close and reopen invalid integer-list, decimal-codec-list, and prototype-shaped map drafts.
Each test confirms the dialog unmounted, the invalid text returns, and Save remains disabled for the visible invalid candidate.
Existing logs record three failing Chromium regressions before correction, followed by 18 passing Chromium cases.
The WebKit log records 17 passing cases and one unrelated physical-touch skip.
See `prototypes/studio-controls-003/dialog-draft-red.log`, `dialog-draft-green.log`, and `dialog-draft-webkit-green.log`.
These are inspected execution records from the implementing agent, not independent reviewer executions.

### Read-only disclosure finding: corrected

SemanticField retains the compact preview for cells. Other compatible read-only/detail contexts now provide `View full` for long values.
The popover contains the complete `formatted` string in a read-only, selectable Textarea with bounded viewport height.
The value is not reparsed into floating-point numbers. Selection and copying use the existing browser text control.

The new browser regression checks a complete JSON document containing the exact token `9007199254740993` and text beyond the preview limit.
It verifies the read-only attribute and keyboard close. Source inspection confirms multiline values use the same disclosure path.
The root reported eight passing semantic scenarios across both engines; that report was not independently executed by this reviewer.

No additional blocker was found in these two corrected paths.
The remaining actual-host, clean-source artifact, packaged renderer, screenshot, and persistent-preview gates still apply.
Version updates alone do not establish release completion.
