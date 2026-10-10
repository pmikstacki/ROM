# Selection and layout composition evidence

Date: 2026-10-08. Scope: Task 3B source-authoring candidate. Installed public-consumer, original consumer and release acceptance remain open.

`SelectionCard` renders a native button with host-controlled selected state. Its action snippet is a sibling surface. `LayoutControls` exposes native move, resize and visibility buttons. It proposes bounded detached layouts through the existing validator. Neither component persists, dispatches backend mutations or changes confirmed host state.

## Public contracts

The coordinator owns the public facade and Vite input. Both components are exported through `rom-studio/ui/components`. The independent fixture imports that entry without application context.

`SelectionCard` requires `id`, `title`, `selected`, `onToggle`, `authorityToken`, `failedLabel`, `unknownLabel` and `children`. Optional props are `actions`, `disabled` and `recoveryToken`. The latter defaults to zero. Child content must be noninteractive native-button content. Put independent controls in `actions`; the component renders them outside the selection button.

`LayoutControls` requires `layout`, `options`, `label`, `itemLabel`, `labels`, `onChange` and `authorityToken`. Optional props are `disabled` and `recoveryToken`. Layout options use the existing catalog/column/row/item contract. Labels provide `left`, `right`, `up`, `down`, `wider`, `narrower`, `taller`, `shorter`, `show`, `hide`, `invalid`, `failed` and `unknown` strings.

`onToggle(id)` and `onChange(command, proposal)` return an explicit `CompositionCommandResult`, synchronously or asynchronously:

| Result | UI behavior |
| --- | --- |
| `accepted` | Release pending state. Render only the host's confirmed selection or layout. |
| `rejected` | Release pending state and display the host's failure label. |
| `unknown` | Display the host's unknown label and retain the command barrier. |
| Thrown/rejected callback or unclassified runtime value | Treat the outcome as unknown. Suppress raw exception text. |

These classifications are host statements, not ROM receipts or permission grants. A transport failure does not establish rejection. The fixture's lost-acknowledgement case records accepted host work before throwing. Both components retain unknown barriers without changing their confirmed display.

An unknown barrier clears only on explicit host recovery token, authority scope or confirmed identity transitions. The host must resolve or quarantine the original accepted command before allowing replacement work. Tokens use `Object.is`, including NaN identity. Disabling controls blocks new dispatch; it does not prevent an owned pending callback from settling. Disabled changes do not release unknown barriers.

Selection identity includes exact ID and host selected state. Layout identity includes confirmed layout and captured configuration. Hosts must keep these stable while outcomes remain unresolved. A new configuration is a scope transition. Do not replace confirmed arrays with speculative proposals to clear the barrier.

The shared named command helper owns one pending/unknown barrier per component. It checks the original epoch after pending publication, before dispatch and after callbacks. Reset/dispose reentry cannot dispatch the obsolete callback. Scope changes fence late outcomes. The helper does not cancel, retry or settle backend work; host lifecycle and R4 recovery remain responsible for it.

## Layout bounds

`captureLayoutOptions` captures declared fields once. The existing `validateLayout` validates that snapshot. `proposeLayoutChange` keeps exact IDs, applies a single-cell command, and validates the complete detached proposal. Invalid input, unknown IDs, unsupported commands or out-of-bounds changes produce no proposal callback. The host chooses any reset or fallback.

Buttons disable impossible moves and sizes. Visibility is host-confirmed. Catalogs, collision/grid algorithms and persistence remain application-owned. The component does not select localStorage or infer Resource names.

## Test-first failures and corrections

Historical logs remain under `/var/tmp`:

- `rom-010-selection-layout-public-red.log`: actual build failed because both public exports were missing.
- `rom-010-selection-layout-unit-red.log`: actual focused unit failed because the proposal helper was missing.
- `rom-010-selection-layout-browser-first.log`: fourteen passed, two failed. At 390px, the fixture's raw JSON output overflowed. `mobile-diagnostic.log` identified that output as the only overflowing element. The fixture output now wraps; no component layout change was needed.
- `rom-010-selection-layout-disabled-red.log`: both engines reproduced a stranded pending card when disabled during a held callback. Dispatch and completion ownership were separated.
- `rom-010-selection-layout-ack-reentry-red.log`: three passed, two failed. Host work followed by an exception was incorrectly classified as rejected. Pending publisher reset/dispose also allowed obsolete callback dispatch.
- `rom-010-selection-layout-ack-browser-red.log`: both engines reproduced the incorrect lost-acknowledgement classification before correction.

The final helper consumes exceptions as unknown. Only explicit host rejection releases that barrier. It rechecks ownership after publication. Existing positive thenable coverage was updated to expect unknown when its getter throws.

## Final executed checks

Seven focused Node cases passed. They cover exact bounded proposals, corrupt/unknown input, unknown barriers, late rejection/disposal, throwing thenables, lost acknowledgement and publisher reentry. All 414 Studio unit tests passed. Svelte checking reported zero errors and warnings. Component build passed.

The final browser command passed 64 cases with no skip. Twenty executions cover the ten new selection/layout cases across both engines. Forty-four executions cover the existing public details, reference and history compositions. New cases cover mouse/keyboard multiselection, sibling actions, 1280/390 geometry, bounds, rejected/unknown outcomes, locale changes, disabled transitions, corruption, principal changes and lost acknowledgement.

Commands ran from `/root/ROM/studio`, with a 180-second command bound:

```sh
timeout 180s npm run build:components
timeout 180s npm run check
timeout 180s npm run test:unit
node --experimental-strip-types --test tests/unit/ui-selection-card.test.ts tests/unit/ui-layout-controls.test.ts
timeout 180s env ROM_WEBKIT_EXECUTABLE=/var/tmp/rom-studio-webkit-2359/pw_run.sh npx --no-install playwright test --config tests/components/playwright.config.ts selection-layout-composition.spec.ts history-composition.spec.ts reference-composition.spec.ts details-composition.spec.ts --workers=1
```

The browser configuration uses Chromium 138 and locked WebKit 2359. Final logs use the `rom-010-selection-layout-` prefix: `focused-green.log`, `all-units-final.log`, `check-final.log`, `build-final.log` and `browser-final.log`.

Owned source hashes matched before and after the final browser run. `/var/tmp/rom-010-selection-layout-evidence/` contains the manifests, preserved source archive and result metadata. Product hashes are:

| File | SHA-256 |
| --- | --- |
| SelectionCard.svelte | `3bf110424466b9eac38845d5417f629ff49c33be2dd29c52d51a0d54d1ad1e10` |
| LayoutControls.svelte | `067cb63d682265fe6cc545ff8c2492258c1ec11a00224d52403a2535be6d64c6` |
| selection-card.ts | `da0099723f5c0d5e4d3fa28e02e10a01994023e977fd710f86136c8cf1689e76` |
| layout-controls.ts | `d9822ad0c8f356d7a2de0cb8770610c812b6d0a80d940fa0c400d24bb13427b9` |

## Limits and handoff

This fixture is compiled from repository source. It is not an installed consumer or a real durable store. Explicit recovery controls stand in for host-verified resolution; they do not prove receipts, persistence or retry identity. Private data clearing and host continuations require current-authority checks after awaits.

Independent review, current-source installed public composition, actual persistence/reload and principal-bound mutation flows remain required. Original Astral Plane and held-out human/assistive-technology acceptance remain open. The coordinator owns full local verification and integration. No native build, dependency, commit or deployment was made in this task.
