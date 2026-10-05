# ROM 0.0.3: Studio screen ergonomics audit

Date: 2026-10-05. Status: source observations and proposed acceptance scenarios.

The audit covers the current login, navigation, Resource, Work, attachment, and error screens.
No browser tests were run for this audit. Recommendations below are proposed behavior.
Source observations describe the baseline inspected before the approved inspector and Work implementation.

## Screen inventory and observations

| Screen | Source observation | Proposed change |
| --- | --- | --- |
| Login | Provider buttons, Connect, auth errors, and connecting status exist in `App.svelte`. The footer exposes implementation language. | Use task language for sign-in, loading, unavailable providers, and reconnect. Keep provider labels readable and keyboard accessible. |
| Navigation | Sidebar separates workspace pages and Resource kinds. Breadcrumbs show the machine kind. Mobile close restores focus to its trigger. | Use declared Resource labels throughout navigation. Preserve active-page indication and focus return. |
| Resources | Toolbar has quick filters, details panel, refresh, live observation, create, applied-query badges, and pagination. | Retain quick filters. Make the right inspector toggle consistent across pages. Use readable query summaries and Resource titles. |
| Resource details | Heading renders `Resource` plus exact ID. Revision, stale-draft notice, patch form, actions, and confirmed deletion exist. | Show authorized title and type first. Put exact ID and copy action below. Group edit, action, and deletion controls. |
| Work | Capabilities and list require separate buttons. Rows show handle, category, and serialized state. Selection shows raw JSON. | Load authorized capabilities and the first page. Show compact definition, status, attempts, and scheduling context. Put structured details in the right inspector. |
| Attachments | Upload form precedes a generic Resource table. Selection and results use in-flow cards. | Keep upload as an explicit operation. Move selected attachment details to the shared inspector. Show readable limits and operation states. |
| Errors | Global alerts, local alerts, form summaries, field errors, and unknown-outcome notices exist. | Distinguish field errors, request failure, conflict, unavailable capability, and unknown outcome. Keep the relevant draft and recovery action visible. |

Observed sources:
[App](../../studio/src/App.svelte),
[shell](../../studio/src/lib/presentation/StudioShell.svelte),
[navigation](../../studio/src/lib/presentation/StudioNavigation.svelte),
[Resource page](../../studio/src/lib/application/ResourcePage.svelte),
[Resource details](../../studio/src/lib/application/ResourceDetails.svelte),
[Resource table](../../studio/src/lib/resources/ResourceTable.svelte),
[Work page](../../studio/src/lib/application/WorkPage.svelte),
[attachment page](../../studio/src/lib/application/AttachmentPage.svelte),
[attachment panel](../../studio/src/lib/attachments/AttachmentPanel.svelte).

## Unified right inspector

Resources already use [ResponsiveInspector](../../studio/src/lib/application/ResponsiveInspector.svelte).
It displays an inline right panel above the shared 1279-pixel breakpoint and a right Sheet below it.
The Sheet's default side is right. The inline panel has no explicit collapse control in the inspected source.
Its accessibility labels and Sheet description refer to Resource tools, even when its title changes.

Use one inspector pattern for Resources, Work, and selected attachments.
Place a chevron at the right edge of the workspace toolbar or collapsed panel rail.
The closed-state chevron points left, indicating that the panel enters from the right.
The open-state chevron points right, indicating collapse toward the right edge.
Use explicit accessible labels: Open details panel and Close details panel.
Publish expanded state and the controlled panel ID.

On desktop, opening the inspector reduces the content area without moving it below the list.
On narrow screens, use the right Sheet with a clear title, visible close control, focus containment, and focus return.
Closing the inspector must not submit, reset, or discard a draft.
Moving between inline and Sheet hosts must preserve the same mounted editor.
The existing `preserveEditor` mechanism is relevant to this requirement.

Retain the quick-filter popover in the Resource toolbar.
The popover and inspector must share one draft and one applied query.
Open full filters moves the editor into the inspector without changing the draft.
Closing either surface must not apply pending conditions.

## Work information design

The [frontend validator](../../studio/src/lib/client/work-validation.ts) and
[backend protocol](../../crates/rom/src/operator/protocol.rs) expose a current Work view.
It includes handle, generation/revision, category, definition name/version, state, attempts, due, delivery, source, and target.
Supported list filters include state, category, definition, limit, and cursor.
The current Work page requests only `limit: 50` and does not expose those filters or continuation.

Use definition name as the main row label.
Show category and state as separate readable text.
Show attempts and due as compact secondary context.
Keep the exact handle copyable and accessible, while avoiding its use as the main heading.
Do not format `due` as a wall-clock date until its clock and unit contract is confirmed.
Keep a technical-details disclosure for exact version and wire values.

Use Filters and Details tabs in the shared inspector.
Work filters must use the existing operator query fields.
Keep draft filter state separate from applied state.
If a filter would hide the selected item, keep the selection behavior explicit and predictable.
Use cursor continuation from the returned page rather than inferred page numbers or totals.

Show source and target as authorized Resource references when present.
Readable labels must use authorized projections and the shared title contract.
An absent reference is not an error.
Do not perform unrestricted reads to obtain labels.

The current contract does not expose an event history or attempt timestamps.
Do not infer a timeline from attempts, state, delivery, or due.
A snapshot can say Current status or Scheduling details.
A future timeline needs an actual authorized history source with defined ordering, timestamps, and pagination.

## Recovery safety and draft preservation

Work currently requires confirmation before Retry or Reconcile.
The request includes handle, expected version, a generated key, retry epoch, and operation.
An unresolved transport result keeps the pending request for Retry same recovery request.
The selected Work object and pending request are local component state.
Switching away from Work unmounts that state in `App.svelte`.

Keep confirmation associated with the exact selected handle and version.
Reset confirmation when selection changes.
Do not let an icon click, row selection, filter change, or Enter in a filter invoke recovery.
Keep evidence references separate from general filters.
Explain which selected work item the operation affects before confirmation.

Preserve a pending unknown-outcome request across inspector close and layout changes.
Before allowing navigation to discard it, define an application-level retention or explicit navigation guard.
Never regenerate its idempotency key during a retry.
Do not present Scheduled as completed execution.
Do not present Unresolved as confirmed failure.

Resource drafts currently live in forms keyed by selected ID.
Work, Resource, and attachment navigation need an explicit dirty-draft policy.
Closing a panel is a reversible display action and should preserve the draft.
Changing selected identity requires preservation or an explicit discard choice.
Session changes must clear unauthorized values without submitting stale work.

## Proposed E2E acceptance scenarios

These scenarios are test requirements, not executed results.

| Scenario | Required assertion |
| --- | --- |
| Right panel toggle | At desktop width, the chevron opens the panel on the right. Toggle labels and expanded state match visibility. Closing restores the list width. |
| Mobile panel focus | At 390 pixels, keyboard focus enters the Sheet, stays inside, and returns to its trigger after Escape or Close. |
| Breakpoint with draft | Enter a Resource value. Resize through 1279 pixels. The value, intent, validation error, and focused control survive. |
| Quick/full filter transfer | Add an unapplied condition in the popover. Open full filters. The same condition remains and no request occurs until Apply. |
| Dirty close | Edit a Resource, close the inspector, and reopen it. The draft remains. No mutation was sent. |
| Dirty identity change | Edit one Resource and select another. Assert the defined keep/discard behavior and prevent silent loss. |
| Human title and long ID | Use an authorized title and a long opaque ID. Title and type remain readable. Copy returns the exact ID. No JSON parsing changes identity. |
| Withheld title | Withhold the title field. Heading and reference picker use the safe fallback without fetching or displaying withheld text. |
| Enum labels | Select a readable enum label. Submit the exact descriptor value. Inspect table and detail labels for agreement. |
| Nested list focus | Edit a row, move it, and remove another row. Draft value order and focused item remain correct. Keyboard movement works without dragging. |
| Work loading | Enter Work with inspect permission. Load once per valid page lifecycle. Refresh does not create duplicate requests or a render loop. |
| Work filters | Apply supported state/category/definition filters. Assert exact operator query and cursor reset. Unapplied drafts do not change results. |
| Work snapshot | Show real definition, state, attempts, due, and delivery. No fabricated timeline, dates, execution count, or completion time appears. |
| Work confirmation binding | Confirm recovery for one handle, then inspect another. Recovery is disabled until the new item is confirmed. |
| Work unknown outcome | Drop the control response after submission. Retry the same request. Assert identical handle, expected version, key, retry epoch, operation, and evidence. |
| Work conflict | Change the version before control. Show conflict and require fresh inspection. Do not automatically regenerate and submit recovery. |
| Work capability denial | Disable retry or reconciliation. Controls are unavailable. No hidden control request is sent. |
| Work navigation safety | Navigate away with an unknown control result. Assert the accepted retention or guard policy; returning cannot silently create a new request. |
| Attachment selection | Select an attachment, confirm detachment, then select another. Confirmation resets. The selected exact ID matches the request. |
| Attachment unknown outcome | Lose upload acknowledgement. Retry preserves reservation key, Resource ID, and file contents. Closing details does not discard pending state. |
| Session expiration | Expire the session with an open inspector. Remove authorized values and prevent old drafts or recoveries from submitting under a new session. |
| Errors and keyboard | Trigger a field error and a request error. Each has readable text and suitable announcement. The user can return to the relevant control. |
| Narrow and zoomed layout | At 390 pixels and 200% zoom, controls, errors, long labels, and pending notices remain operable without page-wide horizontal overflow. |
| Empty states | Distinguish no matching results, unavailable capability, no selected item, loading, and request failure. Do not show an empty list as successful recovery. |

Prioritize inspector consistency, readable titles, Work structure, and recovery retention before adding visual history.
Capture browser evidence for the acceptance scenarios after implementation.
Source inspection alone does not establish accessibility, mobile usability, or recovery safety.

## Executed login and attachment increment

Date: 2026-10-05. The baseline observations above remain historical source inspection.
This increment changes login and attachment presentation. It does not certify the whole Studio screen inventory.

Login now uses the extracted `LoginPage` component with the existing authentication flow.
Provider links retain their exact routes. The primary provider retains its automatic redirect and explicit fallback choice.
Long provider labels wrap on narrow screens. Session checking and unavailable providers use readable status text.
An expired session removes authorized Resource values and provides a sign-in path.
The explicit expired-authenticated-response case uses `SessionExpiredError`; it does not match error strings to establish authority.

Attachments now use the shared right inspector on desktop and mobile.
A title comes from the authorized projection. The exact opaque ID remains visible and copyable.
Detachment confirmation resets on selection change and before submission. The request uses the exact selected ID.
Loading, unavailable capability, query failure, empty results, committed upload, and unknown outcome remain distinct states.
Unknown reservation and upload responses retain the controller's frozen reservation, ID, key, and file contents.
The application blocks ordinary navigation while an attachment operation is pending or unknown.
Closing the inspector does not clear the pending operation or upload draft.

### Failed regressions and fixes

The first focused attachment regression reproduced the missing right inspector before the presentation change.
A separate explicit expired-session regression reproduced the missing sign-in explanation and provider choice.
The application owner repaired that session path while preserving the provider redirect policy.

A narrow-screen check at 390 pixels with 200% CSS zoom exposed attachment page overflow.
The document width was 508 pixels against a 390-pixel viewport.
The attachment header, form grid, store selector minimum width, and upload button caused the overflow.
The local fix lets actions wrap and lets form controls shrink within a zero-minimum grid track.
The same regression then measured a 390-pixel document width.
The Resource table retains its own horizontal scroll area.

### Focused executed cases

The isolated Chromium development-server run passed eleven focused tests before the added zoom assertions.
The later narrow login and attachment zoom checks passed after the attachment fix.
The Studio type check passed with zero errors and zero warnings.
These tests use the real application, SDK, and components with deterministic HTTP route fixtures.
They do not establish a real identity-provider exchange or a real object-store upload.

| Screen | Executed positive and error cases |
| --- | --- |
| Login | Long provider label at 390 pixels; exact login route; keyboard focus; unavailable session retry; primary redirect and fallback choice. |
| Login session changes | Authenticated session becomes unauthenticated; expired authenticated response; protected table values disappear; sign-in remains available. |
| Attachment details | Desktop right inspector; authorized human title; long opaque ID; exact clipboard copy; confirmation resets when selection changes. |
| Attachment mobile | Right Sheet contains focus; Escape returns focus to the selected row; 390-pixel layout and 200% CSS zoom avoid page overflow. |
| Attachment recovery | Lost reservation response repeats the same reservation and key; lost upload response repeats identical ID and bytes; pending navigation is blocked. |
| Attachment errors | Denied capability shows unavailable state without an upload form or successful empty list; query denial stays distinct from empty results; refresh restores rows. |

Focused commands:

```sh
cd studio
npm run check
npx playwright test --config /tmp/rom-ergonomics-playwright.config.mjs
```

The temporary development configuration used Chromium and `http://127.0.0.1:43326/rom-studio/`.
It selected `login-ergonomics.spec.ts` and `attachments-ergonomics.spec.ts` without changing the shared component build.
The release owner separately coordinates the complete component-bundle Chromium and WebKit run.
Its results must be recorded separately; this focused record does not claim they passed.

### Remaining audit scope

Resource editing, Settings, Work recovery, relation selection, and sortable lists are separate concurrent work.
The full combined screen audit remains incomplete until those executed results and the packaged entry point are reviewed together.
Actual browser zoom controls, a human usability review, and a real provider/object-store journey remain outside this focused test evidence.
The current attachment query returns at most 50 rows. This increment does not invent totals or continuation metadata.

### Later combined browser evidence

The later rebuilt component bundle passed the selected integration matrix on Chromium and WebKit.
[`integration-browser-latest.log`](../../prototypes/studio-controls-003/integration-browser-latest.log) records 81 passes and one explicit WebKit touch skip.
[`integration-build-latest.log`](../../prototypes/studio-controls-003/integration-build-latest.log) records the fresh bundle build.
All eleven login and attachment tests passed in both engines.
The skipped case belongs to the sortable-list touch fixture, not the login or attachment tests.

The attachment upload fixture uses an actual local HTTP receiver.
It collects the browser's streamed Blob bytes and asserts their exact contents.
The first upload response destroys its socket to produce a lost acknowledgement.
The retry retains the reservation ID, key, selected file, and identical bytes.
The fixture redirects only the upload fetch destination and supplies CORS preflight responses.
It does not substitute an echoed fixture body for browser upload evidence.

The earlier complete component run exposed three WebKit fixture failures.
WebKit rejected the unsupported clipboard permission and supplied no intercepted binary request body.
The fixture now uses supported clipboard permissions and the actual upload receiver.
Exact ID clipboard reads and exact upload byte assertions passed in both engines after that repair.
These local results do not establish a real external identity-provider exchange or production object-store behavior.

## Frozen 0.0.3 component acceptance

The final combined component run records 219 passes and one explicit WebKit physical-touch skip.
Both Chromium and WebKit used the same rebuilt component assets. All 146 unit tests passed.
Svelte check reported zero errors and warnings.
Evidence: [browser log](../../prototypes/studio-controls-003/review-final-browser.log),
[unit log](../../prototypes/studio-controls-003/review-final-unit.log), and
[source identity](../../prototypes/studio-controls-003/review-final-maintained-source-identity.json).

This run includes invalid drafts across expanded-editor unmounts and complete read-only semantic disclosure.
The [independent spec review](rom-0.0.3-final-spec-review.md) records the earlier findings and their corrections.
These component results do not replace actual-host, clean-source artifact, or persistent-preview acceptance.
