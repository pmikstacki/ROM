# Studio draft, validation focus and keyboard acceptance

Date: 2026-10-04 UTC. Source context: `ed7755791d10fcff4cc81afb1a2c0d9be7733dce` plus the two owned test files.

Three new browser scenarios pass in Chromium and WebKit. The complete affected component suite passes 42 cases.
No product defect was reproduced. No product source, native build, dependency or lockfile changed in this task.

## Executed scenarios

The [control fixture](../../studio/tests/components/Harness.svelte) mounts the actual shared ResourceForm, renderers, table and action/query controls.
The [tests](../../studio/tests/components/controls.spec.ts) use real browser controls and the existing local component build.
Their submission callback records serialized input. It does not call a native backend or claim a committed Resource mutation.

| Scenario | Assertions |
| --- | --- |
| Descriptor version changes with an open draft | Integer and text drafts survive a version increment. The form announces the change, disables editing and disables submission. No submission is recorded. |
| Explicit reconciliation by reopening | The fixture caller deliberately discards the draft and remounts the form under the current descriptor. Enter activates that explicit control. Operations reset to omitted. New keyboard-submitted input uses only the newly entered value. |
| Invalid integer and error focus | Keyboard input exceeds u64. The same field retains focus and its typed text. `aria-invalid` and the visible error identify rejection. Enter records no submission. Keyboard correction clears the error; Enter then submits the exact corrected integer. |
| Native keyboard semantics | ArrowDown changes the operation select. Tab reaches the newly available checkbox. Space toggles it. Enter activates submission and preserves boolean true in the shared wire serializer. |

Reopening is an explicit caller-owned operation. The fixture uses a keyed remount of the existing ResourceForm.
The test does not add an automatic draft reset or a new production reconciliation API.
It isolates descriptor-version changes from the earlier Resource row-revision conflict scenario.

## Failure-first evidence and checks

The first focused run had six cases: four passed and two failed.
The focus and keyboard scenarios already passed in both engines.
The descriptor tests could not find the fixture's version-advance button.
The [initial log](evidence/rom-0.0.2/draft-acceptance/initial-browser.log) retains that expected fixture failure.
This is not reported as a production RED/GREEN defect.

The fixture then gained mutable descriptor version and explicit keyed reopening controls.
Production ResourceForm and ValueEditor behavior remained unchanged.
Both fixture files passed formatting after the final browser run; formatting changed neither file.

| Command | Result |
| --- | --- |
| `npm run check` | Zero errors and warnings. [Log](evidence/rom-0.0.2/draft-acceptance/typecheck.log). |
| `npm run build:components` | Exit 0. [Log](evidence/rom-0.0.2/draft-acceptance/component-build-final.log). |
| `ROM_WEBKIT_EXECUTABLE=/var/tmp/rom-studio-webkit-2359/pw_run.sh ./node_modules/.bin/playwright test --config tests/components/playwright.config.ts` | 42 passed, no failures or skips. [Log](evidence/rom-0.0.2/draft-acceptance/browser-final.log). |
| Prettier with `prettier-plugin-svelte`, owned files | Passed; final write reported both files unchanged. [Check log](evidence/rom-0.0.2/draft-acceptance/format.log). |
| Owned `git diff --check` | Passed. |

The commands ran from the maintained `studio` directory on the NixOS host with Node 22.16.0.
The existing pinned WebKit wrapper was reused. No native executable or browser runtime was rebuilt.
The [source record](evidence/rom-0.0.2/draft-acceptance/source.json) includes both owned files, relevant Studio inputs and frontend lock identity.
It does not claim a full repository identity or the coordinator's final clean release gate.

## Acceptance limits

These tests close the recorded component descriptor-change scenario and add focused error-focus and keyboard evidence.
Error focus means retained focus at the invalid input. Automatic focus transfer from a server error is not tested or promised here.
The suite also reruns its existing assembled-control axe checks in both engines.
Those scans do not certify accessibility, human usability, or the complete connected application.
Actual host mutations, live membership, external author consumption and final release artifacts retain their separate acceptance gates.
