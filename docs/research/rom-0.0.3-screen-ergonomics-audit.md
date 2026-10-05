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
