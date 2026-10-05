# ROM 0.0.3 initial increment: independent UI review

Date: 2026-10-05. Baseline: `091836e`. Evidence: source review of the working changes and new files.
This review did not run tests. Test statements below describe inspected scenarios, not executed results.
Other agents were changing Settings tests during this review. Findings refer to the source locations inspected before those changes completed.

## Scope

This review covers Work presentation and recovery, Settings navigation, Resource labels, clipboard handling, and the shared right inspector.
It applies [the quality gates](../quality.md) and the [initial increment design](../../openspec/changes/improve-studio-ergonomics-0-0-3/design.md).
The semantic control catalog and final 0.0.3 artifact remain separate tasks.

## Findings

### P2: Field presentation labels do not reach accessible control names

Location: [FieldHost.svelte](../../studio/src/lib/renderers/FieldHost.svelte), lines 127 and 138–142; expanded controls also use machine names at lines 92 and 112.

A descriptor can label `smtp_host` as `Mail server`. The visible row uses `Mail server`, but the input keeps `smtp_host value`.
The group, expanded editor, and field action labels also use the machine name.
Thus the displayed label does not identify its control through the accessible name. Voice and screen-reader users receive inconsistent names.

Use one resolved presentation label for visible and accessible control text. Keep `field.name` for mutation keys and error paths.
Add a browser scenario with a label that differs from its machine name. Verify the textbox, expanded dialog, and field actions by that label.

### P2: Mobile inspector close restores focus to a different control

Locations: [ResourcePage.svelte](../../studio/src/lib/application/ResourcePage.svelte), line 334; [WorkPage.svelte](../../studio/src/lib/application/WorkPage.svelte), line 253.

Resource `Open` and Work `Inspect` buttons open the mobile inspector. Both close callbacks always focus the toolbar chevron.
They do not retain the button that opened the inspector. The OpenSpec scenario requires focus to return to the activating control.

Remember the actual opener for each activation. Use the chevron as a fallback when the opener no longer exists or cannot receive focus.
Add narrow-screen keyboard scenarios that open from a Resource row and a Work item, then close with Escape.
The inspected right-inspector tests check chevron focus, but do not cover these entry points.

## Reviewed guarantees

Work keeps the recovery request in component state while the inspector closes.
The retry path passes that same object to the client, preserving the key, expected version, operation, and evidence.
The new navigation callback reaches the App guard. Navigation handlers check the guard before changing page or Resource kind.
Known recovery results clear the pending request and require another inspection before a new recovery action.
This source review found no new request-identity defect in that path.

Settings selects only descriptors with explicit Settings classification. Its groups use the shared ResourcePage, ResourceForm, controller, and revision-checked mutation path.
The screen does not introduce a separate configuration write API or infer Settings from Resource names.
The inspected Settings test checks ordinary patch correspondence and draft retention after a conflict response.

ResourceSummary interpolates labels, titles, and exact identity as Svelte text. It does not render descriptor text as HTML.
The title resolver uses a declared field in the current projection and does not parse the identity.
The clipboard action writes the exact identity and provides a selectable-text fallback when copying fails.
Clipboard success, failure, and hostile-text behavior still need browser scenarios.

The inspector relocates one mounted editor between stable desktop and mobile hosts. Closing does not conditionally destroy that editor.
The inspected tests cover filter drafts, Resource drafts, viewport changes, and chevron focus.
The separate quick-filter and full-filter hosts reuse the same mounted FilterPanel.

## DRY and verification follow-up

The increment shares Resource title resolution, Settings grouping, the inspector shell, and the inspector toggle.
WorkDetails separates snapshot presentation from WorkPage request state.
Settings reuses ordinary forms instead of duplicating mutation or validation behavior.
The field-label finding exposes the remaining duplicated name-resolution policy; consolidate it while preserving wire keys.

Run the affected checks after fixes. Run the full local verifier and downstream fixtures before integration, as the quality gates require.
Do not treat this source review as release acceptance or as proof that the pending semantic controls are implemented.

## Fix disposition: 2026-10-05

The field-label fix keeps canonical names in descriptors, codecs, errors, and wire values.
`displayLabel` supplies accessible control names and nested display labels.
Custom renderer props expose the optional human label without changing codec identity.
The browser presentation test asserts the visible label and matching textbox name.

Resource and Work inspectors retain the actual row opener.
Opening focuses that button before entering the Sheet. Closing returns focus to the connected opener, with a chevron fallback.
Mobile browser scenarios exercise both row openers.

Work filters and recovery controls now live in focused components.
WorkPage retains orchestration, pending receipt identity, capabilities, and navigation guards.
No recovery request construction moved into the presentation components.
See the [initial increment evidence](evidence/rom-0.0.3/initial-increment/README.md) for executed checks and remaining limits.

## WebKit regression disposition

The first admitted WebKit suite found one failure after mobile Work closed and reopened through the inspector chevron.
The Sheet could remain open without receiving focus; Escape did not reliably close it.
`InspectorToggle` had overwritten the Tooltip trigger's composed click handler in its child Button.
The fix passes `onclick` to `Tooltip.Trigger` so Bits UI retains its internal tooltip-close handler.
No timeout or animation wait was added to the component.
The owner agent reproduced the immediate-Escape scenario and verified six WebKit repetitions after the fix.
The final focused test also asserts Sheet focus entry and passed six repetitions per engine.
Root runs the complete suite separately and records its output in the increment evidence.
